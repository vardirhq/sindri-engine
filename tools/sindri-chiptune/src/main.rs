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

fn compose(seed: u64) -> [Vec<Note>; 3] {
    let mut rng = Rng::new(seed);
    let mut tracks: [Vec<Note>; 3] = std::array::from_fn(|_| Vec::new());
    // Dm, Bb, F, C. Each two bars. Chord roots as MIDI pitches.
    let roots = [38, 34, 41, 36];
    let motif = [0, 2, 1, -3, 0, 1, -1, -3];
    for bar in 0..BARS {
        let root = roots[(bar / 2) % 4];
        let section = bar / 8;
        let energy = section_energy(bar);
        // bass: strong beat roots, occasional fifth, with explicit rests
        for beat in 0..4 {
            if beat == 3 && section == 0 {
                continue;
            }
            let pitch = if beat == 2 && section > 0 {
                root + 7
            } else {
                root
            };
            tracks[0].push(Note {
                pitch,
                start: bar * 16 + beat * 4,
                len: 3,
                velocity: 58,
            });
        }
        // Harmony: sparse eighth-note broken chords, not random pitches.
        let intervals = [0, 7, 12, 15, 12, 7, 3, 7];
        for i in 0..8 {
            if section == 0 && i % 2 != 0 {
                continue;
            }
            if section == 3 && i >= 6 {
                continue;
            }
            tracks[1].push(Note {
                pitch: root + 12 + intervals[i],
                start: bar * 16 + i * 2,
                len: 1,
                velocity: 20 + (energy * 24.0) as u8,
            });
        }
        // Two-bar motif, rest/response phrasing, transposed with the harmony.
        if section == 0 && bar % 4 < 2 {
            continue;
        }
        for (i, offset) in motif.iter().enumerate() {
            if i == 3 || i == 7 {
                continue;
            } // space between questions and answers
            if section == 3 && i >= 4 {
                continue;
            }
            let variation = if bar % 8 >= 4 && i == 5 { 1 } else { 0 };
            let contour = offset + variation;
            let pitch = degree(root + 24, contour);
            let start = bar * 16 + i * 2;
            let jitter = if rng.range(12) == 0 { 1 } else { 0 };
            tracks[2].push(Note {
                pitch,
                start,
                len: if i == 2 || i == 6 { 3 } else { 2 },
                velocity: 35 + (energy * 28.0) as u8 + jitter,
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
fn step_seconds() -> f64 {
    60.0 / BPM / 4.0
}

fn voice(notes: &[Note], t: f64, duty: f64, wave: bool) -> f64 {
    let step = step_seconds();
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

fn noise(seed: u64, time: f64) -> f64 {
    let tick = (time / step_seconds()).floor() as usize;
    if tick >= STEPS || tick % 4 != 0 {
        return 0.0;
    }
    let bar = tick / 16;
    if bar < 8 || (bar >= 24 && tick % 16 != 0) {
        return 0.0;
    }
    let phase = time % step_seconds();
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
    white * strength * (1.0 - phase / 0.065)
}

fn render(path: &PathBuf, seed: u64, tracks: &[Vec<Note>; 3]) -> Result<(), Box<dyn Error>> {
    let seconds = f64::from(u32::try_from(STEPS).expect("step count")) * step_seconds();
    let samples = (seconds * f64::from(RATE)).round() as u32;
    let mut file = BufWriter::new(File::create(path)?);
    let bytes = samples.checked_mul(2).ok_or("WAV too long")?;
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
    // Cache current notes by advancing cursors, not searching all notes per sample.
    let mut cursors = [0_usize; 3];
    for i in 0..samples {
        let t = f64::from(i) / f64::from(RATE);
        for (cursor, notes) in cursors.iter_mut().zip(tracks) {
            while *cursor < notes.len()
                && f64::from(
                    u32::try_from(notes[*cursor].start + notes[*cursor].len).expect("step"),
                ) * step_seconds()
                    <= t
            {
                *cursor += 1;
            }
        }
        let channels = [
            (voice(&tracks[0][cursors[0]..], t, 0.5, true), 0.35),
            (voice(&tracks[1][cursors[1]..], t, 0.125, false), 0.22),
            (voice(&tracks[2][cursors[2]..], t, 0.25, false), 0.29),
        ];
        let mixed: f64 = channels.iter().map(|(v, gain)| v * gain).sum::<f64>() + noise(seed, t);
        let sample = (mixed.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        file.write_all(&sample.to_le_bytes())?;
    }
    file.flush()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut seed = 42_u64;
    let mut output = PathBuf::from("chiptune.wav");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => seed = args.next().ok_or("missing --seed value")?.parse()?,
            "--output" => output = PathBuf::from(args.next().ok_or("missing --output path")?),
            "--help" | "-h" => {
                println!(
                    "sindri-chiptune [--seed NUMBER] [--output PATH]\nGenerates 32 bars of D-minor exploratory chiptune at 118 BPM."
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
    let tracks = compose(seed);
    render(&output, seed, &tracks)?;
    println!("Wrote {} (seed {seed}, 118 BPM, 32 bars)", output.display());
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
}
