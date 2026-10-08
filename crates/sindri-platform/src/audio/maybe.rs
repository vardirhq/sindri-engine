//! Sound through a device when the machine has one, and silence when not.

use super::{
    AudioBackend, AudioClip, AudioError, AudioVoiceId, PlaybackSettings, SilentAudioBackend,
};

/// A device's backend, or the silent one when no device would open.
///
/// A game is still a game on a machine without a sound card — a server, a
/// CI runner, a laptop with its output unplugged — so a host that would
/// otherwise refuse to start plays on in silence instead, and says why once.
#[derive(Debug)]
pub enum MaybeAudio<A: AudioBackend> {
    Device(A),
    Silent(SilentAudioBackend),
}

impl<A: AudioBackend> MaybeAudio<A> {
    /// The device `open` gives, or silence, with the reason logged.
    pub fn open_or_silent(open: impl FnOnce() -> Result<A, AudioError>) -> Self {
        match open() {
            Ok(device) => Self::Device(device),
            Err(error) => {
                log::warn!("no audio output, playing silently: {error}");
                Self::Silent(SilentAudioBackend::default())
            }
        }
    }

    fn backend(&mut self) -> &mut dyn AudioBackend {
        match self {
            Self::Device(device) => device,
            Self::Silent(silent) => silent,
        }
    }
}

impl<A: AudioBackend> AudioBackend for MaybeAudio<A> {
    fn register(&mut self, clip: AudioClip) -> Result<(), AudioError> {
        self.backend().register(clip)
    }

    fn play(&mut self, clip: &str, settings: PlaybackSettings) -> Result<AudioVoiceId, AudioError> {
        self.backend().play(clip, settings)
    }

    fn stop(&mut self, voice: AudioVoiceId) {
        self.backend().stop(voice);
    }

    fn set_volume(&mut self, voice: AudioVoiceId, volume: f32) {
        self.backend().set_volume(voice, volume);
    }

    fn is_active(&self, voice: AudioVoiceId) -> bool {
        match self {
            Self::Device(device) => device.is_active(voice),
            Self::Silent(silent) => silent.is_active(voice),
        }
    }

    fn pause_all(&mut self) {
        self.backend().pause_all();
    }

    fn resume_all(&mut self) {
        self.backend().resume_all();
    }

    fn stop_all(&mut self) {
        self.backend().stop_all();
    }

    fn unlock(&mut self) -> Result<(), AudioError> {
        self.backend().unlock()
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioError, MaybeAudio, SilentAudioBackend};
    use crate::audio::{AudioBackend, AudioClip, PlaybackSettings};

    #[test]
    fn a_device_that_will_not_open_plays_silently() {
        let mut audio = MaybeAudio::<SilentAudioBackend>::open_or_silent(|| {
            Err(AudioError::Output("no card".to_owned()))
        });
        assert!(matches!(audio, MaybeAudio::Silent(_)));
        audio
            .register(AudioClip::new("ping.ogg", Vec::new(), "audio/ogg"))
            .unwrap();
        let voice = audio.play("ping.ogg", PlaybackSettings::once(1.0)).unwrap();
        assert!(audio.is_active(voice));
    }
}
