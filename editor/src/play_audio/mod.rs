//! What a scene sounds like while it plays in the editor.
//!
//! Scripts ask for sound through `Audio`, and authored sources start
//! themselves, exactly as in a build; until now the editor dropped all of it,
//! so a play-test was silent. This performs the same requests through the
//! same mixer a shipped game uses, so a bus a settings slider turns down is
//! turned down here too.
//!
//! On top of the game's own mix the editor keeps a monitor: a trim, a mute
//! and a solo per bus. They change what the author hears and nothing the game
//! can observe — `Audio.volume` still answers what the game set — so muting
//! the music to listen for a hit sound cannot change how the game behaves.
//!
//! The device is opened on the first sound, not at startup, for the reason
//! `Audition` gives. Where there is no device, playback carries on silently,
//! so what is playing is still listed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sindri_core::{ComponentSchemaRegistry, World};
use sindri_decay::AudioCommand;
use sindri_platform::{
    AudioBackend, AudioClip, AudioError, AudioMixer, EFFECTS_BUS, MASTER_BUS, MUSIC_BUS,
    PlaybackSettings, PlayingVoice, SilentAudioBackend,
};
use sindri_scene::AudioSourceComponent;

/// Opens the device the first time a sound is wanted.
pub type Opener = fn() -> Result<Box<dyn AudioBackend>, AudioError>;

/// The native device, as a shipped desktop game opens it.
#[must_use]
pub fn native() -> Opener {
    || {
        sindri_platform::NativeAudioBackend::new()
            .map(|backend| Box::new(backend) as Box<dyn AudioBackend>)
    }
}

/// How long a one-shot is listed as playing when there is no device to say
/// when it ends.
pub const SILENT_ONE_SHOT: std::time::Duration = std::time::Duration::from_secs(1);

/// One bus as the Audio panel shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct BusRow {
    pub name: String,
    /// What the game set it to.
    pub game: f32,
    /// The editor's trim on top of that, from 0 to 1.
    pub trim: f32,
    pub muted: bool,
    pub solo: bool,
    /// Whether it is heard at all, given every mute and solo.
    pub audible: bool,
}

/// The editor's audio for Play, and its monitor.
pub struct PlayAudio {
    open: Opener,
    backend: Option<Box<dyn AudioBackend>>,
    /// Why the device could not be opened, said once.
    unavailable: Option<String>,
    /// When each one-shot began, while playing silently: a backend with no
    /// device never finishes one, so it is let go after [`SILENT_ONE_SHOT`]
    /// rather than listed for ever.
    silent_since: BTreeMap<sindri_platform::AudioVoiceId, std::time::Instant>,
    mixer: AudioMixer,
    /// What the game set each bus to; the mixer holds that times the monitor.
    game: BTreeMap<String, f32>,
    trim: BTreeMap<String, f32>,
    muted: BTreeSet<String>,
    solo: BTreeSet<String>,
    /// Every bus named this run, so a bus stays listed after its sound ends.
    named: BTreeSet<String>,
    registered: BTreeSet<String>,
    root: Option<PathBuf>,
    paused: bool,
}

impl PlayAudio {
    #[must_use]
    pub fn new(open: Opener) -> Self {
        Self {
            open,
            backend: None,
            unavailable: None,
            silent_since: BTreeMap::new(),
            mixer: AudioMixer::new(),
            game: BTreeMap::new(),
            trim: BTreeMap::new(),
            muted: BTreeSet::new(),
            solo: BTreeSet::new(),
            named: [MASTER_BUS, MUSIC_BUS, EFFECTS_BUS]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            registered: BTreeSet::new(),
            root: None,
            paused: false,
        }
    }

