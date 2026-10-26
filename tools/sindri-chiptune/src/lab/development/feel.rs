use super::step_range;
use crate::lab::{Note, Rng, Settings};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::lab) enum Accent {
    Downbeats,
    Backbeat,
    Offbeats,
}

pub(in crate::lab) struct Feel {
    pub(in crate::lab) strength: f64,
    pub(in crate::lab) gate: usize,
    pub(in crate::lab) accent: Accent,
}
impl Default for Feel {
    fn default() -> Self {
        Self {
            strength: 0.4,
            gate: 90,
            accent: Accent::Downbeats,
        }
    }
}

// Every result is rounded and clamped to the MIDI velocity range before casting.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn velocity(value: f64) -> u8 {
    value.round().clamp(1.0, 127.0) as u8
}

/// Keeps pitch and onset timing. Crossing notes stay untouched; gate only shortens.
pub(in crate::lab) fn shape_performance(
    settings: &Settings,
    notes: &[Note],
    bars: Range<usize>,
    feel: &Feel,
    variation: u64,
) -> Result<Vec<Note>, &'static str> {
    let range = step_range(bars)?;
    if !(0.0..=1.0).contains(&feel.strength) || !(50..=100).contains(&feel.gate) {
        return Err("Expression must be 0–1 and gate length 50–100 percent.");
    }
    let mut result = notes.to_vec();
    let mut eligible = false;
    for note in &mut result {
        if note.start < range.start || note.start + note.len > range.end {
            continue;
        }
        eligible = true;
        let accent = match feel.accent {
            Accent::Downbeats => note.start.is_multiple_of(16),
            Accent::Backbeat => [4, 12].contains(&(note.start % 16)),
            Accent::Offbeats => note.start % 4 == 2,
        };
        let position = i32::try_from((note.start - range.start) % 64).expect("phrase step");
        let swell = 8 - (position - 32).abs() / 2;
        let mut rng = Rng::new(
            settings.seed
                ^ variation.rotate_left(23)
                ^ u64::try_from(note.start).expect("score step")
                ^ 0x5045_5246_4f52_4d31,
        );
        let jitter = i32::try_from(rng.range(17)).expect("small jitter") - 8;
        let adjustment = f64::from(if accent { 12 } else { -3 } + swell + jitter) * feel.strength;
        note.velocity = velocity(f64::from(note.velocity) + adjustment);
        note.len = (note.len * feel.gate / 100).max(1);
    }
    if !eligible {
        return Err("No complete notes lie inside the selected bars.");
    }
    Ok(result)
}
