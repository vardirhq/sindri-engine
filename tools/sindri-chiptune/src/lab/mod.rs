//! The note model, settings and offline rendering shared by both hosts.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::{env, error::Error};

const RATE: u32 = 44_100;
const BPM: f64 = 118.0;
#[cfg(not(target_arch = "wasm32"))]
mod cli;
mod composer;
mod gui;
mod instruments;
mod live;
mod midi;
mod percussion;
mod project;
mod sfx;
mod song;
mod wav;

#[derive(Clone, Debug, PartialEq)]
struct Settings {
    seed: u64,
    bpm: f64,
    key: i32,
    energy: f64,
    mood: Mood,
    output: PathBuf,
    rack: instruments::Rack,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mood {
    Mysterious,
    Hopeful,
    Tense,
    Melancholic,
}
impl Settings {
    fn default() -> Self {
        Self {
            seed: 42,
            bpm: BPM,
            key: 2,
            energy: 0.45,
            mood: Mood::Mysterious,
            output: PathBuf::from("chiptune.wav"),
            rack: instruments::Rack::default(),
        }
    }
}

const BARS: usize = 32;
const STEPS: usize = BARS * 16;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Note {
    pitch: i32,
    start: usize,
    len: usize,
    velocity: u8,
}

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn range(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[cfg(test)]
fn compose(seed: u64) -> [Vec<Note>; 3] {
    compose_with(&Settings {
        seed,
        ..Settings::default()
    })
}
fn compose_with(settings: &Settings) -> [Vec<Note>; 3] {
    composer::compose(settings)
}

fn freq(midi: i32) -> f64 {
    440.0 * 2_f64.powf((f64::from(midi) - 69.0) / 12.0)
}
#[cfg(test)]
fn step_seconds() -> f64 {
    60.0 / BPM / 4.0
}

fn voice(notes: &[Note], t: f64, step: f64, sound: instruments::TrackSound) -> f64 {
    let active = notes.first().filter(|n| {
        let start = f64::from(u32::try_from(n.start).expect("start")) * step;
        t >= start && t < start + f64::from(u32::try_from(n.len).expect("length")) * step
    });
    let Some(note) = active else {
        return 0.0;
    };
    let local = t - f64::from(u32::try_from(note.start).expect("start")) * step;
    let duration = f64::from(u32::try_from(note.len).expect("length")) * step;
    let hz = freq(note.pitch + sound.octave * 12);
    sound.preset.sample(t * hz, local, duration) * f64::from(note.velocity) / 127.0
}

fn noise(
    seed: u64,
    time: f64,
    step: f64,
    energy: f64,
    mood: Mood,
    preset: instruments::Preset,
) -> f64 {
    percussion::sample(seed, time, step, energy, mood, preset)
}

// Supported tempos bound the sample count; normalized audio is clamped to i16 PCM.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn render_samples(settings: &Settings, tracks: &[Vec<Note>; 3]) -> Vec<f32> {
    let step = 60.0 / settings.bpm / 4.0;
    let seconds = f64::from(u32::try_from(STEPS).expect("step count")) * step;
    let samples = (seconds * f64::from(RATE)).round() as u32;
    let mut audio = Vec::with_capacity(samples as usize);
    // Cache current notes by advancing cursors, not searching all notes per sample.
    let mut cursors = [0_usize; 3];
    for i in 0..samples {
        let t = f64::from(i) / f64::from(RATE);
        for (cursor, notes) in cursors.iter_mut().zip(tracks) {
            while *cursor < notes.len()
                && f64::from(
                    u32::try_from(notes[*cursor].start + notes[*cursor].len).expect("step"),
                ) * step
                    <= t
            {
                *cursor += 1;
            }
        }
        let mut mixed = 0.0;
        for channel in 0..3 {
            if settings.rack.audible(channel) {
                let sound = settings.rack.tracks[channel];
                mixed += voice(&tracks[channel][cursors[channel]..], t, step, sound)
                    * f64::from(sound.volume)
                    * 0.45;
            }
        }
        if settings.rack.audible(3) {
            let sound = settings.rack.tracks[3];
            mixed += noise(
                settings.seed,
                t,
                step,
                settings.energy,
                settings.mood,
                sound.preset,
            ) * f64::from(sound.volume)
                * 0.55;
        }
        let sample = (mixed.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        audio.push(f32::from(sample) / 32767.0);
    }
    audio
}

#[cfg(not(target_arch = "wasm32"))]
fn render_with(settings: &Settings, tracks: &[Vec<Note>; 3]) -> Result<(), Box<dyn Error>> {
    wav::write(&settings.output, &render_samples(settings, tracks))
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn run() -> Result<(), Box<dyn Error>> {
    if env::args().len() == 1 {
        return gui::run();
    }
    cli::run(env::args().skip(1).collect())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn start_web() {
    gui::start_web();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_generation() {
        assert_eq!(compose(123), compose(123));
        assert_ne!(compose(123), compose(456));
    }
    #[test]
    fn notes_are_valid() {
        let tracks = compose(42);
        assert!(
            tracks
                .iter()
                .all(|notes| notes.iter().all(|n| n.len > 0 && n.start + n.len <= STEPS))
        );
    }
    #[test]
    fn duration_is_expected() {
        assert!(
            (f64::from(u32::try_from(STEPS).expect("steps")) * step_seconds() - 65.0847).abs()
                < 0.001
        );
    }
    #[test]
    fn different_seeds_change_melody_and_accompaniment() {
        let a = compose_with(&Settings {
            seed: 42,
            ..Settings::default()
        });
        let b = compose_with(&Settings {
            seed: 43,
            ..Settings::default()
        });
        assert_ne!(a[2], b[2], "melody should change");
        assert_ne!(a[1], b[1], "arpeggio should change");
    }
    #[test]
    fn mood_and_energy_change_composition() {
        let default = Settings::default();
        let original = compose_with(&default);
        let hopeful = compose_with(&Settings {
            mood: Mood::Hopeful,
            ..default.clone()
        });
        let energetic = compose_with(&Settings {
            energy: 0.9,
            ..default
        });
        assert_ne!(original[0], hopeful[0], "mood should change harmony");
        assert_ne!(original[1], energetic[1], "energy should change density");
    }
}
