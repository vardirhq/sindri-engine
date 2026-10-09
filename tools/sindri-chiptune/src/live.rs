//! Continuous sample source: the audio thread owns its timeline and synth state.
use super::{Mood, Note, RATE, Settings, STEPS, voice, noise};
use rodio::Source;
use std::{num::NonZero, sync::{Arc, atomic::{AtomicU64, Ordering}, mpsc::{self, Sender, Receiver}}, time::Duration};

pub(super) enum Command {
    Update(Settings),
    Replace(Settings, [Vec<Note>; 3]),
    Seek(usize),
    Loop(bool),
}

pub(super) struct Transport {
    pub(super) tx: Sender<Command>,
    pub(super) position: Arc<AtomicU64>,
}

pub(super) struct LiveSource {
    settings: Settings,
    tracks: [Vec<Note>; 3],
    rx: Receiver<Command>,
    position: Arc<AtomicU64>,
    sample: u64,
    looped: bool,
    buffer: [f32; 256],
    offset: usize,
}
impl LiveSource {
    pub(super) fn new(settings: Settings, tracks: [Vec<Note>; 3]) -> (Self, Transport) {
        let (tx, rx) = mpsc::channel();
        let position = Arc::new(AtomicU64::new(0));
        let transport = Transport { tx, position: Arc::clone(&position) };
        let source = Self {
            settings, tracks, rx, position, sample: 0, looped: true,
            buffer: [0.0; 256], offset: 256,
        };
        (source, transport)
    }
    fn refresh(&mut self) {
        while let Ok(command) = self.rx.try_recv() {
            match command {
                Command::Update(settings) => self.settings = settings,
                Command::Replace(settings, tracks) => {
                    self.settings = settings;
                    self.tracks = tracks;
                    self.sample = 0;
                }
                Command::Seek(bar) => {
                    self.sample = (bar.min(31) as f64 * 16.0 * self.step() * f64::from(RATE)) as u64;
                }
                Command::Loop(value) => self.looped = value,
            }
        }
    }
    fn step(&self) -> f64 { 60.0 / self.settings.bpm / 4.0 }
    fn fill(&mut self) {
        self.refresh();
        let step = self.step();
        let total = (STEPS as f64 * step * f64::from(RATE)) as u64;
        for sample in &mut self.buffer {
            if self.sample >= total {
                if self.looped { self.sample = 0; } else { *sample = 0.0; continue; }
            }
            let t = self.sample as f64 / f64::from(RATE);
            let mut mixed = 0.0;
            for channel in 0..3 {
                if self.settings.rack.audible(channel) {
                    let sound = self.settings.rack.tracks[channel];
                    mixed += voice(&self.tracks[channel], t, step, sound)
                        * f64::from(sound.volume) * 0.45;
                }
            }
            if self.settings.rack.audible(3) {
                let sound = self.settings.rack.tracks[3];
                mixed += noise(self.settings.seed, t, step, self.settings.energy, self.settings.mood, sound.preset)
                    * f64::from(sound.volume) * 0.55;
            }
            *sample = mixed.clamp(-1.0, 1.0) as f32;
            self.sample += 1;
        }
        self.position.store(self.sample, Ordering::Relaxed);
        self.offset = 0;
    }
}
impl Iterator for LiveSource {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == self.buffer.len() { self.fill(); }
        let sample = self.buffer[self.offset];
        self.offset += 1;
        Some(sample)
    }
}
impl Source for LiveSource {
    fn current_span_len(&self) -> Option<usize> { None }
    fn channels(&self) -> NonZero<u16> { NonZero::new(1).expect("mono") }
    fn sample_rate(&self) -> NonZero<u32> { NonZero::new(RATE).expect("rate") }
    fn total_duration(&self) -> Option<Duration> { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_updates_do_not_reset_position() {
        let settings = Settings::default();
        let notes = super::super::compose_with(&settings);
        let (mut source, transport) = LiveSource::new(settings.clone(), notes);
        for _ in 0..1024 { source.next(); }
        let before = source.sample;
        let mut changed = settings;
        changed.rack.tracks[2].volume = 0.0;
        transport.tx.send(Command::Update(changed)).expect("send");
        for _ in 0..512 { source.next(); }
        assert!(source.sample > before);
    }
}
