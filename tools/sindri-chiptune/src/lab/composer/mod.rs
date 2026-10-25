//! The composer: a song's harmony, phrases, bass riff and arpeggio, planned
//! once and repeated with purpose.
//!
//! Coherence comes from deliberate repetition. Each section loops four chords,
//! one a bar. The tune is a period: a four-bar question that ends open and a
//! four-bar answer that comes home, both built from one motif. The bass plays
//! one riff all song, the arpeggio one figure per section, and the Return
//! brings the theme back before a cadence ends it on the tonic.
mod melody;
mod variation;

use super::{BARS, Mood, Note, Rng, Settings};
use melody::Bar;
pub(super) use variation::vary_lead;

/// The rhythmic feel of a song: what the drums play and how the parts move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::lab) enum Groove {
    /// Sparse and floating.
    Drift,
    /// Straight, bright pop eighths.
    Bounce,
    /// Four on the floor, pushing.
    Drive,
    /// Slow and held.
    Ballad,
    /// Swung eighths, a skip in its step.
    Shuffle,
    /// The backbeat on three: heavy and wide.
    HalfTime,
}

/// A mood's chord loops for verse and rise, its dominant, and its grooves.
type Library = (
    &'static [[i32; 4]],
    &'static [[i32; 4]],
    i32,
    &'static [Groove],
);

#[derive(Clone, Copy)]
struct Style {
    scale: [i32; 7],
    /// The chord loop of the intro, theme and return, a bar each.
    verse: [i32; 4],
    /// The rise's contrasting loop.
    lift: [i32; 4],
    /// The chord that leads home in the second-last bar.
    dominant: i32,
    groove: Groove,
}

impl Style {
    fn new(mood: Mood, rng: &mut Rng) -> Self {
        fn pick<T: Copy>(rng: &mut Rng, options: &[T]) -> T {
            options[usize::try_from(rng.range(u64::try_from(options.len()).expect("few")))
                .expect("index")]
        }
        let scale = super::synth::scale(mood);
        // Loops listeners know, written as scale degrees of each mood's mode:
        // I-V-vi-IV and its relatives, i-VI-III-VII, modal vamps, Phrygian
        // half-step pulls. The rise gets a contrasting loop of its own.
        let (verses, lifts, dominant, grooves): Library = match mood {
            Mood::Hopeful => (
                &[
                    [0, 4, 5, 3],
                    [0, 5, 3, 4],
                    [0, 3, 0, 4],
                    [0, 3, 5, 4],
                    [5, 3, 0, 4],
                    [0, 2, 3, 4],
                    [0, 4, 3, 3],
                    [3, 0, 4, 5],
                    [0, 5, 1, 4],
                ],
                &[
                    [3, 4, 2, 5],
                    [5, 3, 0, 4],
                    [3, 4, 5, 5],
                    [1, 4, 0, 5],
                    [3, 3, 4, 4],
                ],
                4,
                &[Groove::Bounce, Groove::Shuffle, Groove::Drive],
            ),
            Mood::Melancholic => (
                &[
                    [0, 5, 2, 6],
                    [0, 3, 6, 2],
                    [0, 5, 3, 6],
                    [0, 6, 5, 6],
                    [0, 3, 4, 0],
                    [5, 6, 0, 0],
                    [0, 2, 3, 6],
                    [0, 5, 6, 4],
                ],
                &[[5, 6, 2, 0], [3, 6, 2, 5], [5, 3, 6, 6], [2, 6, 5, 4]],
                6,
                &[Groove::Ballad, Groove::HalfTime, Groove::Drift],
            ),
            Mood::Mysterious => (
                &[
                    [0, 3, 0, 6],
                    [0, 6, 3, 0],
                    [0, 5, 3, 3],
                    [0, 1, 0, 3],
                    [0, 6, 5, 6],
                    [0, 2, 3, 0],
                    [0, 3, 6, 4],
                ],
                &[[2, 3, 6, 0], [5, 3, 6, 6], [3, 6, 2, 5], [6, 5, 3, 4]],
                6,
                &[Groove::Drift, Groove::Shuffle, Groove::HalfTime],
            ),
            Mood::Tense => (
                &[
                    [0, 1, 0, 6],
                    [0, 5, 1, 0],
                    [0, 6, 5, 1],
                    [0, 0, 1, 1],
                    [0, 3, 1, 0],
                    [0, 5, 6, 1],
                    [0, 1, 3, 1],
                ],
                &[[5, 6, 1, 0], [3, 1, 5, 6], [5, 5, 1, 1], [6, 5, 3, 1]],
                1,
                &[Groove::Drive, Groove::HalfTime, Groove::Bounce],
            ),
        };
        Self {
            scale,
            verse: pick(rng, verses),
            lift: pick(rng, lifts),
            dominant,
            groove: pick(rng, grooves),
        }
    }

