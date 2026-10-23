//! The rules, played through by a bot with nothing drawn.
//!
//! The project here is a small one built for the test: the game's own rules
//! and islands, and `autoplay.decay`, which plays every warden greedily and
//! lets the Hollow answer as they always do. What it proves is that the rules
//! run a whole crossing without a script failing, that an island can be held
//! by somebody trying, and that the same seed plays the same crossing.

// The board holds whole numbers the scripts wrote, so they compare exactly.
#![allow(clippy::float_cmp)]

use std::path::{Path, PathBuf};

use sindri_runtime::ProjectRun;

const STEP: f32 = 1.0 / 60.0;

fn game() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A project holding the game's rules and the bot, played with `seed`.
/// `CAUSEWAY_TRACE` has the bot print every board it plays, for a report.
fn project(seed: u32) -> PathBuf {
    project_from(seed, 0)
}

/// The same, with the bot starting on island `first`.
fn project_from(seed: u32, first: u32) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "causeway-autoplay-{}-{seed}-{first}",
        std::process::id()
    ));
    let scripts = root.join("assets/scripts");
    std::fs::create_dir_all(&scripts).expect("the project folder is made");
    std::fs::write(
        root.join("sindri.toml"),
        "format_version = 1\n\n[project]\nname = \"Autoplay\"\nmain_scene = \"assets/autoplay.scene\"\n",
    )
    .expect("the manifest writes");
    for script in ["rules.decay", "islands.decay"] {
        std::fs::copy(
            game().join("assets/scripts").join(script),
            scripts.join(script),
        )
        .expect("the game's script copies");
    }
    std::fs::copy(
        game().join("tests/autoplay.decay"),
        scripts.join("autoplay.decay"),
    )
    .expect("the bot copies");
    let tracing = std::env::var("CAUSEWAY_TRACE").is_ok() && seed == 1;
    let scene = format!(
        r#"{{
  "format_version": 10,
  "metadata": {{ "name": "Autoplay" }},
  "entities": [
    {{
      "id": "autoplay",
      "name": "Autoplay",
      "components": {{
        "sindri.script": {{
          "source": "scripts/autoplay.decay",
          "script": "Autoplay",
          "properties": {{ "seed": {seed}.0, "first": {first}.0, "tracing": {tracing} }}
        }}
      }}
    }}
  ]
}}"#
    );
    std::fs::write(root.join("assets/autoplay.scene"), scene).expect("the scene writes");
    root
}

/// Plays until the bot has finished or the step limit runs out, failing on
/// any script error.
fn play(root: &Path) -> ProjectRun {
    let mut run = ProjectRun::open(root, [1280.0, 720.0]).expect("the project opens");
    for _ in 0..40000 {
        let report = run.step(STEP).expect("the step runs");
        if std::env::var("CAUSEWAY_TRACE").is_ok() {
            for line in &report.scripts.printed {
                println!("{}", line.message);
            }
        }
        let notes = report.notes();
        assert!(notes.is_empty(), "a step went wrong: {notes:?}");
        if run.board("crossed") > 0.5 || run.board("lost_on") > 0.5 {
            break;
        }
    }
    run
}

#[test]
fn a_bot_plays_a_whole_crossing_through_the_rules() {
    let run = play(&project(7));
    let lost_on = run.board("lost_on");
    let crossed = run.board("crossed");
    println!(
        "crossed {crossed}, lost on island {lost_on}, hearth {}, rounds {}",
        run.board("hearth"),
        run.board("rounds_played")
    );
    assert!(
        crossed > 0.5 || lost_on > 0.5,
        "the crossing ends one way or the other"
    );
    assert!(
        run.board("won_0") > 0.5,
        "the first island is held by a player who is trying"
    );
}

#[test]
fn a_seed_plays_the_same_crossing_every_time() {
    let first = play(&project(11));
    let second = play(&project(11));
    for key in ["hearth", "rounds_played", "lost_on", "crossed"] {
        assert_eq!(first.board(key), second.board(key), "{key} differs");
    }
}

/// How hard each island is for the bot, over many seeds: the Hearth left after
/// each island it held, or where it fell. Not a pass or a fail but the numbers
/// to tune islands and waves against; the bot plays greedily and takes no
/// boons, so a person does better.
///
/// `CAUSEWAY_FIRST` starts every run on that island with a fresh Hearth, so a
/// later world can be tuned on its own.
///
/// ```bash
/// cargo test --release -p sindri-causeway --test a_crossing_is_played -- --ignored --nocapture
/// CAUSEWAY_FIRST=4 cargo test --release -p sindri-causeway --test a_crossing_is_played -- --ignored --nocapture
/// ```
#[test]
#[ignore = "a report for tuning, slow without --release"]
fn how_hard_each_island_is() {
    let first: u32 = std::env::var("CAUSEWAY_FIRST")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let (mut crossed, mut lost) = (0, [0; 12]);
    let names = [
        "Bulwark",
        "Mason",
        "Lamplighter",
        "Tidecaller",
        "Harpooner",
        "Shieldback",
    ];
    let mut with = [(0, 0); 6];
    for seed in 1..=24u32 {
        let run = play(&project_from(seed, first));
        let hearths: Vec<f32> = (0..12).map(|i| run.board(&format!("hearth_{i}"))).collect();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let party: Vec<usize> = (0..4)
            .map(|i| run.board(&format!("party_{i}")) as usize)
            .filter(|&hero| hero > 0)
            .map(|hero| hero - 1)
            .collect();
        println!(
            "seed {seed:2}: {:?}; Hearth after each island {hearths:?}, lost on {}",
            party.iter().map(|&hero| names[hero]).collect::<Vec<_>>(),
            run.board("lost_on")
        );
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let on = run.board("lost_on") as usize;
        for &hero in &party {
            with[hero].1 += 1;
            if run.board("crossed") > 0.5 {
                with[hero].0 += 1;
            }
        }
        // Only the islands this run's road went through.
        for i in first..12 {
            if run.board(&format!("won_{i}")) < 0.5 && on != i as usize + 1 {
                continue;
            }
            let k = |cause: &str| run.board(&format!("{cause}_{i}"));
            println!(
                "  island {i}: Hearth lost to strikes {}, vents {}, the level {}; wardens' health {}, {}, {}",
                k("dmg_strike"),
                k("dmg_vent"),
                k("dmg_level"),
                k("wdmg_strike"),
                k("wdmg_vent"),
                k("wdmg_level")
            );
        }
        if run.board("crossed") > 0.5 {
            crossed += 1;
        }
        if on > 0 {
            lost[on - 1] += 1;
        }
    }
    println!("crossed {crossed} of 24; fell on each island {lost:?}");
    for (hero, (won, played)) in with.iter().enumerate() {
        println!("  {}: crossed {won} of {played}", names[hero]);
    }
    // Keep the fixtures from piling up in the temp folder between reports.
    for seed in 1..=24u32 {
        let _ = std::fs::remove_dir_all(project_from(seed, first));
    }
}
