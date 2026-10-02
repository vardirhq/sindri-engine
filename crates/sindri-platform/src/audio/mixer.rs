//! Buses: named volumes a sound is routed through, under one master.
//!
//! A settings screen's music slider moves the `music` bus, and every voice
//! routed there, playing now or started later, follows it. The mixer sits in
//! front of a backend rather than inside one, so native, browser and silent
//! playback share one set of rules and a backend only has to change the
//! volume of a voice it is already playing.

use std::collections::BTreeMap;

use super::{AudioBackend, AudioError, AudioVoiceId, PlaybackSettings};

/// The bus every other bus is under: the game's overall volume.
pub const MASTER_BUS: &str = "master";
/// Where `Audio.loop` and a looping authored source go unless told otherwise.
pub const MUSIC_BUS: &str = "music";
/// Where `Audio.play` and a one-shot authored source go unless told otherwise.
pub const EFFECTS_BUS: &str = "effects";

/// Bus volumes, and the voices playing through each.
///
/// A bus exists as soon as it is named, at full volume; nothing has to
/// declare it first. A voice is heard at its own volume times its bus's
/// times the master's.
#[derive(Clone, Debug, Default)]
pub struct AudioMixer {
    buses: BTreeMap<String, f32>,
    voices: BTreeMap<AudioVoiceId, Routed>,
}

#[derive(Clone, Debug)]
struct Routed {
    clip: String,
    bus: String,
    volume: f32,
    looping: bool,
}

/// A voice the mixer started that the backend is still playing.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayingVoice {
    pub voice: AudioVoiceId,
    pub clip: String,
    pub bus: String,
    /// Its own volume, before its bus and the master scale it.
    pub volume: f32,
    pub looping: bool,
}

