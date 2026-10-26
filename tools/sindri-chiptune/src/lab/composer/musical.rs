//! Versioned rules shared by full generation and selective regeneration.
use super::{Bar, Plan, tune};

/// The Rise develops the theme's head, then turns its tail the other way.
pub(super) fn develop(theme: &Bar) -> Bar {
    let pivot = theme.0.get(theme.0.len() / 2).map_or(0, |n| n.2);
    Bar(theme
        .0
        .iter()
        .enumerate()
        .map(|(i, &(slot, length, degree))| {
            let degree = if i <= theme.0.len() / 2 {
                degree
            } else {
                2 * pivot - degree
            };
            (slot, length, degree + 2)
        })
        .collect())
}

/// Four-bar questions remain open; eight-bar answers prepare and reach home.
/// These are modal cadences: no automatic major dominant in a minor mode.
pub(super) fn chord(plan: &Plan, bar: usize) -> i32 {
    if !plan.musical {
        return plan.style.chord(bar);
    }
    match bar % 8 {
        6 => plan.style.dominant,
        7 if bar == 7 => plan.style.dominant,
        7 => 0,
        _ => plan.style.chord(bar),
    }
}

/// Dense foreground phrases get a steadier, quieter accompaniment rhythm.
/// Cadences keep their space; no layer's section participation is changed.
pub(super) fn accompaniment_gap(plan: &Plan, bar: usize, original: usize) -> usize {
    let density = tune(plan, bar).map_or(0, |line| line.0.len());
    if density >= 6 {
        original.max(4)
    } else if density >= 4 {
        original.max(2)
    } else {
        original
    }
}

/// Keep the arpeggio below the sounding lead and choose the nearest octave of
/// its planned pitch class. Pitch classes and deliberate rhythmic figures stay.
pub(super) fn arp_pitch(
    plan: &Plan,
    bar: usize,
    slot: usize,
    degree: i32,
    previous: Option<i32>,
) -> i32 {
    let class = plan.style.pitch(plan.tonic + 12, degree).rem_euclid(12);
    let ceiling = tune(plan, bar)
        .and_then(|line| {
            line.0
                .iter()
                .find(|&&(start, length, _)| start <= slot && slot < start + length)
                .map(|n| plan.style.pitch(plan.tonic + 24, n.2) - 3)
        })
        .unwrap_or(71)
        .min(71);
    let target = previous.unwrap_or(59);
    (45..=ceiling.max(56))
        .filter(|pitch| pitch.rem_euclid(12) == class)
        .min_by_key(|pitch| ((pitch - target).abs(), (pitch - 59).abs(), *pitch))
        .expect("an octave contains every pitch class")
}

#[cfg(test)]
mod tests;
