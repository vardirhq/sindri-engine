//! The browser backend, over the Web Audio API.
//!
//! One `AudioContext` plays every voice. Browsers keep it suspended until the
//! person interacts with the page, and iOS Safari only lets it start from
//! *inside* a gesture's own handler, not from a game frame that runs a moment
//! later. So the backend listens for taps, clicks and key presses itself and
//! resumes the context right there. A backend built on one `<audio>` element
//! per sound could never satisfy that: each new element needs a gesture of its
//! own, and a game's sounds start in its frame loop, which is how a phone
//! played the platformer in silence.

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{
    AudioBuffer, AudioBufferSourceNode, AudioContext, AudioContextState, AudioScheduledSourceNode,
    GainNode,
};

use super::{AudioBackend, AudioClip, AudioError, AudioVoiceId, PlaybackMode, PlaybackSettings};

/// A clip's decoded samples; `None` until the browser finishes decoding it.
type Decoded = Rc<RefCell<Option<AudioBuffer>>>;

/// The DOM events a browser counts as a person meaning to interact: the ones
/// that may start audio.
const GESTURES: [&str; 5] = ["pointerdown", "pointerup", "touchend", "keydown", "click"];

/// One playing source and the gain it plays through.
struct BrowserVoice {
    source: AudioBufferSourceNode,
    gain: GainNode,
    ended: Rc<Cell<bool>>,
    /// Owned so the handler lives as long as the voice it reports on.
    _on_ended: Closure<dyn FnMut()>,
}

impl BrowserVoice {
    fn halt(&self) {
        self.source
            .unchecked_ref::<AudioScheduledSourceNode>()
            .set_onended(None);
        let _ = self
            .source
            .unchecked_ref::<AudioScheduledSourceNode>()
            .stop();
        let _ = self.gain.disconnect();
        self.ended.set(true);
    }
}

pub struct BrowserAudioBackend {
    context: Option<AudioContext>,
    clips: BTreeMap<String, Decoded>,
    voices: BTreeMap<AudioVoiceId, BrowserVoice>,
    next_voice: u64,
    unlocked: bool,
    /// Resumes the context from inside a gesture; removed when dropped.
    unlock: Option<Closure<dyn FnMut()>>,
}

impl BrowserAudioBackend {
    #[must_use]
    pub fn new() -> Self {
        let context = AudioContext::new().ok();
        let unlock = context.as_ref().map(|context| {
            let context = context.clone();
            let unlock = Closure::<dyn FnMut()>::new(move || {
                if context.state() != AudioContextState::Running {
                    let _ = context.resume();
                    // Older iOS also wants a sound started inside the gesture.
                    if let (Ok(buffer), Ok(source)) = (
                        context.create_buffer(1, 1, 22_050.0),
                        context.create_buffer_source(),
                    ) {
                        source.set_buffer(Some(&buffer));
                        let _ = source.connect_with_audio_node(&context.destination());
                        let _ = source.unchecked_ref::<AudioScheduledSourceNode>().start();
                    }
                }
            });
            if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                for event in GESTURES {
                    let _ = document
                        .add_event_listener_with_callback(event, unlock.as_ref().unchecked_ref());
                }
            }
            unlock
        });
        if context.is_none() {
            log::warn!("this browser has no Web Audio; the game plays silently");
        }
        Self {
            context,
            clips: BTreeMap::new(),
            voices: BTreeMap::new(),
            next_voice: 0,
            unlocked: false,
            unlock,
        }
    }

    /// Forgets voices that have finished, so a footstep every few frames does
    /// not accumulate a source per sound for as long as the game runs.
    fn reap(&mut self) {
        self.voices.retain(|_, voice| !voice.ended.get());
    }
}

impl Default for BrowserAudioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BrowserAudioBackend {
    fn drop(&mut self) {
        if let (Some(unlock), Some(document)) = (
            &self.unlock,
            web_sys::window().and_then(|window| window.document()),
        ) {
            for event in GESTURES {
                let _ = document
                    .remove_event_listener_with_callback(event, unlock.as_ref().unchecked_ref());
            }
        }
        if let Some(context) = &self.context {
            let _ = context.close();
        }
    }
}

