//! Continuous sample source: the audio thread owns its timeline and synth state.
use super::{Note, RATE, STEPS, Settings, noise, voice};
use rodio::Source;
use std::{
    num::NonZero,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::Duration,
};

pub(super) enum Command {
    Update(Settings),
    Replace(Settings, [Vec<Note>; 3]),
    Edit([Vec<Note>; 3]),
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
    cursors: [usize; 3],
    offset: usize,
}
impl LiveSource {
    pub(super) fn new(settings: Settings, tracks: [Vec<Note>; 3]) -> (Self, Transport) {
        let (tx, rx) = mpsc::channel();
        let position = Arc::new(AtomicU64::new(0));
        let transport = Transport {
            tx,
            position: Arc::clone(&position),
        };
        let source = Self {
            settings,
            tracks,
            rx,
            position,
            sample: 0,
            looped: true,
            buffer: [0.0; 256],
            offset: 256,
            cursors: [0; 3],
        };
        (source, transport)
    }
    // Song length and seek position are bounded to minutes at 44.1 kHz.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn refresh(&mut self) {
        while let Ok(command) = self.rx.try_recv() {
            match command {
                Command::Update(settings) => {
                    // Preserve musical position across tempo changes, then re-seek cursors.
                    self.sample = (f64::from(u32::try_from(self.sample).expect("short song"))
                        * self.settings.bpm
                        / settings.bpm)
                        .round() as u64;
                    self.settings = settings;
                    self.cursors = [0; 3];
                }
                Command::Edit(tracks) => {
                    self.tracks = tracks;
                    self.cursors = [0; 3];
                }
                Command::Replace(settings, tracks) => {
                    self.settings = settings;
                    self.tracks = tracks;
                    self.sample = 0;
                    self.cursors = [0; 3];
                }
                Command::Seek(bar) => {
                    self.sample = (f64::from(u32::try_from(bar.min(31)).expect("bar"))
                        * 16.0
                        * self.step()
                        * f64::from(RATE)) as u64;
                    self.cursors = [0; 3];
                }
                Command::Loop(value) => self.looped = value,
            }
        }
    }
    fn step(&self) -> f64 {
        60.0 / self.settings.bpm / 4.0
    }
    // The timeline is always below u32::MAX samples; conversion retains precision.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn fill(&mut self) {
        self.refresh();
        let step = self.step();
        let total =
            (f64::from(u32::try_from(STEPS).expect("step count")) * step * f64::from(RATE)) as u64;
        for sample in &mut self.buffer {
            if self.sample >= total {
                if self.looped {
                    self.sample = 0;
                    self.cursors = [0; 3];
                } else {
                    *sample = 0.0;
                    continue;
                }
            }
            let t =
                f64::from(u32::try_from(self.sample).expect("short composition")) / f64::from(RATE);
            for (cursor, notes) in self.cursors.iter_mut().zip(&self.tracks) {
                while *cursor < notes.len()
                    && f64::from(
                        u32::try_from(notes[*cursor].start + notes[*cursor].len)
                            .expect("note step"),
                    ) * step
                        <= t
                {
                    *cursor += 1;
                }
            }
            let mut mixed = 0.0;
            for channel in 0..3 {
                if self.settings.rack.audible(channel) {
                    let sound = self.settings.rack.tracks[channel];
                    mixed += voice(
                        &self.tracks[channel][self.cursors[channel]..],
                        t,
                        step,
                        sound,
                    ) * f64::from(sound.volume)
                        * 0.45;
                }
            }
            if self.settings.rack.audible(3) {
                let sound = self.settings.rack.tracks[3];
                mixed += noise(
                    self.settings.seed,
                    t,
                    step,
                    self.settings.energy,
                    self.settings.mood,
                    sound.preset,
                ) * f64::from(sound.volume)
                    * 0.55;
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
    // The fixed song length fits in u64 samples for the supported tempo range.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == self.buffer.len() {
            self.refresh();
            if !self.looped
                && self.sample
                    >= (f64::from(u32::try_from(STEPS).expect("steps"))
                        * self.step()
                        * f64::from(RATE)) as u64
            {
                return None;
            }
            self.fill();
        }
        let sample = self.buffer[self.offset];
        self.offset += 1;
        Some(sample)
    }
}
impl Source for LiveSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> NonZero<u16> {
        NonZero::new(1).expect("mono")
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
    #[test]
    fn tempo_updates_preserve_musical_position() {
        let settings = Settings::default();
        let (mut source, transport) =
            LiveSource::new(settings.clone(), crate::desktop::compose_with(&settings));
        source.sample = 44_100;
        let mut changed = settings;
        changed.bpm *= 2.0;
        transport.tx.send(Command::Update(changed)).expect("send");
        source.refresh();
        assert_eq!(source.sample, 22_050);
        assert_eq!(source.cursors, [0; 3]);
    }
    #[test]
    fn note_edits_preserve_the_timeline() {
        let settings = Settings::default();
        let (mut source, transport) =
            LiveSource::new(settings.clone(), crate::desktop::compose_with(&settings));
        source.sample = 44_100;
        transport
            .tx
            .send(Command::Edit(std::array::from_fn(|_| Vec::new())))
            .expect("send");
        source.refresh();
        assert_eq!(source.sample, 44_100);
        assert!(source.tracks.iter().all(Vec::is_empty));
    }
    #[test]
    fn non_looping_playback_finishes() {
        let settings = Settings::default();
        let (mut source, transport) =
            LiveSource::new(settings.clone(), crate::desktop::compose_with(&settings));
        source.sample = 6_000_000;
        transport.tx.send(Command::Loop(false)).expect("send");
        assert!(source.next().is_none());
    }
    #[test]
    fn live_updates_do_not_reset_position() {
        let settings = Settings::default();
        let notes = super::super::compose_with(&settings);
        let (mut source, transport) = LiveSource::new(settings.clone(), notes);
        for _ in 0..1024 {
            source.next();
        }
        let before = source.sample;
        let mut changed = settings;
        changed.rack.tracks[2].volume = 0.0;
        transport.tx.send(Command::Update(changed)).expect("send");
        for _ in 0..512 {
            source.next();
        }
        assert!(source.sample > before);
    }
}
