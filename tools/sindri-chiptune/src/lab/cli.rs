//! Headless rendering: the same songs and effects as the Sound Lab, from a shell.
use super::{Mood, Settings, compose_with, project, render_samples, sfx, song::SongFile, wav};
use std::{error::Error, fs, path::PathBuf};

const HELP: &str = "\
Sindri Sound Lab: chiptune songs and game sound effects.

USAGE
  sindri-chiptune                         Open the Sound Lab window
  sindri-chiptune song [OPTIONS]          Compose and render a 32-bar song
  sindri-chiptune sfx <KIND> [OPTIONS]    Generate and render a sound effect

SONG OPTIONS
  --seed N  --mood M  --key 0-11  --bpm 60-220  --energy 0-1
  --open FILE.sindrisong                  Render a saved (and edited) song
  moods: mysterious, hopeful, tense, melancholic

SFX OPTIONS
  KIND: coin, jump, laser, explosion, power-up, hurt, blip, thud
  --seed N                                Variation of the kind (default 1)
  --open FILE.sindrisfx                   Render a saved effect instead of a kind

OUTPUT (both)
  --output FILE.wav                       Where the WAV goes
  --source FILE                           Also save the editable .sindrisong/.sindrisfx
  --project DIR --name NAME               Deliver into a Sindri project: writes
                                          assets/audio/NAME.wav and its source, and
                                          lists the clip in sindri.toml [assets]";

#[derive(Default)]
struct Output {
    wav: Option<PathBuf>,
    source: Option<PathBuf>,
    project: Option<PathBuf>,
    name: Option<String>,
}

pub(super) fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut args = args.into_iter().peekable();
    match args.peek().map(String::as_str) {
        Some("--help" | "-h" | "help") => {
            println!("{HELP}");
            Ok(())
        }
        Some("sfx") => {
            args.next();
            effect(args.collect())
        }
        Some("song") => {
            args.next();
            music(args.collect())
        }
        // The original interface: song options with no subcommand.
        _ => music(args.collect()),
    }
}

fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("missing {flag} value"))
}

/// Takes an output flag if `flag` is one, so both commands share them.
fn output_flag(
    flag: &str,
    args: &mut impl Iterator<Item = String>,
    out: &mut Output,
) -> Result<bool, String> {
    match flag {
        "--output" => out.wav = Some(value(args, flag)?.into()),
        "--source" => out.source = Some(value(args, flag)?.into()),
        "--project" => out.project = Some(value(args, flag)?.into()),
        "--name" => out.name = Some(value(args, flag)?),
        _ => return Ok(false),
    }
    Ok(true)
}

fn music(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut settings = Settings::default();
    let mut out = Output::default();
    let mut opened: Option<SongFile> = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if output_flag(&arg, &mut args, &mut out)? {
            continue;
        }
        match arg.as_str() {
            "--seed" => settings.seed = value(&mut args, &arg)?.parse()?,
            "--bpm" => settings.bpm = value(&mut args, &arg)?.parse()?,
            "--energy" => settings.energy = value(&mut args, &arg)?.parse()?,
            "--key" => settings.key = value(&mut args, &arg)?.parse()?,
            "--mood" => settings.mood = mood(&value(&mut args, &arg)?)?,
            "--open" => {
                let path = value(&mut args, &arg)?;
                let text = fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
                opened = Some(SongFile::from_json(&text).map_err(|e| format!("{path}: {e}"))?);
            }
            _ => return Err(format!("unknown argument: {arg} (try --help)").into()),
        }
    }
    if !(60.0..=220.0).contains(&settings.bpm)
        || !(0.0..=1.0).contains(&settings.energy)
        || !(0..=11).contains(&settings.key)
    {
        return Err("bpm must be 60..220, energy 0..1, key 0..11".into());
    }
    let song = opened.unwrap_or_else(|| SongFile::new(&settings, &compose_with(&settings)));
    let samples = render_samples(&song.settings(), &song.tracks);
    deliver(
        &out,
        "chiptune",
        &samples,
        "sindrisong",
        &song.to_json(),
        true,
    )?;
    println!(
        "  seed {}, {} BPM, 32 bars, {:.1} s",
        song.seed,
        song.bpm,
        seconds(&samples) / 2.0
    );
    Ok(())
}

