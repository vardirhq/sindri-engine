use super::*;

fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0, |high, s| high.max(s.abs()))
}

#[test]
fn every_kind_is_deterministic_audible_and_bounded() {
    for kind in Kind::ALL {
        assert!((MAX_SECONDS - 3.0).abs() < f64::EPSILON);
        for seed in 0..24 {
            let sound = kind.generate(seed);
            assert_eq!(sound, kind.generate(seed), "{kind:?} seed {seed}");
            sound.check().expect("a generated effect is in range");
            let samples = sound.render();
            assert!(!samples.is_empty());
            assert!(
                samples.len() <= usize::try_from(RATE * 3 + 1).expect("limit"),
                "{kind:?} is too long"
            );
            assert!(peak(&samples) > 0.05, "{kind:?} seed {seed} is silent");
            assert!(samples.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
        }
    }
}

#[test]
fn seeds_and_kinds_give_different_sounds() {
    assert_ne!(Kind::Coin.generate(1), Kind::Coin.generate(2));
    assert_ne!(Kind::Coin.generate(1), Kind::Jump.generate(1));
}

#[test]
fn shaped_kinds_move_pitch_the_way_their_names_say() {
    for seed in 0..16 {
        let jump = Kind::Jump.generate(seed);
        assert!(jump.pitch(0.1) > jump.pitch(0.0), "a jump rises");
        let laser = Kind::Laser.generate(seed);
        assert!(laser.pitch(0.05) < laser.pitch(0.0), "a laser falls");
        let coin = Kind::Coin.generate(seed);
        assert!(
            coin.pitch(coin.arp_delay + 0.001) > coin.pitch(0.0),
            "a coin steps up"
        );
    }
}

#[test]
fn effects_end_quietly_without_a_click() {
    for kind in Kind::ALL {
        let samples = kind.generate(7).render();
        let tail = &samples[samples.len().saturating_sub(32)..];
        assert!(peak(tail) < 0.05, "{kind:?} ends on {}", peak(tail));
    }
}

#[test]
fn falling_cutoff_stops_the_sound_early() {
    let sound = Sfx {
        slide: -8.0,
        cutoff: 300.0,
        sustain: 1.0,
        decay: 1.0,
        frequency: 1_000.0,
        ..Sfx::default()
    };
    let samples = sound.render();
    assert!(samples.len() < RATE as usize / 2);
}

#[test]
fn mutation_stays_close_and_in_range() {
    let original = Kind::PowerUp.generate(3);
    let mutated = original.mutate(9);
    assert_ne!(original, mutated);
    mutated.check().expect("mutation is clamped");
    assert!((mutated.frequency / original.frequency - 1.0).abs() <= 0.081);
    assert_eq!(mutated.wave, original.wave);
}

#[test]
fn files_round_trip_and_refuse_bad_documents() {
    let file = SfxFile {
        format: FORMAT,
        kind: Kind::Laser,
        seed: 12,
        sound: Kind::Laser.generate(12),
    };
    assert_eq!(SfxFile::from_json(&file.to_json()), Ok(file.clone()));

    let mut wrong = file.clone();
    wrong.format = 99;
    assert!(SfxFile::from_json(&wrong.to_json()).is_err());

    let mut out_of_range = file;
    out_of_range.sound.frequency = 1.0e9;
    assert!(SfxFile::from_json(&out_of_range.to_json()).is_err());

    assert!(
        SfxFile::from_json("{\"format\":1,\"kind\":\"coin\",\"seed\":1,\"sound\":{\"bogus\":1}}")
            .is_err()
    );
}

#[test]
fn a_partial_sound_takes_defaults_for_what_it_omits() {
    let file = SfxFile::from_json(
        "{\"format\":1,\"kind\":\"blip\",\"seed\":0,\"sound\":{\"frequency\":880.0}}",
    )
    .expect("partial");
    assert!((file.sound.frequency - 880.0).abs() < f64::EPSILON);
    assert_eq!(file.sound.wave, Sfx::default().wave);
}

#[test]
fn kinds_parse_from_their_file_names() {
    for kind in Kind::ALL {
        assert_eq!(Kind::parse(kind.slug()), Some(kind));
    }
    assert_eq!(Kind::parse("powerup"), Some(Kind::PowerUp));
    assert_eq!(Kind::parse("kazoo"), None);
}

/// The platformer's sounds were made here: each committed `.sindrisfx` still
/// renders the WAV beside it, so reopening one in the Lab edits what ships.
#[test]
fn the_platformer_sounds_render_from_their_committed_sources() {
    let audio = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../games/lamplighter/assets/audio");
    for name in ["jump", "coin", "land", "goal"] {
        let text =
            std::fs::read_to_string(audio.join(format!("{name}.sindrisfx"))).expect("the source");
        let file = SfxFile::from_json(&text).expect("a valid source");
        let rendered: Vec<i16> = crate::lab::wav::decode(
            &crate::lab::wav::encode(&file.sound.render(), 1).expect("encodes"),
        )
        .expect("decodes");
        let committed = crate::lab::wav::decode(
            &std::fs::read(audio.join(format!("{name}.wav"))).expect("the WAV"),
        )
        .expect("a WAV this tool wrote");
        assert_eq!(rendered.len(), committed.len(), "{name} length");
        // A platform's libm may round a transcendental differently in the last
        // place, so allow one step of 16-bit quantization.
        let worst = rendered
            .iter()
            .zip(&committed)
            .map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs())
            .max()
            .unwrap_or(0);
        assert!(worst <= 1, "{name} differs by {worst}");
    }
}
