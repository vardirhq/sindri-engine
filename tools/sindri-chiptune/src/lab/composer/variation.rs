//! Deterministic variations that start from an existing score.
use super::{Mood, Note, Rng, Settings};

/// Make a deterministic, in-key variation of an existing lead. Notes in the
/// same position of a four-bar phrase move together, so repeated motifs stay
/// recognizable. Rhythm, velocity, and the final note remain unchanged.
pub(in crate::lab) fn vary_lead(notes: &[Note], settings: &Settings, variation: u64) -> Vec<Note> {
    if notes.is_empty() {
        return Vec::new();
    }

    let mut order: Vec<usize> = (0..notes.len()).collect();
    order.sort_by_key(|index| notes[*index].start);
    let mut cells = vec![(0_usize, 0_usize); notes.len()];
    let mut last_bar = None;
    let mut note_in_bar = 0;
    for index in order {
        let bar = notes[index].start / 16;
        if last_bar != Some(bar) {
            last_bar = Some(bar);
            note_in_bar = 0;
        }
        cells[index] = (bar % 4, note_in_bar);
        note_in_bar += 1;
    }

    let mut unique_cells = Vec::new();
    for &cell in &cells {
        if !unique_cells.contains(&cell) {
            unique_cells.push(cell);
        }
    }
    let cell_count = unique_cells.len();
    let first_cell = usize::try_from(variation % u64::try_from(cell_count).expect("notes"))
        .expect("bounded cell");
    let mut varied_cells = vec![unique_cells[first_cell]];
    if cell_count > 1 {
        varied_cells.push(unique_cells[(first_cell + 1) % cell_count]);
    }

    let final_note = notes
        .iter()
        .enumerate()
        .max_by_key(|(_, note)| note.start)
        .map(|(index, _)| index);
    let mut varied = notes.to_vec();
    for (index, note) in notes.iter().enumerate() {
        if Some(index) == final_note || !varied_cells.contains(&cells[index]) {
            continue;
        }
        let (bar, ordinal) = cells[index];
        let cell = (u64::try_from(bar).expect("four bars") << 32)
            | u64::try_from(ordinal).expect("bounded notes per bar");
        let mut rng = Rng::new(settings.seed ^ variation.rotate_left(31) ^ cell ^ 0xd4b1_208f);
        let step = i32::try_from(rng.range(2) + 1).expect("one or two scale steps");
        let direction = if rng.range(2) == 0 { -1 } else { 1 };
        varied[index].pitch =
            move_scale_steps(note.pitch, settings.key, settings.mood, direction * step);
    }
    varied
}

fn move_scale_steps(pitch: i32, key: i32, mood: Mood, steps: i32) -> i32 {
    let scale = super::super::synth::scale(mood);
    let pitches: Vec<i32> = (0..=127)
        .filter(|candidate| scale.contains(&(*candidate - key).rem_euclid(12)))
        .collect();
    let (index, _) = pitches
        .iter()
        .enumerate()
        .min_by_key(|(_, candidate)| (**candidate - pitch).abs())
        .expect("a musical scale has notes");
    let distance = usize::try_from(steps.abs()).expect("small scale step");
    let target = if steps < 0 {
        index.saturating_sub(distance)
    } else {
        (index + distance).min(pitches.len() - 1)
    };
    pitches[target]
}
