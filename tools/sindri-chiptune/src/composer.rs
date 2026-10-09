//! Standalone hierarchical chiptune composer: harmony, motifs, sections and grooves.
use super::{Mood, Note, Rng, Settings, BARS};

#[derive(Clone, Copy)]
enum Groove { Drift, Bounce, Drive, Ballad }

#[derive(Clone, Copy)]
struct Style {
    scale: [i32; 7],
    progression: [i32; 8],
    groove: Groove,
    max_step: i32,
    silence: u64,
}

impl Style {
    fn new(mood: Mood, rng: &mut Rng) -> Self {
        let alternate = rng.range(2) == 0;
        match mood {
            Mood::Mysterious => Self {
                scale: [0, 2, 3, 5, 7, 9, 10],
                progression: if alternate { [0, 5, 3, 6, 0, 4, 5, 0] } else { [0, 6, 3, 4, 5, 2, 4, 0] },
                groove: Groove::Drift, max_step: 2, silence: 34,
            },
            Mood::Hopeful => Self {
                scale: [0, 2, 4, 5, 7, 9, 11],
                progression: if alternate { [0, 4, 3, 5, 0, 3, 4, 0] } else { [0, 3, 5, 4, 3, 4, 0, 0] },
                groove: Groove::Bounce, max_step: 4, silence: 12,
            },
            Mood::Tense => Self {
                scale: [0, 1, 3, 5, 7, 8, 10],
                progression: if alternate { [0, 1, 6, 1, 0, 5, 1, 0] } else { [0, 6, 1, 4, 0, 1, 5, 0] },
                groove: Groove::Drive, max_step: 5, silence: 10,
            },
            Mood::Melancholic => Self {
                scale: [0, 2, 3, 5, 7, 8, 10],
                progression: if alternate { [0, 5, 2, 6, 3, 5, 4, 0] } else { [0, 3, 5, 2, 4, 6, 3, 0] },
                groove: Groove::Ballad, max_step: 2, silence: 42,
            },
        }
    }

    fn pitch(self, tonic: i32, degree: i32) -> i32 {
        let index = usize::try_from(degree.rem_euclid(7)).expect("scale index");
        tonic + degree.div_euclid(7) * 12 + self.scale[index]
    }
}

#[derive(Clone, Copy)]
struct Motif {
    degrees: [i32; 8],
    slots: [usize; 8],
    lengths: [usize; 8],
}

fn candidate(rng: &mut Rng, style: Style) -> Motif {
    let mut degrees = [0; 8];
    let mut lengths = [1; 8];
    let mut current = i32::try_from(rng.range(5)).expect("degree") - 2;
    let patterns = match style.groove {
        Groove::Drift => [[0, 2, 5, 7, 8, 10, 13, 15], [0, 3, 4, 6, 9, 11, 12, 14]],
        Groove::Bounce => [[0, 2, 4, 6, 8, 10, 12, 14], [0, 2, 3, 6, 8, 10, 11, 14]],
        Groove::Drive => [[0, 1, 2, 4, 8, 9, 10, 12], [0, 2, 4, 5, 8, 10, 12, 13]],
        Groove::Ballad => [[0, 3, 4, 7, 8, 11, 12, 15], [0, 2, 4, 6, 9, 11, 13, 15]],
    };
    let slots = patterns[usize::try_from(rng.range(2)).expect("pattern")];
    for (i, degree) in degrees.iter_mut().enumerate() {
        let width = u64::try_from(style.max_step * 2 + 1).expect("step width");
        let step = i32::try_from(rng.range(width)).expect("step") - style.max_step;
        current = (current + step).clamp(-6, 9);
        *degree = current;
        lengths[i] = match style.groove {
            Groove::Ballad => if i % 2 == 0 { 3 } else { 1 },
            Groove::Drift => if i % 3 == 0 { 2 } else { 1 },
            Groove::Bounce | Groove::Drive => 1,
        };
        lengths[i] = lengths[i].min(16 - slots[i]);
    }
    Motif { degrees, slots, lengths }
}

fn score(m: &Motif, style: Style) -> i32 {
    let mut result = 0;
    for pair in m.degrees.windows(2) {
        let distance = (pair[1] - pair[0]).abs();
        result += if distance <= 2 { 6 } else if distance <= style.max_step { 2 } else { -6 };
        if distance == 0 { result -= 2; }
    }
    // Prefer a phrase with direction, contrast, and a recognizable ending.
    result += (m.degrees[0] - m.degrees[7]).abs().min(4) * 2;
    result += if m.degrees[0] != m.degrees[3] { 4 } else { -4 };
    result
}

fn choose_motif(rng: &mut Rng, style: Style) -> Motif {
    let mut best = candidate(rng, style);
    let mut highest = score(&best, style);
    for _ in 0..63 {
        let trial = candidate(rng, style);
        let value = score(&trial, style);
        if value > highest {
            best = trial;
            highest = value;
        }
    }
    best
}

fn intensity(bar: usize, energy: f64) -> f64 {
    let curve = match bar / 8 { 0 => 0.48, 1 => 0.82, 2 => 1.0, _ => 0.54 };
    (curve * energy).clamp(0.02, 1.0)
}