    /// Starts a run of `scene`: authored sources marked autoplay begin.
    /// Answers what could not be played, for the console.
    pub fn start(
        &mut self,
        scene: Option<&Path>,
        world: &World,
        components: &ComponentSchemaRegistry,
    ) -> Vec<String> {
        self.stop();
        self.root = scene.and_then(Path::parent).map(Path::to_path_buf);
        let Ok(sources) = components.query::<AudioSourceComponent>(world) else {
            return Vec::new();
        };
        let mut problems = Vec::new();
        for (_, source) in sources {
            self.named.insert(source.bus().to_owned());
            if !source.autoplay {
                continue;
            }
            let settings = if source.looping {
                PlaybackSettings::looping(source.normalized_volume())
            } else {
                PlaybackSettings::once(source.normalized_volume())
            };
            if let Err(problem) = self.play(&source.clip, settings, source.bus()) {
                problems.push(problem);
            }
        }
        problems
    }

    /// Performs what the scripts asked for this step.
    pub fn perform(&mut self, commands: Vec<AudioCommand>) -> Vec<String> {
        let mut problems = Vec::new();
        for command in commands {
            let played = match command {
                AudioCommand::Play { clip, volume, bus } => {
                    self.play(&clip, PlaybackSettings::once(volume), &bus)
                }
                AudioCommand::Loop { clip, volume, bus } => {
                    self.play(&clip, PlaybackSettings::looping(volume), &bus)
                }
                AudioCommand::SetVolume { bus, volume } => {
                    self.game.insert(bus.clone(), volume.clamp(0.0, 1.0));
                    self.named.insert(bus);
                    self.apply();
                    Ok(())
                }
                AudioCommand::StopAll => {
                    if let Some(backend) = self.backend.as_deref_mut() {
                        self.mixer.stop_all(backend);
                    }
                    Ok(())
                }
                AudioCommand::PauseAll => {
                    if let Some(backend) = self.backend.as_deref_mut() {
                        backend.pause_all();
                    }
                    Ok(())
                }
                AudioCommand::ResumeAll => {
                    if let Some(backend) = self.backend.as_deref_mut()
                        && !self.paused
                    {
                        backend.resume_all();
                    }
                    Ok(())
                }
            };
            if let Err(problem) = played {
                problems.push(problem);
            }
        }
        problems
    }

    /// Holds or releases every voice, as the transport's pause does.
    pub fn set_paused(&mut self, paused: bool) {
        if self.paused == paused {
            return;
        }
        self.paused = paused;
        if let Some(backend) = self.backend.as_deref_mut() {
            if paused {
                backend.pause_all();
            } else {
                backend.resume_all();
            }
        }
    }

    /// Ends the run: everything stops and the game's bus volumes are
    /// forgotten, so the next run starts from its own mix. The monitor stays.
    pub fn stop(&mut self) {
        if let Some(backend) = self.backend.as_deref_mut() {
            self.mixer.stop_all(backend);
            backend.resume_all();
        }
        self.paused = false;
        self.game.clear();
        self.named
            .retain(|bus| [MASTER_BUS, MUSIC_BUS, EFFECTS_BUS].contains(&bus.as_str()));
        self.apply();
    }

    /// What is playing now, oldest first.
    pub fn playing(&mut self) -> Vec<PlayingVoice> {
        let Some(backend) = self.backend.as_deref_mut() else {
            return Vec::new();
        };
        if self.unavailable.is_some() {
            let now = std::time::Instant::now();
            for voice in self.mixer.playing(backend) {
                if voice.looping {
                    continue;
                }
                let began = *self.silent_since.entry(voice.voice).or_insert(now);
                if now.duration_since(began) >= SILENT_ONE_SHOT {
                    backend.stop(voice.voice);
                    self.silent_since.remove(&voice.voice);
                }
            }
        }
        self.mixer.playing(backend)
    }

    /// Stops one voice, from the panel.
    pub fn stop_voice(&mut self, voice: sindri_platform::AudioVoiceId) {
        if let Some(backend) = self.backend.as_deref_mut() {
            backend.stop(voice);
        }
    }

    /// Every bus named this run, master first.
    pub fn buses(&mut self) -> Vec<BusRow> {
        for voice in self.playing() {
            self.named.insert(voice.bus);
        }
        let mut names: Vec<&String> = self.named.iter().collect();
        names.sort_by_key(|name| (name.as_str() != MASTER_BUS, name.as_str()));
        names
            .into_iter()
            .map(|name| BusRow {
                name: name.clone(),
                game: self.game_volume(name),
                trim: self.trim_of(name),
                muted: self.muted.contains(name),
                solo: self.solo.contains(name),
                audible: self.audible(name),
            })
            .collect()
    }

