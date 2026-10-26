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
mod development;
mod ensemble;
mod gui;
mod instruments;
mod live;
mod midi;
#[cfg(not(target_arch = "wasm32"))]
mod ogg;
mod phrases;
mod project;
mod seed;
mod sfx;
mod song;
mod synth;
mod wav;

#[derive(Clone, Debug, PartialEq)]
struct Settings {
    seed: u64,
    generation: Generation,
    seed_text: Option<String>,
    bpm: f64,
    key: i32,
    energy: f64,
    mood: Mood,
    output: PathBuf,
    rack: instruments::Rack,
}
/// Versioned composition and accompaniment rules; missing saved fields mean legacy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Generation {
    #[default]
    Legacy,
    Musical,
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
            generation: Generation::Musical,
            seed_text: None,
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

#[cfg(test)]
fn step_seconds() -> f64 {
    60.0 / BPM / 4.0
}

/// A whole song as interleaved stereo samples, ready to loop.
fn render_samples(settings: &Settings, tracks: &[Vec<Note>; 3]) -> Vec<f32> {
    synth::render_song(settings, tracks)
        .into_iter()
        .flatten()
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn render_with(settings: &Settings, tracks: &[Vec<Note>; 3]) -> Result<(), Box<dyn Error>> {
    wav::write(&settings.output, &render_samples(settings, tracks), 2)
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
            generation: Generation::Musical,
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