fn bass(tracks: &mut [Vec<Note>; 3], rng: &mut Rng, style: Style, root: i32, bar: usize, power: f64) {
    let slots: &[usize] = match style.groove {
        Groove::Drift => &[0, 8],
        Groove::Bounce => &[0, 3, 8, 12],
        Groove::Drive => &[0, 4, 8, 12],
        Groove::Ballad => &[0, 10],
    };
    for (i, slot) in slots.iter().enumerate() {
        if i > 0 && rng.range(100) > (55.0 + power * 40.0) as u64 { continue; }
        let pitch = if i > 0 && rng.range(4) == 0 { root + 7 } else { root };
        tracks[0].push(Note {
            pitch, start: bar * 16 + *slot,
            len: match style.groove { Groove::Ballad | Groove::Drift => 6, Groove::Bounce => 2, Groove::Drive => 3 },
            velocity: (42.0 + power * 42.0) as u8,
        });
    }
}

fn accompaniment(tracks: &mut [Vec<Note>; 3], rng: &mut Rng, style: Style, root: i32, bar: usize, power: f64) {
    let intervals = match style.groove {
        Groove::Drift => [0, 7, 3, 10],
        Groove::Bounce => [0, 4, 7, 12],
        Groove::Drive => [0, 6, 7, 13],
        Groove::Ballad => [0, 3, 7, 10],
    };
    let (gap, duration) = match style.groove {
        Groove::Drift => (4, 3),
        Groove::Bounce => (2, 1),
        Groove::Drive => (2, 1),
        Groove::Ballad => (4, 3),
    };
    let phase = usize::try_from(rng.range(4)).expect("phase");
    for slot in (0..16).step_by(gap) {
        if rng.range(100) > (36.0 + power * 52.0) as u64 { continue; }
        let chord_index = (slot / gap + phase) % 4;
        tracks[1].push(Note {
            pitch: root + 12 + intervals[chord_index],
            start: bar * 16 + slot,
            len: duration,
            velocity: (19.0 + power * 38.0) as u8,
        });
    }
}

fn melody(tracks: &mut [Vec<Note>; 3], rng: &mut Rng, style: Style, motif: Motif, root: i32, bar: usize, power: f64) {
    let phrase = bar % 8;
    let section = bar / 8;
    if section == 0 && phrase < 2 { return; }
    for i in 0..8 {
        let rest = style.silence.saturating_sub((power * 24.0) as u64);
        if rng.range(100) < rest { continue; }
        let change = if phrase >= 4 && i == 5 { i32::try_from(rng.range(3)).expect("change") - 1 } else { 0 };
        let direction = if section == 2 && phrase >= 4 { -1 } else { 1 };
        let degree = direction * motif.degrees[i] + change + if section == 2 { 2 } else { 0 };
        tracks[2].push(Note {
            pitch: style.pitch(root + 24, degree),
            start: bar * 16 + motif.slots[i],
            len: motif.lengths[i],
            velocity: (35.0 + power * 45.0) as u8,
        });
    }
}

pub(super) fn compose(settings: &Settings) -> [Vec<Note>; 3] {
    let mut planner = Rng::new(settings.seed ^ 0x8de4);
    let mut bass_rng = Rng::new(settings.seed ^ 0xb45a);
    let mut arp_rng = Rng::new(settings.seed ^ 0xa22c);
    let mut lead_rng = Rng::new(settings.seed ^ 0xf452);
    let style = Style::new(settings.mood, &mut planner);
    let motif_a = choose_motif(&mut planner, style);
    let motif_b = choose_motif(&mut planner, style);
    let tonic = 36 + settings.key;
    let mut tracks: [Vec<Note>; 3] = std::array::from_fn(|_| Vec::new());
    for bar in 0..BARS {
        let power = intensity(bar, settings.energy);
        let degree = style.progression[(bar / 2) % 8];
        let root = style.pitch(tonic, degree);
        bass(&mut tracks, &mut bass_rng, style, root, bar, power);
        accompaniment(&mut tracks, &mut arp_rng, style, root, bar, power);
        let motif = if bar / 8 == 2 { motif_b } else { motif_a };
        melody(&mut tracks, &mut lead_rng, style, motif, root, bar, power);
    }
    tracks
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moods_have_distinct_melodic_and_harmonic_patterns() {
        let mut settings = Settings::default();
        let original = compose(&settings);
        settings.mood = Mood::Hopeful;
        let bright = compose(&settings);
        settings.mood = Mood::Tense;
        let tense = compose(&settings);
        assert_ne!(original[0], bright[0]);
        assert_ne!(bright[2], tense[2]);
    }
    #[test]
    fn seeds_change_actual_melodic_pitches() {
        let first = compose(&Settings::default());
        let second = compose(&Settings { seed: 931, ..Settings::default() });
        let a: Vec<_> = first[2].iter().map(|n| n.pitch).collect();
        let b: Vec<_> = second[2].iter().map(|n| n.pitch).collect();
        assert_ne!(a, b);
    }
}
