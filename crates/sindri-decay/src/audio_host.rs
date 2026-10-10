//! Audio requests from Decay without giving the language a device.
//!
//! Scripts enqueue intent. The game host drains that intent after a script
//! step and performs it through `sindri-platform::AudioBackend`. Keeping the
//! command between the language and the platform is what preserves Decay's
//! no-I/O boundary and still lets a silent backend assert sound in tests.

use decay_ir::Path;
use decay_runtime::{Host, RuntimeError, Value};

use crate::{Blackboard, ScriptContext, host::Spawning};

pub(crate) const AUDIO: &str = "Audio";

/// How many unperformed requests a runner keeps.
///
/// A caller with no audio device never drains, so without a bound a script
/// calling `Audio.play` every frame grows the queue for as long as the runner
/// lives — which is what the editor's play mode does. Audio intent goes stale
/// in milliseconds, so the newest requests are the ones worth keeping and the
/// oldest are dropped to make room.
const PENDING_LIMIT: usize = 256;

fn enqueue(queue: &mut Vec<AudioCommand>, command: AudioCommand) {
    if queue.len() >= PENDING_LIMIT {
        queue.remove(0);
    }
    queue.push(command);
}

/// The bus `Audio.play` routes to: `sindri_platform::EFFECTS_BUS`.
const EFFECTS: &str = "effects";
/// The bus `Audio.loop` routes to: `sindri_platform::MUSIC_BUS`.
const MUSIC: &str = "music";

#[derive(Clone, Debug, PartialEq)]
pub enum AudioCommand {
    Play {
        clip: String,
        volume: f32,
        bus: String,
    },
    Loop {
        clip: String,
        volume: f32,
        bus: String,
    },
    /// A bus's volume, for the host's mixer to apply to what it is playing.
    SetVolume {
        bus: String,
        volume: f32,
    },
    StopAll,
    PauseAll,
    ResumeAll,
}

/// What scripts asked to be played, and the bus volumes they have set.
///
/// The volumes are kept here as well as sent, so `Audio.volume` answers on
/// the step it was set and in a host with no device at all; the host's mixer
/// holds the same numbers for the voices it is playing.
#[derive(Clone, Debug, Default)]
pub struct AudioQueue {
    commands: Vec<AudioCommand>,
    buses: std::collections::BTreeMap<String, f32>,
}

impl AudioQueue {
    /// Takes the requests made since the last call, oldest first.
    pub fn take(&mut self) -> Vec<AudioCommand> {
        std::mem::take(&mut self.commands)
    }

    /// Drops unperformed requests; bus volumes, a player's settings, stay.
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// A bus's volume as scripts last set it: full for one never set.
    #[must_use]
    pub fn volume(&self, bus: &str) -> f32 {
        self.buses.get(bus).copied().unwrap_or(1.0)
    }

    #[cfg(test)]
    pub(crate) fn commands(&self) -> &[AudioCommand] {
        &self.commands
    }
}

/// The ordinary world host plus the `Audio.*` namespace.
///
/// The queue is borrowed rather than global. It was a thread-local, which meant
/// every caller of `Scripts::advance` shared one queue and only the game ever
/// emptied it: the editor pushed a command per `Audio.play` per frame of play
/// mode and nothing ever drained them. Whoever runs the scripts owns the
/// requests they produce.
pub struct WorldHost<'a> {
    inner: crate::host::WorldHost<'a>,
    audio: &'a mut AudioQueue,
}

