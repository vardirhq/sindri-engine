//! Melodies that sound written: a motif built on chord tones, stated, carried
//! onto the next chords, and closed by a cadence.
//!
//! The first composer drew each note's step at random, which a listener hears
//! as wandering. Here a strong beat always lands on a tone of the bar's chord,
//! the notes between move by step toward the next one, the line rises to a
//! peak and turns back, and a phrase ends by walking to its goal.
use super::{Groove, Rng};

/// A bar's rhythm: each note's sixteenth and length.
pub(super) type Rhythm = &'static [(usize, usize)];

/// A motif's rhythm for a song: a known cell half the time, otherwise one
/// built by rule. Either way the song keeps it, which is most of what makes a
/// tune recognizable, and across songs the rules give thousands of them.
pub(super) fn rhythm(rng: &mut Rng, groove: Groove) -> Vec<(usize, usize)> {
    let known = rhythms(groove);
    if rng.range(2) == 0 {
        let i = usize::try_from(rng.range(u64::try_from(known.len()).expect("few"))).expect("i");
        return known[i].to_vec();
    }
    // Every cell starts on the downbeat and lands on beat three; the rest are
    // drawn from the onsets the groove allows, the even ones far likelier.
    let (onsets, extra): (&[usize], u64) = match groove {
        Groove::Ballad | Groove::Drift | Groove::HalfTime => {
            (&[2, 3, 4, 6, 10, 11, 12, 14], 1 + rng.range(3))
        }
        Groove::Shuffle => (&[2, 4, 6, 10, 12, 14], 2 + rng.range(3)),
        Groove::Bounce => (&[2, 3, 4, 6, 10, 11, 12, 14], 2 + rng.range(4)),
        Groove::Drive => (&[1, 2, 3, 4, 5, 6, 10, 12, 13, 14], 3 + rng.range(4)),
    };
    let mut chosen = vec![0, 8];
    let mut tries = 0;
    while chosen.len() < 2 + usize::try_from(extra).expect("few") && tries < 64 {
        tries += 1;
        let slot = onsets
            [usize::try_from(rng.range(u64::try_from(onsets.len()).expect("few"))).expect("i")];
        let odd_rejected = slot % 2 == 1 && rng.range(3) != 0;
        if !chosen.contains(&slot) && !odd_rejected {
            chosen.push(slot);
        }
    }
    chosen.sort_unstable();
    chosen
        .iter()
        .enumerate()
        .map(|(i, &slot)| (slot, chosen.get(i + 1).copied().unwrap_or(16) - slot))
        .collect()
}

/// Rhythm cells that suit each groove, a short list of known-good ones.
fn rhythms(groove: Groove) -> &'static [Rhythm] {
    match groove {
        Groove::Drift => &[
            &[(0, 3), (4, 2), (6, 2), (8, 6), (14, 2)],
            &[(0, 6), (6, 2), (8, 4), (12, 4)],
        ],
        Groove::Bounce => &[
            &[(0, 2), (2, 2), (4, 4), (8, 2), (10, 2), (12, 4)],
            &[(0, 3), (3, 3), (6, 2), (8, 2), (10, 2), (12, 4)],
            &[(0, 2), (2, 1), (3, 3), (6, 2), (8, 4), (12, 4)],
        ],
        Groove::Drive => &[
            &[(0, 2), (2, 2), (4, 2), (6, 2), (8, 4), (12, 2), (14, 2)],
            &[(0, 3), (3, 3), (6, 2), (8, 3), (11, 3), (14, 2)],
        ],
        Groove::Ballad => &[
            &[(0, 6), (6, 2), (8, 8)],
            &[(0, 4), (4, 2), (6, 2), (8, 6), (14, 2)],
        ],
        Groove::Shuffle => &[
            &[(0, 4), (4, 2), (6, 2), (8, 4), (12, 4)],
            &[(0, 2), (2, 2), (4, 4), (8, 2), (10, 6)],
        ],
        Groove::HalfTime => &[
            &[(0, 6), (6, 4), (10, 6)],
            &[(0, 3), (3, 5), (8, 6), (14, 2)],
        ],
    }
}

