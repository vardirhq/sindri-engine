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
fn project(seed: u32) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("causeway-autoplay-{}-{seed}", std::process::id()));
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
          "properties": {{ "seed": {seed}.0 }}
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
    for _ in 0..20000 {
        let report = run.step(STEP).expect("the step runs");
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