/// Everything a script can reach beyond the world and the frame.
///
/// Bundled for the same reason `ScriptFrame` is one level up: the list had
/// reached eight and every capability the scripting surface grows adds another.
/// A caller with no physics and no screen UI leaves two fields out rather than
/// passing two `None`s in the right positions.
pub struct HostServices<'a> {
    pub spawning: Spawning<'a>,
    /// Reusable authored data a typed `Profile` field may name.
    pub profiles: &'a crate::ProfileSources,
    /// The physics a script may read and drive, when the host runs any.
    pub physics: Option<crate::Physics2d<'a>>,
    /// Where the screen elements are and what the pointer is doing to them.
    pub screen_ui: Option<&'a sindri_scene::ScreenUi>,
    /// What the person just did, as an intention rather than as a button.
    pub gestures: Option<&'a sindri_core::Gestures>,
    /// How far this frame's drag asks the camera to move.
    pub camera_pan: Option<[f32; 3]>,
    /// Which block the pointer is on, when the host picked one this frame.
    pub aim: Option<sindri_scene::voxel::VolumeAim>,
    /// The run's random stream, when the host is running one.
    pub random: Option<&'a mut sindri_core::Rng>,
    /// What the game remembers, when the host is keeping a save.
    pub saves: Option<&'a mut sindri_core::SaveStore>,
    /// The fleck pool, when the host is running one.
    pub effects: Option<&'a mut sindri_scene::Effects2d>,
    /// Where each animated sprite has got to, when the host advances any.
    pub animations: Option<&'a mut sindri_scene::SpriteAnimations>,
    /// The tile sets a stacked volume's cells name, when the host binds any.
    pub tile_sets: Option<&'a sindri_scene::TileSetBindings>,
    /// What the script asked to be played, in order, and the bus volumes.
    pub audio: &'a mut AudioQueue,
    /// Which scene is being played, and which one a script asked for.
    ///
    /// `None` for a host that plays exactly one scene, and then `Scene.go`
    /// says so rather than accepting a request that goes nowhere — a game
    /// whose doors silently never open should be heard about on the first
    /// frame, not mistaken for a door nobody walked into.
    pub scenes: Option<&'a mut crate::SceneChannel>,
}

impl<'a> WorldHost<'a> {
    /// Lets this host reach the other scripts in the pass.
    #[must_use]
    pub(crate) fn with_peers(mut self, peers: crate::host::Peers<'a>) -> Self {
        self.inner = self.inner.with_peers(peers);
        self
    }

    pub(crate) fn with_tweens(mut self, tweens: &'a mut crate::tweens::Tweens) -> Self {
        self.inner.tweens = Some(tweens);
        self
    }

    pub(crate) fn with_actions(mut self, actions: &'a crate::actions::InputActions) -> Self {
        self.inner.actions = Some(actions);
        self
    }

    pub(crate) fn with_sequences(
        mut self,
        sequences: Option<&'a mut sindri_scene::Sequences>,
    ) -> Self {
        self.inner.sequences = sequences;
        self
    }

    /// Supplies independent 3D controls and the completed event snapshot.
    #[must_use]
    pub fn with_physics3d(mut self, physics: Option<crate::Physics3d<'a>>) -> Self {
        self.inner.physics3d = physics;
        self
    }

    /// Supplies the scene's controller queue and cached results.
    #[must_use]
    pub fn with_characters(mut self, characters: Option<crate::Characters2d<'a>>) -> Self {
        self.inner.characters = characters;
        self
    }

    pub fn new(
        world: &'a mut sindri_core::World,
        entity: sindri_core::EntityId,
        context: ScriptContext<'a>,
        blackboard: &'a mut Blackboard,
        services: HostServices<'a>,
    ) -> Self {
        let HostServices {
            spawning,
            profiles,
            physics,
            screen_ui,
            aim,
            gestures,
            camera_pan,
            random,
            saves,
            effects,
            animations,
            audio,
            scenes,
            tile_sets,
        } = services;
        Self {
            inner: crate::host::WorldHost::new(
                world,
                entity,
                context,
                blackboard,
                crate::host::WorldServices {
                    spawning,
                    profiles,
                    saves,
                    effects,
                    physics,
                    screen_ui,
                    aim,
                    gestures,
                    camera_pan,
                    random,
                    animations,
                    scenes,
                    tile_sets,
                },
            ),
            audio,
        }
    }

    /// Cursor capture intent from this invocation, for the windowed host.
    pub fn take_pointer_lock_request(&mut self) -> Option<bool> {
        self.inner.take_pointer_lock_request()
    }

    pub fn take_printed(&mut self) -> Vec<String> {
        self.inner.take_printed()
    }
}