/// How a phrase closes: fewer, longer notes, landing on the last.
pub(super) fn cadences(groove: Groove) -> &'static [Rhythm] {
    match groove {
        Groove::Ballad | Groove::Drift | Groove::HalfTime => {
            &[&[(0, 8), (8, 8)], &[(0, 4), (4, 4), (8, 8)]]
        }
        Groove::Bounce | Groove::Drive | Groove::Shuffle => {
            &[&[(0, 4), (4, 4), (8, 8)], &[(0, 2), (2, 2), (4, 4), (8, 8)]]
        }
    }
}

/// The final bar of a song: a step, then the tonic held with room after it.
pub(super) const ENDING: Rhythm = &[(0, 4), (4, 8)];

/// One bar of melody in scale degrees: start, length, degree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Bar(pub(super) Vec<(usize, usize, i32)>);

fn is_chord_tone(degree: i32, root: i32) -> bool {
    [0, 2, 4].contains(&(degree - root).rem_euclid(7))
}

/// The chord tone nearest `target`, preferring not to repeat `avoid`.
fn nearest_chord_tone(target: i32, root: i32, avoid: Option<i32>) -> i32 {
    (target - 4..=target + 4)
        .filter(|d| is_chord_tone(*d, root) && Some(*d) != avoid)
        .min_by_key(|d| ((d - target).abs(), *d))
        .expect("a chord tone lies within four scale steps")
}

/// The chord tone nearest `target` that is no more than a fourth from
/// `previous`, so a strong beat never forces a leap.
fn chord_tone_near(target: i32, root: i32, previous: i32, avoid: Option<i32>) -> i32 {
    (target - 5..=target + 5)
        .filter(|d| is_chord_tone(*d, root) && Some(*d) != avoid && (d - previous).abs() <= 3)
        .min_by_key(|d| ((d - target).abs(), *d))
        .unwrap_or_else(|| nearest_chord_tone(target, root, avoid))
}

/// Invents a one-bar motif on chord `root`, starting near `centre` and
/// heading in `direction` until halfway, then turning back: an arch.
pub(super) fn invent(
    rng: &mut Rng,
    rhythm: &[(usize, usize)],
    root: i32,
    centre: i32,
    direction: i32,
) -> Bar {
    let mut notes = Vec::new();
    let mut previous = nearest_chord_tone(centre, root, None);
    let mut heading = direction;
    for (i, (slot, length)) in rhythm.iter().enumerate() {
        if i == rhythm.len().div_ceil(2) {
            heading = -direction;
        }
        let degree = if i == 0 {
            previous
        } else if slot % 4 == 0 {
            // A strong beat: a chord tone, a third or so further along.
            chord_tone_near(previous + heading * 2, root, previous, Some(previous))
        } else {
            // A weak beat: a step, now and then a skip, in the same direction.
            previous + heading * if rng.range(4) == 0 { 2 } else { 1 }
        };
        // Keep the line in its register; leave the edges by turning round.
        let mut degree = degree.clamp(centre - 3, centre + 4);
        if slot % 4 == 0 && !is_chord_tone(degree, root) {
            degree = chord_tone_near(degree, root, previous, None);
        }
        if degree >= centre + 4 {
            heading = -1;
        } else if degree <= centre - 3 {
            heading = 1;
        }
        notes.push((*slot, *length, degree));
        previous = degree;
    }
    Bar(notes)
}

/// The same motif carried onto another chord: its shape kept, its strong
/// beats on the new chord's tones, and its first note placed to continue
/// smoothly from `previous`, the last note before it, when there is one.
pub(super) fn sequence(motif: &Bar, root: i32, previous: Option<i32>) -> Bar {
    let Some(&(_, _, first)) = motif.0.first() else {
        return motif.clone();
    };
    // Each chord tone near the motif's start is a way to carry it; take the
    // one closest to where the line already is, without drifting far from
    // the motif's own register.
    let start = (first - 5..=first + 5)
        .filter(|d| is_chord_tone(*d, root))
        .min_by_key(|d| previous.map_or(0, |p| (d - p).abs() * 2) + (d - first).abs())
        .expect("a chord tone lies within five scale steps");
    let shift = start - first;
    let mut last = start;
    Bar(motif
        .0
        .iter()
        .map(|&(slot, length, degree)| {
            let moved = degree + shift;
            let degree = if slot % 4 == 0 && !is_chord_tone(moved, root) {
                chord_tone_near(moved, root, last, None)
            } else {
                moved
            };
            last = degree;
            (slot, length, degree)
        })
        .collect())
}