    pub fn set_trim(&mut self, bus: &str, trim: f32) {
        self.trim.insert(bus.to_owned(), trim.clamp(0.0, 1.0));
        self.apply();
    }

    pub fn set_muted(&mut self, bus: &str, muted: bool) {
        if muted {
            self.muted.insert(bus.to_owned());
        } else {
            self.muted.remove(bus);
        }
        self.apply();
    }

    pub fn set_solo(&mut self, bus: &str, solo: bool) {
        if solo {
            self.solo.insert(bus.to_owned());
        } else {
            self.solo.remove(bus);
        }
        self.apply();
    }

    /// Why nothing can be heard, when the device would not open: the
    /// backend's own words.
    pub fn unavailable(&self) -> Option<&str> {
        self.unavailable.as_deref()
    }

    fn game_volume(&self, bus: &str) -> f32 {
        self.game.get(bus).copied().unwrap_or(1.0)
    }

    fn trim_of(&self, bus: &str) -> f32 {
        self.trim.get(bus).copied().unwrap_or(1.0)
    }

    /// Whether a bus is heard: the master only by its own mute, any other by
    /// its mute and, while anything is soloed, by being soloed itself.
    fn audible(&self, bus: &str) -> bool {
        if self.muted.contains(bus) {
            return false;
        }
        bus == MASTER_BUS || self.solo.is_empty() || self.solo.contains(bus)
    }

    /// What the mixer is given for a bus: the game's volume times the monitor.
    fn heard(&self, bus: &str) -> f32 {
        if self.audible(bus) {
            self.game_volume(bus) * self.trim_of(bus)
        } else {
            0.0
        }
    }

    /// Hands the mixer every named bus at what is heard of it.
    fn apply(&mut self) {
        let buses: Vec<(String, f32)> = self
            .named
            .iter()
            .chain(self.game.keys())
            .map(|bus| (bus.clone(), self.heard(bus)))
            .collect();
        let Some(backend) = self.backend.as_deref_mut() else {
            return;
        };
        for (bus, heard) in buses {
            self.mixer.set_bus_volume(backend, &bus, heard);
        }
    }

    fn backend(&mut self) -> &mut dyn AudioBackend {
        if self.backend.is_none() {
            let opened = (self.open)().unwrap_or_else(|error| {
                self.unavailable = Some(error.to_string());
                Box::new(SilentAudioBackend::default())
            });
            self.backend = Some(opened);
            self.apply();
        }
        self.backend
            .as_deref_mut()
            .unwrap_or_else(|| unreachable!("opened above"))
    }

    fn play(&mut self, clip: &str, settings: PlaybackSettings, bus: &str) -> Result<(), String> {
        let newly_named = self.named.insert(bus.to_owned());
        let fresh = !self.registered.contains(clip);
        let file = self.root.as_ref().map(|root| root.join(clip));
        self.backend();
        if newly_named {
            self.apply();
        }
        let backend = self
            .backend
            .as_deref_mut()
            .unwrap_or_else(|| unreachable!("opened above"));
        if fresh {
            let Some(file) = file else {
                return Err(format!(
                    "Audio: {clip} cannot be found: the scene is not saved"
                ));
            };
            let bytes = std::fs::read(&file).map_err(|error| format!("Audio: {clip}: {error}"))?;
            let mime = crate::audition::mime_of(&file)
                .ok_or_else(|| format!("Audio: {clip} is not a format the engine plays"))?;
            backend
                .register(AudioClip::new(clip, bytes, mime))
                .map_err(|error| format!("Audio: {error}"))?;
            self.registered.insert(clip.to_owned());
        }
        self.mixer
            .play(backend, clip, settings, bus)
            .map(|_| ())
            .map_err(|error| format!("Audio: {error}"))
    }
}

#[cfg(test)]
mod tests;
