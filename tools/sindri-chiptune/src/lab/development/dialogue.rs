use super::{clipped, nearest_class, shifted, step_range};
use crate::lab::{Note, Settings, midi, phrases};
use std::ops::Range;

pub(in crate::lab) struct Dialogue {
    pub(in crate::lab) caller: usize,
    pub(in crate::lab) phrase_bars: usize,
}
impl Default for Dialogue {
    fn default() -> Self {
        Self {
            caller: 2,
            phrase_bars: 2,
        }
    }
}

/// Rewrites Arpeggio and Lead into alternating complete bar phrases.
pub(in crate::lab) fn arrange_dialogue(
    settings: &Settings,
    tracks: &[Vec<Note>; 3],
    bars: Range<usize>,
    options: &Dialogue,
) -> Result<[Vec<Note>; 3], &'static str> {
    midi::validate(tracks)?;
    let range = step_range(bars)?;
    if !(1..=2).contains(&options.caller) || !(1..=2).contains(&options.phrase_bars) {
        return Err("Dialogue uses Arpeggio and Lead with one- or two-bar phrases.");
    }
    let length = options.phrase_bars * 16;
    if !range.len().is_multiple_of(length * 2) {
        return Err("Select complete call/response pairs: a multiple of 2 or 4 bars.");
    }
    let responder = 3 - options.caller;
    let mut calls = Vec::new();
    let mut answers = Vec::new();
    for start in (range.start..range.end).step_by(length * 2) {
        let phrase = clipped(&tracks[options.caller], &(start..start + length));
        for note in &phrase {
            let when = note.start + length;
            let role_shift = if responder == 1 { -12 } else { 12 };
            let mut pitch = shifted(note.pitch, settings, -1) + role_shift;
            if let (Some(from), Some(to)) = (
                phrases::root_at(&tracks[0], note.start),
                phrases::root_at(&tracks[0], when),
            ) {
                pitch += (to - from + 6).rem_euclid(12) - 6;
            }
            answers.push(Note {
                pitch: pitch.clamp(0, 127),
                start: when,
                ..*note
            });
        }
        if let Some(last) = answers
            .last_mut()
            .filter(|note| note.start >= start + length)
            && let Some(root) = phrases::root_at(&tracks[0], last.start)
        {
            last.pitch = nearest_class(last.pitch, root);
        }
        calls.extend(phrase);
    }
    if calls.is_empty() {
        return Err(
            "The selected call phrases contain no notes. Choose other bars or the other caller.",
        );
    }
    let mut result = tracks.clone();
    result[options.caller] = phrases::replace_range(&tracks[options.caller], &calls, range.clone());
    result[responder] = phrases::replace_range(&tracks[responder], &answers, range);
    midi::validate(&result)?;
    Ok(result)
}