impl Host for WorldHost<'_> {
    fn load(&mut self, subject: Option<u64>, path: &Path) -> Result<Option<Value>, RuntimeError> {
        self.inner.load(subject, path)
    }

    fn store(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        value: Value,
    ) -> Result<bool, RuntimeError> {
        self.inner.store(subject, path, value)
    }

    fn call(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        args: &[Value],
    ) -> Result<Option<Value>, RuntimeError> {
        if subject.is_none() {
            let parts: Vec<&str> = path.0.iter().map(String::as_str).collect();
            if let [namespace, name] = parts.as_slice()
                && *namespace == AUDIO
            {
                return audio_call(self.audio, name, path, args).map(Some);
            }
        }
        self.inner.call(subject, path, args)
    }
}

fn normalized_volume(path: &Path, value: Option<&Value>) -> Result<f32, RuntimeError> {
    let Some(Value::Number(volume)) = value else {
        return Err(RuntimeError::Host(format!(
            "{} takes a volume number between 0 and 1, and the script gave {value:?}",
            path.dotted()
        )));
    };
    if !volume.is_finite() || !(0.0..=1.0).contains(volume) {
        return Err(RuntimeError::Host(format!(
            "{} takes a finite volume number between 0 and 1, and the script gave {volume}",
            path.dotted()
        )));
    }

    // Decay numbers are f64 while the platform audio boundary intentionally
    // uses f32, matching Rodio and the rest of the real-time audio path. The
    // normalized range above guarantees this conversion cannot overflow.
    #[allow(clippy::cast_possible_truncation)]
    let volume = *volume as f32;
    Ok(volume)
}

fn text_arg(path: &Path, args: &[Value], at: usize, what: &str) -> Result<String, RuntimeError> {
    match args.get(at) {
        Some(Value::String(text)) => Ok(text.clone()),
        _ => Err(RuntimeError::Host(format!(
            "{} takes {what} as text",
            path.dotted()
        ))),
    }
}

fn audio_call(
    queue: &mut AudioQueue,
    name: &str,
    path: &Path,
    args: &[Value],
) -> Result<Value, RuntimeError> {
    let command = match name {
        "play" | "loop" | "play_on" | "loop_on" => {
            // `play_on` and `loop_on` name the bus first, as `set_volume` does.
            let (bus, at) = match name {
                "play" => (EFFECTS.to_owned(), 0),
                "loop" => (MUSIC.to_owned(), 0),
                _ => (text_arg(path, args, 0, "a bus name")?, 1),
            };
            let clip = text_arg(path, args, at, "an audio asset id")?;
            let volume = normalized_volume(path, args.get(at + 1))?;
            if name.starts_with("play") {
                AudioCommand::Play { clip, volume, bus }
            } else {
                AudioCommand::Loop { clip, volume, bus }
            }
        }
        "set_volume" => {
            let bus = text_arg(path, args, 0, "a bus name")?;
            let volume = normalized_volume(path, args.get(1))?;
            queue.buses.insert(bus.clone(), volume);
            AudioCommand::SetVolume { bus, volume }
        }
        "volume" => {
            let bus = text_arg(path, args, 0, "a bus name")?;
            return Ok(Value::Number(f64::from(queue.volume(&bus))));
        }
        "stop_all" => AudioCommand::StopAll,
        "pause_all" => AudioCommand::PauseAll,
        "resume_all" => AudioCommand::ResumeAll,
        _ => return Ok(Value::Null),
    };
    enqueue(&mut queue.commands, command);
    Ok(Value::Unit)
}

#[cfg(test)]
mod tests {
    use decay_ir::Path;
    use decay_runtime::{Host, RuntimeError, Value};
    use sindri_core::{EntityData, World};
    use sindri_platform::InputState;

    use super::{AudioCommand, AudioQueue, WorldHost};
    use crate::{Blackboard, ScriptContext, host::Spawning};

    /// A spawning context for a test that is not about spawning.
    fn nothing_to_spawn() -> (
        crate::PrefabSources,
        std::collections::BTreeSet<sindri_core::EntityId>,
        Vec<sindri_core::EntityId>,
    ) {
        (
            crate::PrefabSources::new(),
            std::collections::BTreeSet::new(),
            Vec::new(),
        )
    }