impl AudioMixer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A bus's own volume, from 0 to 1: full for a bus never set.
    #[must_use]
    pub fn bus_volume(&self, bus: &str) -> f32 {
        self.buses.get(bus).copied().unwrap_or(1.0)
    }

    /// How loud something routed to `bus` at full volume is heard.
    #[must_use]
    pub fn gain(&self, bus: &str) -> f32 {
        let own = self.bus_volume(bus);
        if bus == MASTER_BUS {
            own
        } else {
            own * self.bus_volume(MASTER_BUS)
        }
    }

    /// Plays `clip` through `bus`, at its own volume scaled by the bus's.
    ///
    /// # Errors
    ///
    /// Whatever the backend refuses the clip with.
    pub fn play(
        &mut self,
        audio: &mut dyn AudioBackend,
        clip: &str,
        settings: PlaybackSettings,
        bus: &str,
    ) -> Result<AudioVoiceId, AudioError> {
        self.voices.retain(|voice, _| audio.is_active(*voice));
        let volume = settings.volume.clamp(0.0, 1.0);
        let voice = audio.play(
            clip,
            PlaybackSettings {
                volume: volume * self.gain(bus),
                ..settings
            },
        )?;
        self.voices.insert(
            voice,
            Routed {
                clip: clip.to_owned(),
                bus: bus.to_owned(),
                volume,
                looping: settings.mode == super::PlaybackMode::Loop,
            },
        );
        Ok(voice)
    }

    /// Sets a bus's volume and applies it at once to every voice it reaches:
    /// those routed to it, or every voice for the master.
    pub fn set_bus_volume(&mut self, audio: &mut dyn AudioBackend, bus: &str, volume: f32) {
        let volume = if volume.is_finite() {
            volume.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.buses.insert(bus.to_owned(), volume);
        self.voices.retain(|voice, _| audio.is_active(*voice));
        for (voice, routed) in &self.voices {
            if bus == MASTER_BUS || routed.bus == bus {
                let gain = if routed.bus == MASTER_BUS {
                    self.bus_volume(MASTER_BUS)
                } else {
                    self.bus_volume(&routed.bus) * self.bus_volume(MASTER_BUS)
                };
                audio.set_volume(*voice, routed.volume * gain);
            }
        }
    }

    /// What is playing now, oldest first, forgetting voices that have ended.
    pub fn playing(&mut self, audio: &dyn AudioBackend) -> Vec<PlayingVoice> {
        self.voices.retain(|voice, _| audio.is_active(*voice));
        self.voices
            .iter()
            .map(|(voice, routed)| PlayingVoice {
                voice: *voice,
                clip: routed.clip.clone(),
                bus: routed.bus.clone(),
                volume: routed.volume,
                looping: routed.looping,
            })
            .collect()
    }

    /// Every bus given a volume, in name order.
    pub fn buses(&self) -> impl Iterator<Item = (&str, f32)> {
        self.buses
            .iter()
            .map(|(bus, volume)| (bus.as_str(), *volume))
    }

    /// Stops every voice, as `AudioBackend::stop_all` does, and forgets them.
    /// Bus volumes stay: a slider set before a scene change still holds after.
    pub fn stop_all(&mut self, audio: &mut dyn AudioBackend) {
        audio.stop_all();
        self.voices.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioMixer, EFFECTS_BUS, MASTER_BUS, MUSIC_BUS};
    use crate::{AudioBackend, AudioClip, AudioEvent, PlaybackSettings, SilentAudioBackend};

    fn backend() -> SilentAudioBackend {
        let mut audio = SilentAudioBackend::default();
        for clip in ["music.ogg", "shot.wav"] {
            audio
                .register(AudioClip::new(clip, vec![0], "audio/wav"))
                .expect("register");
        }
        audio
    }

    fn played_volume(audio: &SilentAudioBackend, clip: &str) -> f32 {
        audio
            .events()
            .iter()
            .rev()
            .find_map(|event| match event {
                AudioEvent::Played {
                    clip: played,
                    settings,
                    ..
                } if played == clip => Some(settings.volume),
                _ => None,
            })
            .expect("played")
    }

    #[test]
    fn a_voice_starts_at_its_volume_times_its_bus_and_the_master() {
        let mut audio = backend();
        let mut mixer = AudioMixer::new();
        mixer.set_bus_volume(&mut audio, MASTER_BUS, 0.5);
        mixer.set_bus_volume(&mut audio, MUSIC_BUS, 0.5);
        mixer
            .play(
                &mut audio,
                "music.ogg",
                PlaybackSettings::looping(0.8),
                MUSIC_BUS,
            )
            .expect("play");
        assert!((played_volume(&audio, "music.ogg") - 0.2).abs() < 1.0e-6);
    }

    #[test]
    fn moving_a_bus_moves_only_the_voices_routed_through_it() {
        let mut audio = backend();
        let mut mixer = AudioMixer::new();
        let music = mixer
            .play(
                &mut audio,
                "music.ogg",
                PlaybackSettings::looping(0.5),
                MUSIC_BUS,
            )
            .expect("play");
        let shot = mixer
            .play(
                &mut audio,
                "shot.wav",
                PlaybackSettings::once(1.0),
                EFFECTS_BUS,
            )
            .expect("play");
        audio.take_events();
        mixer.set_bus_volume(&mut audio, MUSIC_BUS, 0.2);
        assert_eq!(
            audio.take_events(),
            [AudioEvent::VolumeSet {
                voice: music,
                volume: 0.1
            }]
        );
        mixer.set_bus_volume(&mut audio, MASTER_BUS, 0.5);
        let events = audio.take_events();
        assert!(events.contains(&AudioEvent::VolumeSet {
            voice: music,
            volume: 0.05
        }));
        assert!(events.contains(&AudioEvent::VolumeSet {
            voice: shot,
            volume: 0.5
        }));
    }

    #[test]
    fn a_stopped_voice_is_forgotten_and_bus_volumes_outlive_it() {
        let mut audio = backend();
        let mut mixer = AudioMixer::new();
        mixer
            .play(
                &mut audio,
                "music.ogg",
                PlaybackSettings::looping(1.0),
                MUSIC_BUS,
            )
            .expect("play");
        mixer.set_bus_volume(&mut audio, MUSIC_BUS, 0.25);
        mixer.stop_all(&mut audio);
        audio.take_events();
        mixer.set_bus_volume(&mut audio, MUSIC_BUS, 0.5);
        assert!(audio.take_events().is_empty());
        assert!((mixer.bus_volume(MUSIC_BUS) - 0.5).abs() < f32::EPSILON);
        assert!((mixer.bus_volume("unnamed") - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn what_is_playing_is_listed_with_its_bus_until_it_stops() {
        let mut audio = backend();
        let mut mixer = AudioMixer::new();
        let music = mixer
            .play(
                &mut audio,
                "music.ogg",
                PlaybackSettings::looping(0.5),
                MUSIC_BUS,
            )
            .expect("play");
        mixer
            .play(
                &mut audio,
                "shot.wav",
                PlaybackSettings::once(1.0),
                EFFECTS_BUS,
            )
            .expect("play");
        let playing = mixer.playing(&audio);
        assert_eq!(playing.len(), 2);
        assert_eq!(playing[0].voice, music);
        assert_eq!(playing[0].clip, "music.ogg");
        assert_eq!(playing[0].bus, MUSIC_BUS);
        assert!(playing[0].looping && !playing[1].looping);
        audio.stop(music);
        let left: Vec<String> = mixer.playing(&audio).into_iter().map(|p| p.clip).collect();
        assert_eq!(left, ["shot.wav"]);
        mixer.set_bus_volume(&mut audio, MUSIC_BUS, 0.25);
        let buses: Vec<(&str, f32)> = mixer.buses().collect();
        assert_eq!(buses, [(MUSIC_BUS, 0.25)]);
    }
}
