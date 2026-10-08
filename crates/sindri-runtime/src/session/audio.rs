//! What a run sounds like: the scene's autoplaying sources, and what the
//! scripts asked to hear.

use sindri_core::World;
use sindri_decay::AudioCommand;
use sindri_platform::{AudioBackend, AudioError, PlaybackSettings};
use sindri_scene::AudioSourceComponent;

use super::Session;
use crate::RuntimeError;

impl Session {
    pub(super) fn start_autoplay(
        &mut self,
        world: &World,
        audio: &mut dyn AudioBackend,
    ) -> Result<(), RuntimeError> {
        if self.autoplay_started {
            return Ok(());
        }
        for (_, source) in self.components.query::<AudioSourceComponent>(world)? {
            if !source.autoplay {
                continue;
            }
            let settings = if source.looping {
                PlaybackSettings::looping(source.normalized_volume())
            } else {
                PlaybackSettings::once(source.normalized_volume())
            };
            match self.mixer.play(audio, &source.clip, settings, source.bus()) {
                Ok(_) => {}
                Err(AudioError::Locked) => return Ok(()),
                Err(error) => return Err(error.into()),
            }
        }
        self.autoplay_started = true;
        Ok(())
    }

    pub(super) fn flush_audio(&mut self, audio: &mut dyn AudioBackend) -> Result<(), RuntimeError> {
        fn survivable(error: &AudioError) -> bool {
            matches!(error, AudioError::MissingClip(_) | AudioError::Locked)
        }

        for command in std::mem::take(&mut self.pending_audio) {
            let played = match command {
                AudioCommand::Play { clip, volume, bus } => {
                    self.mixer
                        .play(audio, &clip, PlaybackSettings::once(volume), &bus)
                }
                AudioCommand::Loop { clip, volume, bus } => {
                    self.mixer
                        .play(audio, &clip, PlaybackSettings::looping(volume), &bus)
                }
                AudioCommand::SetVolume { bus, volume } => {
                    self.mixer.set_bus_volume(audio, &bus, volume);
                    continue;
                }
                AudioCommand::StopAll => {
                    self.mixer.stop_all(audio);
                    continue;
                }
                AudioCommand::PauseAll => {
                    audio.pause_all();
                    continue;
                }
                AudioCommand::ResumeAll => {
                    audio.resume_all();
                    continue;
                }
            };
            match played {
                Ok(_) => {}
                Err(error) if survivable(&error) => log::warn!("{error}"),
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    /// What the scripts and sequences asked to hear since this was last
    /// asked, for a host that plays sound its own way rather than through a
    /// backend this session drives: the editor, with its mixer monitor.
    pub fn take_audio_commands(&mut self) -> Vec<AudioCommand> {
        std::mem::take(&mut self.pending_audio)
    }
}
