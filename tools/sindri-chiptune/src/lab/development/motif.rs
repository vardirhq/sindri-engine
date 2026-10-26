use super::{clipped, scale_pitch, scale_position, shifted, step_range};
use crate::lab::{Note, Settings, midi, phrases};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::lab) enum Shape {
    Repeat,
    Sequence,
    Invert,
}

pub(in crate::lab) struct Motif {
    /// Zero-based destination bar.
    pub(in crate::lab) target: usize,
    pub(in crate::lab) shape: Shape,
    pub(in crate::lab) steps: i32,
    pub(in crate::lab) stretch: usize,
    pub(in crate::lab) follow_harmony: bool,
}
impl Default for Motif {
    fn default() -> Self {
        Self {
            target: 16,
            shape: Shape::Sequence,
            steps: 2,
            stretch: 1,
            follow_harmony: true,
        }
    }
}

pub(in crate::lab) fn develop_motif(
    settings: &Settings,
    tracks: &[Vec<Note>; 3],
    layer: usize,
    bars: Range<usize>,
    options: &Motif,
) -> Result<[Vec<Note>; 3], &'static str> {
    midi::validate(tracks)?;
    let range = step_range(bars)?;
    if layer >= 3
        || options.target >= 32
        || !(1..=2).contains(&options.stretch)
        || !(-7..=7).contains(&options.steps)
    {
        return Err("Choose a melodic layer, destination, and bounded motif settings.");
    }
    let target = options.target * 16;
    let length = range.len() * options.stretch;
    if target + length > crate::lab::STEPS {
        return Err("The developed motif would extend beyond bar 32.");
    }
    let source = clipped(&tracks[layer], &range);
    let first = source
        .first()
        .ok_or("There are no source notes in these bars.")?;
    let (axis, _) = scale_position(first.pitch, settings.key, settings.mood);
    let notes = source
        .iter()
        .map(|note| {
            let start = target + (note.start - range.start) * options.stretch;
            let mut pitch = match options.shape {
                Shape::Repeat => note.pitch,
                Shape::Sequence => shifted(note.pitch, settings, options.steps),
                Shape::Invert => {
                    let (index, chromatic) =
                        scale_position(note.pitch, settings.key, settings.mood);
                    (scale_pitch(2 * axis - index, settings.key, settings.mood) - chromatic)
                        .clamp(0, 127)
                }
            };
            if layer != 0
                && options.follow_harmony
                && let (Some(from), Some(to)) = (
                    phrases::root_at(&tracks[0], note.start),
                    phrases::root_at(&tracks[0], start),
                )
            {
                pitch = (pitch + (to - from + 6).rem_euclid(12) - 6).clamp(0, 127);
            }
            Note {
                pitch,
                start,
                len: note.len * options.stretch,
                ..*note
            }
        })
        .collect::<Vec<_>>();
    let mut result = tracks.clone();
    result[layer] = phrases::replace_range(&tracks[layer], &notes, target..target + length);
    midi::validate(&result)?;
    Ok(result)
}
