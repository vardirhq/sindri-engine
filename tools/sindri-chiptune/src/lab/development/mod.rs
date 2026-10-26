//! Explicit musical transformations; default seeded composition stays unchanged.
mod dialogue;
mod family;
mod feel;
mod motif;

use super::{BARS, Mood, Note, Settings, synth};
pub(super) use dialogue::{Dialogue, arrange_dialogue};
pub(super) use family::{FAMILY_NAMES, family};
pub(super) use feel::{Accent, Feel, shape_performance};
pub(super) use motif::{Motif, Shape, develop_motif};
use std::ops::Range;

pub(super) fn step_range(bars: Range<usize>) -> Result<Range<usize>, &'static str> {
    if bars.start >= bars.end || bars.end > BARS {
        return Err("Select a valid bar range within bars 1–32.");
    }
    Ok(bars.start * 16..bars.end * 16)
}

fn clipped(notes: &[Note], range: &Range<usize>) -> Vec<Note> {
    notes
        .iter()
        .filter_map(|note| {
            let start = note.start.max(range.start);
            let end = (note.start + note.len).min(range.end);
            (end > start).then_some(Note {
                start,
                len: end.saturating_sub(start),
                ..*note
            })
        })
        .collect()
}

/// Closest diatonic position plus its chromatic offset, including negative octaves.
fn scale_position(pitch: i32, key: i32, mood: Mood) -> (i32, i32) {
    let scale = synth::scale(mood);
    let index = (-14..=84)
        .min_by_key(|index| (scale_pitch(*index, key, mood) - pitch).abs())
        .expect("scale positions");
    (
        index,
        pitch
            - (key
                + index.div_euclid(7) * 12
                + scale[usize::try_from(index.rem_euclid(7)).expect("degree")]),
    )
}

fn scale_pitch(index: i32, key: i32, mood: Mood) -> i32 {
    key + index.div_euclid(7) * 12
        + synth::scale(mood)[usize::try_from(index.rem_euclid(7)).expect("degree")]
}

fn shifted(pitch: i32, settings: &Settings, steps: i32) -> i32 {
    let (index, chromatic) = scale_position(pitch, settings.key, settings.mood);
    (scale_pitch(index + steps, settings.key, settings.mood) + chromatic).clamp(0, 127)
}

fn nearest_class(pitch: i32, class: i32) -> i32 {
    (0..=127)
        .filter(|candidate| (candidate - class).rem_euclid(12) == 0)
        .min_by_key(|candidate| (*candidate - pitch).abs())
        .expect("pitch class")
}

#[cfg(test)]
mod tests;