fn effect(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut out = Output::default();
    let mut kind = None;
    let mut seed = 1;
    let mut opened: Option<sfx::SfxFile> = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if output_flag(&arg, &mut args, &mut out)? {
            continue;
        }
        match arg.as_str() {
            "--seed" => seed = value(&mut args, &arg)?.parse()?,
            "--open" => {
                let path = value(&mut args, &arg)?;
                let text = fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
                opened = Some(sfx::SfxFile::from_json(&text).map_err(|e| format!("{path}: {e}"))?);
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown argument: {flag} (try --help)").into());
            }
            name => {
                kind = Some(sfx::Kind::parse(name).ok_or_else(|| {
                    format!("unknown effect kind {name}: try coin, jump, laser, explosion, power-up, hurt, blip or thud")
                })?);
            }
        }
    }
    let file = match (opened, kind) {
        (Some(file), _) => file,
        (None, Some(kind)) => sfx::SfxFile {
            format: sfx::FORMAT,
            kind,
            seed,
            sound: kind.generate(seed),
        },
        (None, None) => {
            return Err("name an effect kind or --open a .sindrisfx (try --help)".into());
        }
    };
    let samples = file.sound.render();
    deliver(
        &out,
        file.kind.slug(),
        &samples,
        "sindrisfx",
        &file.to_json(),
        false,
    )?;
    println!(
        "  {} seed {}, {:.2} s",
        file.kind.name(),
        file.seed,
        seconds(&samples)
    );
    Ok(())
}

fn deliver(
    out: &Output,
    fallback: &str,
    samples: &[f32],
    extension: &str,
    source: &str,
    music: bool,
) -> Result<(), Box<dyn Error>> {
    let channels = if music { 2 } else { 1 };
    if let Some(root) = &out.project {
        let name = out.name.as_deref().unwrap_or(fallback);
        // A song goes in compressed; an effect is short enough that WAV costs
        // nothing and starts without decoding.
        let (audio, format) = if music {
            (super::ogg::encode(samples, 2)?, "ogg")
        } else {
            (wav::encode(samples, channels)?, "wav")
        };
        let placed = project::place(root, name, &audio, format, extension, source)?;
        println!(
            "Wrote {} and {}",
            placed.wav.display(),
            placed.source.display()
        );
        if placed.listed {
            println!("Listed {} in sindri.toml [assets] include", placed.clip);
        }
        println!(
            "Play it from Decay: {}",
            project::snippet(&placed.clip, music)
        );
    }
    let wav_path = out.wav.clone().or_else(|| {
        out.project
            .is_none()
            .then(|| format!("{fallback}.wav").into())
    });
    if let Some(path) = wav_path {
        if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("ogg")) {
            std::fs::write(&path, super::ogg::encode(samples, u8::try_from(channels)?)?)?;
        } else {
            wav::write(&path, samples, channels)?;
        }
        println!("Wrote {}", path.display());
    }
    if let Some(path) = &out.source {
        fs::write(path, source)?;
        println!("Wrote {}", path.display());
    }
    Ok(())
}

fn mood(name: &str) -> Result<Mood, String> {
    Ok(match name {
        "mysterious" => Mood::Mysterious,
        "hopeful" => Mood::Hopeful,
        "tense" => Mood::Tense,
        "melancholic" => Mood::Melancholic,
        _ => return Err("mood must be mysterious, hopeful, tense, or melancholic".into()),
    })
}

// A display duration; sample counts are far below f64's exact-integer range.
#[allow(clippy::cast_precision_loss)]
fn seconds(samples: &[f32]) -> f64 {
    samples.len() as f64 / f64::from(super::RATE)
}
