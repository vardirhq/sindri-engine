//! Replace a bounded phrase without changing the surrounding score.
use super::{Note, STEPS, Settings, composer, synth};
use std::ops::Range;

/// Bar ranges are inclusive in the UI, half-open sixteenth steps in the score.
pub(super) fn regenerate(
    settings: &Settings,
    tracks: &[Vec<Note>; 3],
    track: usize,
    bars: Range<usize>,
    variation: u64,
) -> Result<Vec<Note>, &'static str> {
    if track >= 3 || bars.start >= bars.end || bars.end > 32 {
        return Err("Choose a melodic layer and a valid range within bars 1–32.");
    }
    let range = bars.start * 16..bars.end * 16;
    let mut replacement = composer::compose_track(settings, track, variation);
    if track != 0 {
        let baseline = composer::compose(settings);
        for note in &mut replacement {
            if let (Some(original), Some(edited)) = (
                root_at(&baseline[0], note.start),
                root_at(&tracks[0], note.start),
            ) {
                // Follow edited bass harmony, using the nearest pitch-class shift.
                let shift = (edited - original + 6).rem_euclid(12) - 6;
                note.pitch = (note.pitch + shift).clamp(0, 127);
            }
        }
    }
    Ok(replace_range(&tracks[track], &replacement, range))
}

/// Keep the outside portions of crossing notes and replace only the chosen interval.
pub(super) fn replace_range(
    source: &[Note],
    replacement: &[Note],
    range: Range<usize>,
) -> Vec<Note> {
    let mut result = Vec::new();
    for note in source {
        if note.start < range.start {
            let end = (note.start + note.len).min(range.start);
            result.push(Note {
                len: end - note.start,
                ..*note
            });
        }
        if note.start + note.len > range.end {
            let start = note.start.max(range.end);
            result.push(Note {
                start,
                len: note.start + note.len - start,
                ..*note
            });
        }
    }
    result.extend(replacement.iter().filter_map(|note| {
        let start = note.start.max(range.start);
        let end = (note.start + note.len).min(range.end).min(STEPS);
        (end > start).then_some(Note {
            start,
            len: end.saturating_sub(start),
            ..*note
        })
    }));
    result.sort_by_key(|note| note.start);
    result
}

pub(super) fn root_at(bass: &[Note], step: usize) -> Option<i32> {
    (0..=step / 8)
        .rev()
        .find_map(|cell| synth::bass_at(bass, cell * 8, 8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_is_deterministic_and_preserves_outside_material() {
        let settings = Settings::default();
        let tracks = composer::compose(&settings);
        for track in 0..3 {
            let result = regenerate(&settings, &tracks, track, 8..12, 1).unwrap();
            assert_eq!(
                result,
                regenerate(&settings, &tracks, track, 8..12, 1).unwrap()
            );
            for note in &tracks[track] {
                if note.start + note.len <= 128 || note.start >= 192 {
                    assert!(result.contains(note));
                }
            }
            let mut edited = tracks.clone();
            edited[track] = result;
            super::super::midi::validate(&edited).unwrap();
        }
    }

    #[test]
    fn crossing_notes_are_split_and_edited_bass_transposes_the_new_phrase() {
        let settings = Settings::default();
        let mut tracks = composer::compose(&settings);
        let before = regenerate(&settings, &tracks, 2, 8..12, 3).unwrap();
        for note in &mut tracks[0] {
            note.pitch += 1;
        }
        let after = regenerate(&settings, &tracks, 2, 8..12, 3).unwrap();
        for (a, b) in before.iter().zip(&after) {
            assert_eq!(b.pitch, a.pitch + i32::from((128..192).contains(&a.start)));
        }
        tracks[2] = vec![Note {
            pitch: 60,
            start: 120,
            len: 96,
            velocity: 80,
        }];
        let result = regenerate(&settings, &tracks, 2, 8..12, 1).unwrap();
        assert!(result.contains(&Note {
            pitch: 60,
            start: 120,
            len: 8,
            velocity: 80
        }));
        assert!(result.contains(&Note {
            pitch: 60,
            start: 192,
            len: 24,
            velocity: 80
        }));
        assert!(regenerate(&settings, &tracks, 2, 4..4, 1).is_err());
        assert!(regenerate(&settings, &tracks, 3, 0..4, 1).is_err());
    }
}