    fn pitch(self, tonic: i32, degree: i32) -> i32 {
        let index = usize::try_from(degree.rem_euclid(7)).expect("scale index");
        tonic + degree.div_euclid(7) * 12 + self.scale[index]
    }

    /// The chord, as a scale degree, under each bar.
    fn chord(self, bar: usize) -> i32 {
        match bar {
            30 => self.dominant,
            31 => 0,
            16..=23 => self.lift[bar % 4],
            _ => self.verse[bar % 4],
        }
    }
}

/// How hard each part of the form plays, from the energy setting.
fn intensity(bar: usize, energy: f64) -> f64 {
    let curve = match bar / 8 {
        0 => 0.48,
        1 => 0.82,
        2 => 1.0,
        _ => 0.7,
    };
    (curve * energy).clamp(0.02, 1.0)
}

// Velocity is explicitly clamped to the MIDI range before conversion.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn velocity(value: f64) -> u8 {
    value.round().clamp(1.0, 127.0) as u8
}

/// What the bass plays on a beat of its riff.
#[derive(Clone, Copy)]
enum Step {
    Root,
    Octave,
    Fifth,
    /// A scale step into the next chord's root at a phrase end.
    Lead,
}

/// The groove a song's seed and mood give it, for the drums to play.
pub(in crate::lab) fn groove(settings: &Settings) -> Groove {
    Style::new(settings.mood, &mut Rng::new(settings.seed ^ 0x8de4)).groove
}

/// Riffs per groove; a song picks one and plays it under every bar.
fn riffs(groove: Groove) -> &'static [&'static [(usize, usize, Step)]] {
    use Step::{Fifth, Lead, Octave, Root};
    match groove {
        Groove::Drift => &[
            &[(0, 6, Root), (6, 2, Fifth), (8, 6, Root), (14, 2, Lead)],
            &[(0, 8, Root), (8, 4, Fifth), (12, 4, Lead)],
        ],
        Groove::Bounce => &[
            &[
                (0, 3, Root),
                (3, 3, Root),
                (6, 2, Octave),
                (8, 3, Root),
                (11, 3, Fifth),
                (14, 2, Lead),
            ],
            &[
                (0, 2, Root),
                (2, 2, Octave),
                (4, 2, Root),
                (6, 2, Octave),
                (8, 2, Root),
                (10, 2, Octave),
                (12, 2, Fifth),
                (14, 2, Lead),
            ],
        ],
        Groove::Drive => &[
            &[
                (0, 2, Root),
                (2, 2, Root),
                (4, 2, Octave),
                (6, 2, Root),
                (8, 2, Root),
                (10, 2, Root),
                (12, 2, Octave),
                (14, 2, Lead),
            ],
            &[
                (0, 3, Root),
                (3, 3, Root),
                (6, 2, Fifth),
                (8, 3, Root),
                (11, 3, Root),
                (14, 2, Lead),
            ],
        ],
        Groove::Ballad => &[
            &[(0, 8, Root), (8, 6, Fifth), (14, 2, Lead)],
            &[(0, 6, Root), (6, 2, Fifth), (8, 8, Root)],
        ],
        Groove::Shuffle => &[
            &[
                (0, 4, Root),
                (4, 2, Octave),
                (6, 2, Root),
                (8, 4, Fifth),
                (12, 2, Octave),
                (14, 2, Lead),
            ],
            &[
                (0, 2, Root),
                (2, 2, Root),
                (4, 4, Octave),
                (8, 2, Root),
                (10, 2, Fifth),
                (12, 4, Lead),
            ],
        ],
        Groove::HalfTime => &[
            &[(0, 10, Root), (10, 4, Root), (14, 2, Lead)],
            &[(0, 3, Root), (3, 7, Root), (10, 2, Octave), (12, 4, Lead)],
        ],
    }
}

/// Everything decided once per song, so every bar can be written from it.
struct Plan {
    style: Style,
    tonic: i32,
    bass_octave: i32,
    riff_index: usize,
    riff: &'static [(usize, usize, Step)],
    theme: Bar,
    rise: Bar,
    close_index: usize,
    close: &'static [(usize, usize)],
    arp: usize,
}

