//! Standalone, dependency-free procedural chiptune experiment.
//! No engine, editor, audio device, or browser integration.
use std::{
    env,
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

const RATE: u32 = 44_100;
const BPM: f64 = 118.0;
mod composer;
mod gui;
mod instruments;
mod live;

#[derive(Clone, Debug)]
struct Settings {
    seed: u64,
    bpm: f64,
    key: i32,
    energy: f64,
    mood: Mood,
    output: PathBuf,
    rack: instruments::Rack,
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
            rack: instruments::Rack::default(),
        }
    }
}

const BARS: usize = 32;
const STEPS: usize = BARS * 16;
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

#[cfg(test)]
fn compose(seed: u64) -> [Vec<Note>; 3] {
    compose_with(&Settings {
        seed,
        ..Settings::default()
    })
}
fn compose_with(settings: &Settings) -> [Vec<Note>; 3] {
    composer::compose(settings)
}

fn freq(midi: i32) -> f64 {
    440.0 * 2_f64.powf((f64::from(midi) - 69.0) / 12.0)
}
#[cfg(test)]
fn step_seconds() -> f64 {
    60.0 / BPM / 4.0
}

fn voice(notes: &[Note], t: f64, step: f64, sound: instruments::TrackSound) -> f64 {
    let active = notes.iter().find(|n| {
        let start = f64::from(u32::try_from(n.start).expect("start")) * step;
        t >= start && t < start + f64::from(u32::try_from(n.len).expect("length")) * step
    });
    let Some(note) = active else {
        return 0.0;
    };
    let local = t - f64::from(u32::try_from(note.start).expect("start")) * step;
    let duration = f64::from(u32::try_from(note.len).expect("length")) * step;
    let hz = freq(note.pitch + sound.octave * 12);
    sound.preset.sample(t * hz, local, duration) * f64::from(note.velocity) / 127.0
}

fn noise(
    seed: u64,
    time: f64,
    step: f64,
    energy: f64,
    mood: Mood,
    preset: instruments::Preset,
) -> f64 {
    let tick = (time / step).floor() as usize;
    if tick >= STEPS {
        return 0.0;
    }
    let hit = match mood {
        Mood::Mysterious => tick % 8 == 0,
        Mood::Hopeful => tick % 4 == 0,
        Mood::Tense => tick % 2 == 0,
        Mood::Melancholic => tick % 16 == 8,
    };
    if !hit || (tick / 16 < 8 && matches!(mood, Mood::Mysterious | Mood::Melancholic)) {
        return 0.0;
    }
    let phase = time % step;
    let sample_index = (time * f64::from(RATE)) as u64;
    let mut x = sample_index ^ seed ^ 0xabcd_1234;
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    let white = if x & 1 == 0 { -1.0 } else { 1.0 };
    preset.noise(white, phase) * (energy / 0.45).min(1.5)
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
        let mut mixed = 0.0;
        for channel in 0..3 {
            if settings.rack.audible(channel) {
                let sound = settings.rack.tracks[channel];
                mixed += voice(&tracks[channel][cursors[channel]..], t, step, sound)
                    * f64::from(sound.volume)
                    * 0.45;
            }
        }
        if settings.rack.audible(3) {
            let sound = settings.rack.tracks[3];
            mixed += noise(
                settings.seed,
                t,
                step,
                settings.energy,
                settings.mood,
                sound.preset,
            ) * f64::from(sound.volume)
                * 0.55;
        }
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
            "--mood" => {
                settings.mood = match args.next().ok_or("missing --mood value")?.as_str() {
                    "mysterious" => Mood::Mysterious,
                    "hopeful" => Mood::Hopeful,
                    "tense" => Mood::Tense,
                    "melancholic" => Mood::Melancholic,
                    _ => {
                        return Err(
                            "mood must be mysterious, hopeful, tense, or melancholic".into()
                        );
                    }
                }
            }
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