impl AudioBackend for BrowserAudioBackend {
    fn register(&mut self, clip: AudioClip) -> Result<(), AudioError> {
        let decoded: Decoded = Rc::new(RefCell::new(None));
        // Without Web Audio the clip is kept as never-decoded, so the game
        // loads and plays on in silence rather than refusing to start.
        if let Some(context) = &self.context {
            let bytes = js_sys::Uint8Array::from(clip.bytes.as_slice()).buffer();
            let slot = Rc::clone(&decoded);
            let on_decoded = Closure::once(move |buffer: JsValue| {
                *slot.borrow_mut() = buffer.dyn_into::<AudioBuffer>().ok();
            });
            let id = clip.id.clone();
            let on_failed = Closure::once(move |error: JsValue| {
                log::error!("audio clip '{id}' could not be decoded: {error:?}");
            });
            // The promise's own handlers, so a refused clip is reported once
            // rather than also surfacing as an unhandled rejection.
            let decoding = context.decode_audio_data(&bytes).map_err(|error| {
                AudioError::Browser(format!("could not decode audio: {error:?}"))
            })?;
            let _ = decoding.then2(&on_decoded, &on_failed);
            on_decoded.forget();
            on_failed.forget();
        }
        self.clips.insert(clip.id, decoded);
        Ok(())
    }

    fn play(&mut self, clip: &str, settings: PlaybackSettings) -> Result<AudioVoiceId, AudioError> {
        self.reap();
        let context = self.context.as_ref().ok_or(AudioError::Locked)?;
        // A sound asked for while the context is still suspended would start
        // late, in a burst with every other one, the moment a tap resumed it.
        if !self.unlocked || context.state() != AudioContextState::Running {
            return Err(AudioError::Locked);
        }
        let decoded = self
            .clips
            .get(clip)
            .ok_or_else(|| AudioError::MissingClip(clip.to_owned()))?;
        // Still decoding, or refused by the browser: as good as missing, and
        // survivable in the same way.
        let Some(buffer) = decoded.borrow().clone() else {
            return Err(AudioError::MissingClip(clip.to_owned()));
        };
        let failed = |error: JsValue| AudioError::Browser(format!("could not play: {error:?}"));
        let source = context.create_buffer_source().map_err(failed)?;
        let gain = context.create_gain().map_err(failed)?;
        source.set_buffer(Some(&buffer));
        source.set_loop(settings.mode == PlaybackMode::Loop);
        gain.gain().set_value(settings.volume.clamp(0.0, 1.0));
        source.connect_with_audio_node(&gain).map_err(failed)?;
        gain.connect_with_audio_node(&context.destination())
            .map_err(failed)?;
        let ended = Rc::new(Cell::new(false));
        let finished = Rc::clone(&ended);
        let on_ended = Closure::<dyn FnMut()>::new(move || finished.set(true));
        source
            .unchecked_ref::<AudioScheduledSourceNode>()
            .set_onended(Some(on_ended.as_ref().unchecked_ref()));
        source
            .unchecked_ref::<AudioScheduledSourceNode>()
            .start()
            .map_err(failed)?;
        let voice = AudioVoiceId(self.next_voice);
        self.next_voice = self.next_voice.wrapping_add(1);
        self.voices.insert(
            voice,
            BrowserVoice {
                source,
                gain,
                ended,
                _on_ended: on_ended,
            },
        );
        Ok(voice)
    }

    fn stop(&mut self, voice: AudioVoiceId) {
        if let Some(voice) = self.voices.remove(&voice) {
            voice.halt();
        }
    }

    fn set_volume(&mut self, voice: AudioVoiceId, volume: f32) {
        if let Some(voice) = self.voices.get(&voice) {
            voice.gain.gain().set_value(volume.clamp(0.0, 1.0));
        }
    }

    fn is_active(&self, voice: AudioVoiceId) -> bool {
        self.voices
            .get(&voice)
            .is_some_and(|voice| !voice.ended.get())
    }

    // A buffer source cannot pause, but at a playback rate of zero it holds
    // its place, and a sound started while others are paused still plays —
    // what pausing every element did before.
    fn pause_all(&mut self) {
        for voice in self.voices.values() {
            voice.source.playback_rate().set_value(0.0);
        }
    }

    fn resume_all(&mut self) {
        for voice in self.voices.values() {
            voice.source.playback_rate().set_value(1.0);
        }
    }

    fn stop_all(&mut self) {
        for (_, voice) in std::mem::take(&mut self.voices) {
            voice.halt();
        }
    }

    fn unlock(&mut self) -> Result<(), AudioError> {
        self.unlocked = true;
        if let Some(context) = &self.context
            && context.state() != AudioContextState::Running
        {
            let _ = context.resume();
        }
        Ok(())
    }
}