fn plan(settings: &Settings) -> Plan {
    let mut rng = Rng::new(settings.seed ^ 0x8de4);
    let style = Style::new(settings.mood, &mut rng);
    let mut choose = |options: usize| {
        usize::try_from(rng.range(u64::try_from(options).expect("few"))).expect("index")
    };
    let cadences = melody::cadences(style.groove);
    let close_index = choose(cadences.len());
    let close = cadences[close_index];
    let riffs = riffs(style.groove);
    let riff_index = choose(riffs.len());
    let riff = riffs[riff_index];
    let arp = choose(4);
    let up = if choose(2) == 0 { 1 } else { -1 };
    let centre = i32::try_from(choose(3)).expect("small") + 2;
    let theme_rhythm = melody::rhythm(&mut rng, style.groove);
    let rise_rhythm = melody::rhythm(&mut rng, style.groove);
    let theme = melody::invent(&mut rng, &theme_rhythm, style.verse[0], centre, up);
    // The rise sits higher and moves the other way, so it answers the theme.
    let rise = melody::invent(&mut rng, &rise_rhythm, style.lift[0], centre + 1, -up);
    Plan {
        style,
        tonic: 36 + settings.key,
        bass_octave: 0,
        riff_index,
        riff,
        theme,
        rise,
        close_index,
        close,
        arp,
    }
}

/// The melody's bar `bar` in scale degrees: motif, sequence, or cadence.
fn tune(plan: &Plan, bar: usize) -> Option<Bar> {
    let style = plan.style;
    let chord = style.chord(bar);
    if bar < 4 {
        return None;
    }
    if bar == BARS - 1 {
        let previous = tune(plan, bar - 1)
            .and_then(|b| b.0.last().map(|n| n.2))
            .unwrap_or(0);
        return Some(melody::cadence(melody::ENDING, previous, 0));
    }
    let motif = if (16..24).contains(&bar) {
        &plan.rise
    } else {
        &plan.theme
    };
    let phrase_bar = bar % 4;
    let previous = || tune(plan, bar - 1).and_then(|b| b.0.last().map(|n| n.2));
    if phrase_bar == 0 {
        // A phrase opens on the motif itself, in its own register, so the
        // tune is recognizable each time it begins.
        return Some(melody::sequence(motif, chord, None));
    }
    if phrase_bar < 3 {
        return Some(melody::sequence(motif, chord, previous()));
    }
    // An answer closes every eight bars; the bars between ask a question.
    let answer = bar % 8 == 7;
    let previous = previous().unwrap_or(0);
    Some(melody::cadence(
        plan.close,
        previous,
        melody::goal(chord, answer),
    ))
}

fn lead(tracks: &mut [Vec<Note>; 3], plan: &Plan, bar: usize, power: f64) {
    let Some(line) = tune(plan, bar) else {
        return;
    };
    for (slot, length, degree) in line.0 {
        // The intro only hints at the tune: its strong beats, softly.
        if bar < 8 && slot % 4 != 0 {
            continue;
        }
        let accent = if slot % 4 == 0 { 8.0 } else { -4.0 };
        let hint = if bar < 8 { -14.0 } else { 0.0 };
        tracks[2].push(Note {
            pitch: plan.style.pitch(plan.tonic + 24, degree),
            start: bar * 16 + slot,
            len: length,
            velocity: velocity(52.0 + power * 34.0 + accent + hint),
        });
    }
}

fn bass(tracks: &mut [Vec<Note>; 3], plan: &Plan, bar: usize, power: f64) {
    let style = plan.style;
    let chord = style.chord(bar);
    let next = style.chord((bar + 1) % BARS);
    let root = style.pitch(plan.tonic, chord);
    if bar < 4 || bar == BARS - 1 {
        // The intro and the last bar hold the root.
        tracks[0].push(Note {
            pitch: root + plan.bass_octave,
            start: bar * 16,
            len: if bar == BARS - 1 { 12 } else { 16 },
            velocity: velocity(58.0 + power * 30.0),
        });
        return;
    }
    let phrase_end = bar % 4 == 3 && next != chord;
    for &(slot, length, step) in plan.riff {
        // Light energy keeps the downbeats and the lead-in only.
        if power < 0.35 && slot % 8 != 0 && !matches!(step, Step::Lead) {
            continue;
        }
        let pitch = match step {
            Step::Root => root,
            Step::Fifth => style.pitch(plan.tonic, chord + 4) - 12,
            Step::Lead if phrase_end => {
                let below = style.pitch(plan.tonic, next - 1);
                let above = style.pitch(plan.tonic, next + 1);
                if (below - root).abs() <= (above - root).abs() {
                    below
                } else {
                    above
                }
            }
            Step::Octave | Step::Lead => root + 12,
        };
        tracks[0].push(Note {
            pitch: pitch + plan.bass_octave,
            start: bar * 16 + slot,
            len: length,
            velocity: velocity(50.0 + power * 38.0 + if slot == 0 { 10.0 } else { 0.0 }),
        });
    }
}