    fn spawning<'a>(
        prefabs: &'a crate::PrefabSources,
        started: &'a std::collections::BTreeSet<sindri_core::EntityId>,
        spawned: &'a mut Vec<sindri_core::EntityId>,
    ) -> Spawning<'a> {
        Spawning {
            prefabs,
            started,
            spawned,
        }
    }

    #[test]
    fn audio_call_emits_intent_without_a_device() {
        let mut world = World::default();
        let entity = world.spawn(EntityData::default());
        let input = InputState::default();
        let mut board = Blackboard::new();
        let mut queue = AudioQueue::default();
        let (prefabs, started, mut spawned) = nothing_to_spawn();
        let mut host = WorldHost::new(
            &mut world,
            entity,
            ScriptContext {
                input: &input,
                delta_seconds: 0.0,
                elapsed_seconds: 0.0,
            },
            &mut board,
            crate::HostServices {
                profiles: crate::ProfileSources::none(),
                spawning: spawning(&prefabs, &started, &mut spawned),
                physics: None,
                screen_ui: None,
                aim: None,
                gestures: None,
                camera_pan: None,
                random: None,
                saves: None,
                effects: None,
                animations: None,
                audio: &mut queue,
                scenes: None,
                tile_sets: None,
            },
        );
        host.call(
            None,
            &Path(vec!["Audio".to_owned(), "play".to_owned()]),
            &[
                Value::String("audio/pickup.wav".to_owned()),
                Value::Number(0.8),
            ],
        )
        .expect("audio call");
        host.call(
            None,
            &Path(vec!["Audio".to_owned(), "set_volume".to_owned()]),
            &[Value::String("music".to_owned()), Value::Number(0.25)],
        )
        .expect("set a bus");
        let heard = host
            .call(
                None,
                &Path(vec!["Audio".to_owned(), "volume".to_owned()]),
                &[Value::String("music".to_owned())],
            )
            .expect("read a bus");
        assert_eq!(heard, Some(Value::Number(0.25)));
        drop(host);
        assert_eq!(
            queue.take(),
            [
                AudioCommand::Play {
                    clip: "audio/pickup.wav".to_owned(),
                    volume: 0.8,
                    bus: "effects".to_owned(),
                },
                AudioCommand::SetVolume {
                    bus: "music".to_owned(),
                    volume: 0.25,
                },
            ]
        );
    }

    /// A runner nobody drains does not grow without end.
    #[test]
    fn an_undrained_queue_keeps_only_the_newest_requests() {
        let mut queue = Vec::new();
        for index in 0..(super::PENDING_LIMIT + 10) {
            super::enqueue(
                &mut queue,
                AudioCommand::Play {
                    clip: format!("audio/{index}.wav"),
                    volume: 1.0,
                    bus: "effects".to_owned(),
                },
            );
        }
        assert_eq!(queue.len(), super::PENDING_LIMIT);
        assert_eq!(
            queue.first(),
            Some(&AudioCommand::Play {
                clip: "audio/10.wav".to_owned(),
                volume: 1.0,
                bus: "effects".to_owned(),
            }),
            "the oldest requests are the ones dropped"
        );
    }

    #[test]
    fn audio_call_rejects_volume_outside_normalized_range() {
        let mut world = World::default();
        let entity = world.spawn(EntityData::default());
        let input = InputState::default();
        let mut board = Blackboard::new();
        let mut queue = AudioQueue::default();
        let (prefabs, started, mut spawned) = nothing_to_spawn();
        let mut host = WorldHost::new(
            &mut world,
            entity,
            ScriptContext {
                input: &input,
                delta_seconds: 0.0,
                elapsed_seconds: 0.0,
            },
            &mut board,
            crate::HostServices {
                profiles: crate::ProfileSources::none(),
                spawning: spawning(&prefabs, &started, &mut spawned),
                physics: None,
                screen_ui: None,
                aim: None,
                gestures: None,
                camera_pan: None,
                random: None,
                saves: None,
                effects: None,
                animations: None,
                audio: &mut queue,
                scenes: None,
                tile_sets: None,
            },
        );
        let error = host
            .call(
                None,
                &Path(vec!["Audio".to_owned(), "play".to_owned()]),
                &[
                    Value::String("audio/pickup.wav".to_owned()),
                    Value::Number(1.5),
                ],
            )
            .expect_err("volume outside 0..=1 must fail");
        assert!(matches!(
            error,
            RuntimeError::Host(message) if message.contains("between 0 and 1")
        ));
        assert!(queue.commands().is_empty());
    }
}