/// A closing bar that walks by step from `from` to `goal` and holds it.
pub(super) fn cadence(rhythm: &[(usize, usize)], from: i32, goal: i32) -> Bar {
    // The goal's octave nearest where the line already is.
    let goal = (-2..=2)
        .map(|octave| goal + 7 * octave)
        .min_by_key(|g| (g - from).abs())
        .expect("octaves");
    let count = i32::try_from(rhythm.len()).expect("short rhythm");
    let side = if from >= goal { 1 } else { -1 };
    Bar(rhythm
        .iter()
        .enumerate()
        .map(|(i, &(slot, length))| {
            let steps_left = count - 1 - i32::try_from(i).expect("short rhythm");
            (slot, length, goal + side * steps_left)
        })
        .collect())
}

/// Where a phrase should land on its last chord: an open note for a
/// question, the tonic (or the chord tone nearest it) for an answer.
pub(super) fn goal(root: i32, answer: bool) -> i32 {
    if answer {
        nearest_chord_tone(0, root, None)
    } else {
        // Anything but the tonic, nearest the fifth: the tune is not done.
        (0..7)
            .filter(|d| is_chord_tone(*d, root) && *d != 0)
            .min_by_key(|d| (d - 4).abs())
            .unwrap_or(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_beats_land_on_chord_tones_and_steps_stay_small() {
        for seed in 0..64 {
            let mut rng = Rng::new(seed);
            for groove in [
                Groove::Drift,
                Groove::Bounce,
                Groove::Drive,
                Groove::Ballad,
                Groove::Shuffle,
                Groove::HalfTime,
            ] {
                let made = rhythm(&mut rng, groove);
                let cells: Vec<&[(usize, usize)]> = rhythms(groove)
                    .iter()
                    .copied()
                    .chain([made.as_slice()])
                    .collect();
                for rhythm in cells {
                    for root in 0..7 {
                        let bar = invent(&mut rng, rhythm, root, 4, 1);
                        for pair in bar.0.windows(2) {
                            assert!((pair[1].2 - pair[0].2).abs() <= 4, "a leap in {bar:?}");
                        }
                        for &(slot, _, degree) in &bar.0 {
                            if slot % 4 == 0 {
                                assert!(is_chord_tone(degree, root), "{bar:?} on {root}");
                            }
                        }
                        let moved = sequence(&bar, (root + 3) % 7, Some(4));
                        assert_eq!(moved.0.len(), bar.0.len());
                    }
                }
            }
        }
    }

    #[test]
    fn made_rhythms_fill_the_bar_from_the_downbeat_and_differ() {
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..200 {
            let mut rng = Rng::new(seed);
            let cell = rhythm(&mut rng, Groove::Bounce);
            assert_eq!(cell[0].0, 0);
            assert!(cell.iter().any(|n| n.0 == 8));
            assert_eq!(cell.iter().map(|n| n.1).sum::<usize>(), 16);
            seen.insert(cell);
        }
        assert!(seen.len() > 25, "only {} rhythms", seen.len());
    }

    #[test]
    fn a_cadence_walks_by_step_to_its_goal() {
        let bar = cadence(cadences(Groove::Bounce)[1], 6, 0);
        let degrees: Vec<i32> = bar.0.iter().map(|n| n.2).collect();
        assert_eq!(*degrees.last().expect("notes"), 7);
        for pair in degrees.windows(2) {
            assert_eq!((pair[1] - pair[0]).abs(), 1);
        }
    }

    #[test]
    fn a_question_stays_open_and_an_answer_comes_home() {
        assert_ne!(goal(4, false), 0);
        assert_eq!(goal(0, true), 0);
        assert!(is_chord_tone(goal(6, true), 6));
    }
}
