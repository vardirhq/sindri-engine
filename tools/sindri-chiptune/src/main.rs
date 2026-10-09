//! Standalone, dependency-free procedural chiptune experiment.
//! No engine, editor, audio device, or browser integration.
use std::{
    env,
    error::Error,
    f64::consts::TAU,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

const RATE: u32 = 44_100;
const BPM: f64 = 118.0;
mod gui;

#[derive(Clone, Debug)]
struct Settings {
    seed: u64,
    bpm: f64,
    key: i32,
    energy: f64,
    mood: Mood,
    output: PathBuf,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mood {
    Mysterious,
    Hopeful,
    Tense,
    Melancholic,
}
impl Settings {
    fn default() -> Self {
        Self {
            seed: 42,
            bpm: BPM,
            key: 2,
            energy: 0.45,
            mood: Mood::Mysterious,
            output: PathBuf::from("chiptune.wav"),
        }
    }
}

const BARS: usize = 32;
const STEPS: usize = BARS * 16;
const SCALE: [i32; 7] = [0, 2, 3, 5, 7, 8, 10]; // D natural minor

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Note {
    pitch: i32,
    start: usize,
    len: usize,
    velocity: u8,
}

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn range(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn degree(root: i32, index: i32) -> i32 {
    let oct = index.div_euclid(7);
    let idx = usize::try_from(index.rem_euclid(7)).expect("scale index");
    root + SCALE[idx] + oct * 12
}

fn section_energy(bar: usize) -> f64 {
    match bar / 8 {
        0 => 0.25,
        1 => 0.40,
        2 => 0.58,
        _ => 0.30,
    }
}

#[cfg(test)]
fn compose(seed: u64) -> [Vec<Note>; 3] {
    compose_with(&Settings {
        seed,
        ..Settings::default()
    })
}
fn compose_with(settings: &Settings) -> [Vec<Note>; 3] {
    let mut motif_rng = Rng::new(settings.seed ^ 0x11a4);
    let mut bass_rng = Rng::new(settings.seed ^ 0x22b5);
    let mut arp_rng = Rng::new(settings.seed ^ 0x33c6);
    let mut melody_rng = Rng::new(settings.seed ^ 0x44d7);
    let mut tracks: [Vec<Note>; 3] = std::array::from_fn(|_| Vec::new());
    // Relative chord roots. Mood changes the entire progression, not only velocity.
    let progressions: [[i32; 4]; 4] = [
        [0, -4, 3, -2],  // mysterious: i, VI, III, VII
        [0, 3, 5, -2],   // hopeful: i, III, iv, VII
        [0, -2, -4, -1], // tense: i, VII, VI, leading tone
        [0, -4, -2, 3],  // melancholic: i, VI, VII, III
    ];
    let mood_idx = match settings.mood {
        Mood::Mysterious => 0,
        Mood::Hopeful => 1,
        Mood::Tense => 2,
        Mood::Melancholic => 3,
    };
    let base = 36 + settings.key; // MIDI C2 + pitch class
    let roots = progressions[mood_idx].map(|n| base + n);
    let shapes: [[i32; 8]; 5] = [
        [0, 2, 1, -2, 0, 1, -1, -3],
        [0, 1, 3, 2, 0, -2, -1, 1],
        [0, -2, -1, 1, 2, 0, 1, -1],
        [0, 3, 2, 0, -1, 1, -2, 0],
        [0, 1, -1, 2, 3, 1, -2, 0],
    ];
    let motif = shapes[usize::try_from(motif_rng.range(shapes.len() as u64)).expect("shape")];
    let rhythm = usize::try_from(motif_rng.range(3)).expect("rhythm");
    let direction = if motif_rng.range(2) == 0 { 1 } else { -1 };
    let shift = i32::try_from(motif_rng.range(5)).expect("shift") - 2;
    let arp_order: [[usize; 4]; 4] = [[0, 1, 2, 1], [0, 2, 1, 2], [2, 1, 0, 1], [0, 1, 0, 2]];
    let arp_pattern = arp_order[usize::try_from(arp_rng.range(4)).expect("arp")];
    for bar in 0..BARS {
        let section = bar / 8;
        let root = roots[(bar / 2) % 4];
        let intensity = (section_energy(bar) * settings.energy / 0.45).clamp(0.05, 1.0);
        let bass_density = (0.25 + intensity * 0.72).min(0.97);
        for beat in 0..4 {
            if beat > 0 && bass_rng.range(100) >= (bass_density * 100.0) as u64 {
                continue;
            }
            let variation = if beat == 3 && bass_rng.range(4) == 0 {
                12
            } else if beat == 2 && bass_rng.range(3) == 0 {
                7
            } else {
                0
            };
            tracks[0].push(Note {
                pitch: root + variation,
                start: bar * 16 + beat * 4,
                len: if bass_rng.range(4) == 0 { 2 } else { 3 },
                velocity: (45.0 + intensity * 35.0) as u8,
            });
        }
        let arp_density = (0.20 + intensity * 0.65).min(0.9);
        let chord = match settings.mood {
            Mood::Hopeful => [0, 4, 7],
            Mood::Tense => [0, 3, 6],
            Mood::Mysterious | Mood::Melancholic => [0, 3, 7],
        };
        for i in 0..8 {
            if arp_rng.range(100) >= (arp_density * 100.0) as u64 {
                continue;
            }
            let degree = arp_pattern[(i + (bar % 4)) % 4];
            tracks[1].push(Note {
                pitch: root + 12 + chord[degree] + if section == 2 && i == 6 { 12 } else { 0 },
                start: bar * 16 + i * 2,
                len: if intensity < 0.4 { 2 } else { 1 },
                velocity: (20.0 + intensity * 36.0) as u8,
            });
        }
        let melody_density = (0.25 + intensity * 0.65).min(0.95);
        let phrase_variation = i32::try_from(melody_rng.range(3)).expect("variation") - 1;
        let phrase_shift = if section == 2 { 2 } else { 0 };
        for (i, step) in motif.iter().enumerate() {
            if section == 0 && bar % 4 < 2 {
                continue;
            }
            if melody_rng.range(100) >= (melody_density * 100.0) as u64 {
                continue;
            }
            let offset =
                direction * step + shift + phrase_shift + if i == 5 { phrase_variation } else { 0 };
            let pitch = degree(root + 24, offset);
            let slot = match rhythm {
                0 => i * 2,
                1 => [0, 2, 3, 6, 8, 10, 13, 14][i],
                _ => [0, 1, 4, 6, 8, 11, 12, 14][i],
            };
            tracks[2].push(Note {
                pitch,
                start: bar * 16 + slot,
                len: if i == 3 || i == 7 { 2 } else { 1 },
                velocity: (34.0 + intensity * 44.0) as u8,
            });
        }
    }
    tracks
}

fn freq(midi: i32) -> f64 {
    440.0 * 2_f64.powf((f64::from(midi) - 69.0) / 12.0)
}
fn pulse(time: f64, hz: f64, duty: f64) -> f64 {
    if (time * hz).fract() < duty {
        1.0
    } else {
        -1.0
    }
}
#[cfg(test)]
fn step_seconds() -> f64 {
    60.0 / BPM / 4.0
}

fn voice(notes: &[Note], t: f64, duty: f64, wave: bool, step: f64) -> f64 {
    let active = notes.iter().find(|n| {
        let start = f64::from(u32::try_from(n.start).expect("start")) * step;
        t >= start && t < start + f64::from(u32::try_from(n.len).expect("length")) * step
    });
    let Some(n) = active else {
        return 0.0;
    };
    let local = t - f64::from(u32::try_from(n.start).expect("start")) * step;
    let duration = f64::from(u32::try_from(n.len).expect("length")) * step;
    let attack = (local / 0.006).min(1.0);
    let release = ((duration - local) / 0.03).clamp(0.0, 1.0);
    let hz = freq(n.pitch);
    let sample = if wave {
        // Game Boy-style rounded triangle approximation
        (TAU * hz * t).sin() * 0.75 + (TAU * hz * t * 3.0).sin() * 0.08
    } else {
        pulse(t, hz, duty)
    };
    sample * attack * release * f64::from(n.velocity) / 127.0
}

fn noise(seed: u64, time: f64, step: f64, energy: f64) -> f64 {
    let tick = (time / step).floor() as usize;
    if tick >= STEPS || tick % 4 != 0 {
        return 0.0;
    }
    let bar = tick / 16;
    if bar < 8 || (bar >= 24 && tick % 16 != 0) {
        return 0.0;
    }
    let phase = time % step;
    if phase > 0.065 {
        return 0.0;
    }
    let sample_index = (time * f64::from(RATE)) as u64;
    let mut x = sample_index ^ seed ^ 0xabcd_1234;
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    let white = if x & 1 == 0 { -1.0 } else { 1.0 };
    let strength = if tick % 16 == 8 { 0.10 } else { 0.035 };
    white * strength * (energy / 0.45) * (1.0 - phase / 0.065)
}

fn render_samples(settings: &Settings, tracks: &[Vec<Note>; 3]) -> Vec<f32> {
    let step = 60.0 / settings.bpm / 4.0;
    let seconds = f64::from(u32::try_from(STEPS).expect("step count")) * step;
    let samples = (seconds * f64::from(RATE)).round() as u32;
    let mut audio = Vec::with_capacity(samples as usize);
    // Cache current notes by advancing cursors, not searching all notes per sample.
    let mut cursors = [0_usize; 3];
    for i in 0..samples {
        let t = f64::from(i) / f64::from(RATE);
        for (cursor, notes) in cursors.iter_mut().zip(tracks) {
            while *cursor < notes.len()
                && f64::from(
                    u32::try_from(notes[*cursor].start + notes[*cursor].len).expect("step"),
                ) * step
                    <= t
            {
                *cursor += 1;
            }
        }
        let channels = [
            (voice(&tracks[0][cursors[0]..], t, 0.5, true, step), 0.35),
            (voice(&tracks[1][cursors[1]..], t, 0.125, false, step), 0.22),
            (voice(&tracks[2][cursors[2]..], t, 0.25, false, step), 0.29),
        ];
        let mixed: f64 = channels.iter().map(|(v, gain)| v * gain).sum::<f64>()
            + noise(settings.seed, t, step, settings.energy);
        let sample = (mixed.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        audio.push(f32::from(sample) / 32767.0);
    }
    audio
}

fn render_with(settings: &Settings, tracks: &[Vec<Note>; 3]) -> Result<(), Box<dyn Error>> {
    let audio = render_samples(settings, tracks);
    let mut file = BufWriter::new(File::create(&settings.output)?);
    let bytes = u32::try_from(audio.len())?
        .checked_mul(2)
        .ok_or("WAV too long")?;
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + bytes).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16_u32.to_le_bytes())?;
    file.write_all(&1_u16.to_le_bytes())?;
    file.write_all(&1_u16.to_le_bytes())?;
    file.write_all(&RATE.to_le_bytes())?;
    file.write_all(&(RATE * 2).to_le_bytes())?;
    file.write_all(&2_u16.to_le_bytes())?;
    file.write_all(&16_u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&bytes.to_le_bytes())?;
    for sample in audio {
        let pcm = (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        file.write_all(&pcm.to_le_bytes())?;
    }
    file.flush()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut settings = Settings::default();
    let mut args = env::args().skip(1);
    if env::args().len() == 1 {
        return gui::run();
    }
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => settings.seed = args.next().ok_or("missing --seed value")?.parse()?,
            "--output" => {
                settings.output = PathBuf::from(args.next().ok_or("missing --output path")?)
            }
            "--bpm" => settings.bpm = args.next().ok_or("missing --bpm value")?.parse()?,
            "--energy" => settings.energy = args.next().ok_or("missing --energy value")?.parse()?,
            "--key" => settings.key = args.next().ok_or("missing --key value")?.parse()?,
            "--help" | "-h" => {
                println!(
                    "sindri-chiptune [--seed NUMBER] [--output PATH]\nGenerates 32 bars of D-minor exploratory chiptune at 118 BPM."
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
    if !(60.0..=220.0).contains(&settings.bpm)
        || !(0.0..=1.0).contains(&settings.energy)
        || !(0..=11).contains(&settings.key)
    {
        return Err("bpm must be 60..220, energy 0..1, key 0..11".into());
    }
    let tracks = compose_with(&settings);
    render_with(&settings, &tracks)?;
    println!(
        "Wrote {} (seed {}, {} BPM, 32 bars)",
        settings.output.display(),
        settings.seed,
        settings.bpm
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_generation() {
        assert_eq!(compose(123), compose(123));
        assert_ne!(compose(123), compose(456));
    }
    #[test]
    fn notes_are_valid() {
        let tracks = compose(42);
        assert!(
            tracks
                .iter()
                .all(|notes| notes.iter().all(|n| n.len > 0 && n.start + n.len <= STEPS))
        );
    }
    #[test]
    fn duration_is_expected() {
        assert!((f64::from(STEPS as u32) * step_seconds() - 65.0847).abs() < 0.001);
    }
    #[test]
    fn different_seeds_change_melody_and_accompaniment() {
        let a = compose_with(&Settings {
            seed: 42,
            ..Settings::default()
        });
        let b = compose_with(&Settings {
            seed: 43,
            ..Settings::default()
        });
        assert_ne!(a[2], b[2], "melody should change");
        assert_ne!(a[1], b[1], "arpeggio should change");
    }
    #[test]
    fn mood_and_energy_change_composition() {
        let default = Settings::default();
        let original = compose_with(&default);
        let hopeful = compose_with(&Settings {
            mood: Mood::Hopeful,
            ..default.clone()
        });
        let energetic = compose_with(&Settings {
            energy: 0.9,
            ..default
        });
        assert_ne!(original[0], hopeful[0], "mood should change harmony");
        assert_ne!(original[1], energetic[1], "energy should change density");
    }
}