/// The arpeggio: chord tones in one figure per section, from sparse in the
/// intro to a climbing run in the rise.
fn accompaniment(tracks: &mut [Vec<Note>; 3], plan: &Plan, bar: usize, power: f64) {
    let style = plan.style;
    let degree = style.chord(bar);
    let figures: [&[i32]; 4] = [
        &[0, 2, 4, 2, 0, 4, 2, 4],
        &[0, 4, 2, 4, 7, 4, 2, 4],
        &[0, 2, 4, 7, 4, 2, 0, 2],
        &[4, 2, 0, 2, 4, 7, 4, 2],
    ];
    let (gap, contour): (usize, &[i32]) = match bar / 8 {
        0 => (4, &[0, 4, 2, 4]),
        2 if power > 0.45 => (1, &[0, 2, 4, 7, 9, 7, 4, 2]),
        2 => (2, &[0, 2, 4, 7, 4, 2, 4, 7]),
        _ => (2, figures[plan.arp]),
    };
    for slot in (0..16).step_by(gap) {
        if bar < 4 && slot % 8 != 0 || bar >= 30 && slot >= 8 {
            continue;
        }
        let offset = contour[(slot / gap) % contour.len()];
        tracks[1].push(Note {
            pitch: style.pitch(plan.tonic + 12, degree + offset),
            start: bar * 16 + slot,
            len: if gap == 1 { 1 } else { gap - 1 },
            velocity: velocity(26.0 + power * 34.0 + if slot % 4 == 0 { 10.0 } else { 0.0 }),
        });
    }
}

pub(super) fn compose(settings: &Settings) -> [Vec<Note>; 3] {
    let plan = plan(settings);
    let mut tracks: [Vec<Note>; 3] = std::array::from_fn(|_| Vec::new());
    for bar in 0..BARS {
        let power = intensity(bar, settings.energy);
        bass(&mut tracks, &plan, bar, power);
        accompaniment(&mut tracks, &plan, bar, power);
        lead(&mut tracks, &plan, bar, power);
    }
    tracks
}

/// Compose one replacement part against the same harmony as the current seed.
/// `variation` changes only choices local to the requested part.
pub(super) fn compose_track(settings: &Settings, track: usize, variation: u64) -> Vec<Note> {
    let mut plan = plan(settings);
    let mut rng = Rng::new(settings.seed ^ variation ^ 0x7c31_09e5_4a26_b8df);
    let choose_other = |count: usize, current: usize| {
        (current
            + 1
            + usize::try_from(variation % u64::try_from(count - 1).expect("multiple choices"))
                .expect("bounded choice"))
            % count
    };

    match track {
        0 => {
            let choices = riffs(plan.style.groove);
            plan.riff_index = choose_other(choices.len(), plan.riff_index);
            plan.riff = choices[plan.riff_index];
            plan.bass_octave = match variation % 3 {
                0 => -12,
                1 => 0,
                _ => 12,
            };
        }
        1 => plan.arp = choose_other(4, plan.arp),
        2 => {
            let cadences = melody::cadences(plan.style.groove);
            plan.close_index = choose_other(cadences.len(), plan.close_index);
            plan.close = cadences[plan.close_index];
            let up = if rng.range(2) == 0 { 1 } else { -1 };
            let center = i32::try_from(rng.range(3)).expect("small") + 2;
            let theme_rhythm = melody::rhythm(&mut rng, plan.style.groove);
            let rise_rhythm = melody::rhythm(&mut rng, plan.style.groove);
            plan.theme = melody::invent(&mut rng, &theme_rhythm, plan.style.verse[0], center, up);
            plan.rise = melody::invent(&mut rng, &rise_rhythm, plan.style.lift[0], center + 1, -up);
        }
        _ => return Vec::new(),
    }

    let mut tracks: [Vec<Note>; 3] = std::array::from_fn(|_| Vec::new());
    for bar in 0..BARS {
        let power = intensity(bar, settings.energy);
        match track {
            0 => bass(&mut tracks, &plan, bar, power),
            1 => accompaniment(&mut tracks, &plan, bar, power),
            2 => lead(&mut tracks, &plan, bar, power),
            _ => unreachable!(),
        }
    }
    std::mem::take(&mut tracks[track])
}

#[cfg(test)]
mod tests;
