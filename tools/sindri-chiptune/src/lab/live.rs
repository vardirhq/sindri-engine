//! Continuous playback: the audio thread owns an engine and takes commands.
#[cfg(not(target_arch = "wasm32"))]
use super::RATE;
use super::{Note, Settings, synth::Engine};
#[cfg(not(target_arch = "wasm32"))]
use rodio::Source;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, Sender},
};
#[cfg(not(target_arch = "wasm32"))]
use std::{num::NonZero, time::Duration};

pub(super) enum Command {
    Update(Settings),
    Replace(Settings, [Vec<Note>; 3]),
    Edit([Vec<Note>; 3]),
    Seek(usize),
    Loop(bool),
}

/// How many of the most recent output frames the oscilloscope sees.
pub(super) const SCOPE: usize = 1024;
/// Frames rendered at a time between commands.
const BLOCK: usize = 256;

pub(super) struct Transport {
    pub(super) tx: Sender<Command>,
    pub(super) position: Arc<AtomicU64>,
    /// The latest output as mono, oldest first. Never blocks the audio thread.
    pub(super) scope: Arc<Mutex<Vec<f32>>>,
}

pub(super) struct LiveSource {
    engine: Engine,
    rx: Receiver<Command>,
    position: Arc<AtomicU64>,
    scope: Arc<Mutex<Vec<f32>>>,
    looped: bool,
    finished: bool,
    buffer: Vec<[f32; 2]>,
    /// The next interleaved sample to hand out, counting both channels.
    offset: usize,
}

impl LiveSource {
    pub(super) fn new(settings: Settings, tracks: [Vec<Note>; 3]) -> (Self, Transport) {
        let (tx, rx) = mpsc::channel();
        let position = Arc::new(AtomicU64::new(0));
        let scope = Arc::new(Mutex::new(vec![0.0; SCOPE]));
        let transport = Transport {
            tx,
            position: Arc::clone(&position),
            scope: Arc::clone(&scope),
        };
        let source = Self {
            engine: Engine::new(settings, tracks),
            rx,
            position,
            scope,
            looped: true,
            finished: false,
            buffer: Vec::new(),
            offset: 0,
        };
        (source, transport)
    }

    fn refresh(&mut self) {
        while let Ok(command) = self.rx.try_recv() {
            match command {
                Command::Update(settings) => self.engine.set_settings(settings),
                Command::Edit(tracks) => self.engine.set_tracks(tracks),
                Command::Replace(settings, tracks) => {
                    self.engine.set_settings(settings);
                    self.engine.set_tracks(tracks);
                    self.engine.seek(0);
                    self.finished = false;
                }
                Command::Seek(bar) => {
                    self.engine.seek_bar(bar);
                    self.finished = false;
                }
                Command::Loop(value) => self.looped = value,
            }
        }
    }

    fn fill(&mut self) {
        self.refresh();
        self.buffer.resize(BLOCK, [0.0; 2]);
        let written = self.engine.render(&mut self.buffer, self.looped);
        self.buffer.truncate(written);
        if written == 0 {
            self.finished = true;
        }
        self.position
            .store(self.engine.position(), Ordering::Relaxed);
        // A frame the UI is reading is skipped rather than waited for.
        if let Ok(mut scope) = self.scope.try_lock() {
            let fresh = self.buffer.len().min(SCOPE);
            scope.drain(..fresh);
            scope.extend(self.buffer[..fresh].iter().map(|[l, r]| (l + r) * 0.5));
        }
        self.offset = 0;
    }

    /// The position in samples the transport shows, for tests.
    #[cfg(test)]
    fn sample(&self) -> u64 {
        self.engine.position()
    }
}

impl Iterator for LiveSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.buffer.len() * 2 {
            if self.finished {
                return None;
            }
            self.fill();
            if self.buffer.is_empty() {
                return None;
            }
        }
        let frame = self.buffer[self.offset / 2];
        let sample = frame[self.offset % 2];
        self.offset += 1;
        Some(sample)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Source for LiveSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> NonZero<u16> {
        NonZero::new(2).expect("stereo")
    }
    fn sample_rate(&self) -> NonZero<u32> {
        NonZero::new(RATE).expect("rate")
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> (LiveSource, Transport) {
        let settings = Settings::default();
        LiveSource::new(settings.clone(), crate::lab::compose_with(&settings))
    }

    fn play(source: &mut LiveSource, frames: usize) {
        for _ in 0..frames * 2 {
            source.next();
        }
    }

    #[test]
    fn tempo_updates_preserve_musical_position() {
        let (mut source, transport) = source();
        play(&mut source, 44_100);
        let before = source.sample();
        let mut faster = Settings::default();
        faster.bpm *= 2.0;
        transport.tx.send(Command::Update(faster)).expect("send");
        source.refresh();
        assert!(source.sample().abs_diff(before / 2) <= 1);
    }

    #[test]
    fn note_edits_preserve_the_timeline() {
        let (mut source, transport) = source();
        play(&mut source, 22_050);
        let before = source.sample();
        transport
            .tx
            .send(Command::Edit(std::array::from_fn(|_| Vec::new())))
            .expect("send");
        source.refresh();
        assert_eq!(source.sample(), before);
    }

    #[test]
    fn non_looping_playback_finishes_after_its_tails() {
        let (mut source, transport) = source();
        transport.tx.send(Command::Loop(false)).expect("send");
        transport.tx.send(Command::Seek(31)).expect("send");
        let mut count = 0_u64;
        while source.next().is_some() {
            count += 1;
            assert!(count < 20_000_000, "never finished");
        }
        assert!(count > 0);
    }

    #[test]
    fn playback_is_stereo_and_audible() {
        let (mut source, transport) = source();
        transport.tx.send(Command::Seek(9)).expect("send");
        let samples: Vec<f32> = (0..88_200).filter_map(|_| source.next()).collect();
        let peak = samples.iter().fold(0.0_f32, |a, b| a.max(b.abs()));
        let side: f32 = samples.chunks(2).map(|f| (f[0] - f[1]).abs()).sum();
        assert!(peak > 0.1 && peak <= 1.0, "{peak}");
        assert!(side > 1.0, "left and right should differ");
    }
}
