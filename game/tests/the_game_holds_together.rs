//! Causeway is the showcase, so what would leave it unopenable or unplayable
//! is checked here: a scene naming a file nobody ships, a script that stopped
//! compiling, a property set on a field that was renamed, a HUD that falls off
//! a phone.
//!
//! What happens once it is open -- clicking, walking, ending a turn -- is
//! `the_game_is_played`; whether the rules hold up over a whole crossing is
//! `a_crossing_is_played`.

// The board holds whole numbers the scripts wrote, so they compare exactly.
#![allow(clippy::float_cmp)]

use std::collections::BTreeSet;
use std::path::Path;

use sindri_core::SceneEntityId;
use sindri_runtime::ProjectRun;

fn open(size: [f32; 2]) -> ProjectRun {
    let mut run = ProjectRun::open(&sindri_causeway::project_dir(), size)
        .expect("the game opens from its folder");
    let report = run.step(1.0 / 60.0).expect("the first step runs");
    assert!(report.notes().is_empty(), "{:?}", report.notes());
    run
}

/// Every string in `text` that looks like an asset path with one of these
/// suffixes, quoted as JSON and Decay quote them.
fn references(text: &str, suffixes: &[&str]) -> BTreeSet<String> {
    text.split('"')
        .filter(|part| suffixes.iter().any(|suffix| part.ends_with(suffix)))
        .map(|part| part.split('#').next().unwrap_or(part).to_owned())
        .collect()
}

#[test]
fn every_file_the_game_names_is_shipped() {
    let assets = sindri_causeway::project_dir().join("assets");
    let mut named = BTreeSet::new();
    let mut texts =
        vec![std::fs::read_to_string(assets.join("causeway.scene")).expect("the scene")];
    for folder in ["prefabs", "scripts"] {
        for entry in std::fs::read_dir(assets.join(folder)).expect("the folder") {
            texts.push(std::fs::read_to_string(entry.expect("an entry").path()).expect("a file"));
        }
    }
    for text in &texts {
        named.extend(references(
            text,
            &[
                ".glb", ".png", ".ttf", ".wav", ".ogg", ".prefab", ".decay", ".weave",
            ],
        ));
    }
    assert!(named.len() > 20, "the game names its files: {named:?}");
    for reference in &named {
        assert!(
            assets.join(reference).is_file(),
            "{reference} is named but not shipped"
        );
    }
}

#[test]
fn every_sound_a_script_plays_is_listed_for_export() {
    let root = sindri_causeway::project_dir();
    let manifest = std::fs::read_to_string(root.join("sindri.toml")).expect("the manifest");
    let mut played = BTreeSet::new();
    for entry in std::fs::read_dir(root.join("assets/scripts")).expect("the scripts") {
        let text = std::fs::read_to_string(entry.expect("an entry").path()).expect("a script");
        played.extend(references(&text, &[".wav", ".ogg"]));
    }
    assert!(!played.is_empty(), "the scripts play sounds");
    for clip in played {
        assert!(
            manifest.contains(&format!("\"{clip}\"")),
            "{clip} is played by a script but not listed in [assets] include, so an export would drop it"
        );
    }
}

#[test]
fn the_game_opens_and_its_scripts_start() {
    let run = open([1280.0, 720.0]);
    assert_eq!(run.board("phase"), 0.0, "on its title");
    for id in ["battle", "world-camera", "sea", "island-0", "title"] {
        assert!(run.entity(id).is_some(), "the scene has {id}");
    }
}

/// Presses Enter for a frame.
fn enter(run: &mut ProjectRun) {
    run.key(sindri_platform::Key::Enter, true);
    run.step(1.0 / 60.0).expect("steps");
    run.key(sindri_platform::Key::Enter, false);
}

/// From the title into a battle's first turn: Enter takes the title's leave,
/// settling the camera into the battle's view, and Enter again sets out with
/// the three wardens the muster offers.
fn into_battle(run: &mut ProjectRun) {
    enter(run);
    for _ in 0..60 * 10 {
        run.step(1.0 / 60.0).expect("steps");
        if run.board("phase") == 7.0 {
            break;
        }
    }
    assert_eq!(run.board("phase"), 7.0, "the muster came");
    enter(run);
    for _ in 0..60 * 10 {
        run.step(1.0 / 60.0).expect("steps");
        if run.board("phase") == 2.0 {
            break;
        }
    }
    assert_eq!(run.board("phase"), 2.0, "the wardens' turn came");
}

/// Where a screen element was laid out, in pixels from the top left.
fn centre(run: &ProjectRun, id: &str) -> [f32; 2] {
    let entity = run
        .world
        .entity_for_source_id(&SceneEntityId::new(id).expect("an id"))
        .expect("the element exists");
    run.on_screen(entity)
        .unwrap_or_else(|| panic!("{id} is laid out"))
}

#[test]
fn the_hud_keeps_to_the_corners_on_a_desk_and_reflows_on_a_phone() {
    for size in [[1280.0, 720.0], [390.0, 844.0]] {
        let mut run = open(size);
        // Into a battle, so the HUD is showing.
        into_battle(&mut run);
        for (id, left, top) in [
            ("status", true, true),
            ("hearth", false, true),
            ("card", true, false),
            ("actions", false, false),
        ] {
            let [x, y] = centre(&run, id);
            assert!(
                (0.0..=size[0]).contains(&x) && (0.0..=size[1]).contains(&y),
                "{id} is on a {size:?} screen at ({x}, {y})"
            );
            if size[0] > size[1] {
                assert_eq!(x < size[0] / 2.0, left, "{id} keeps to its side");
                assert_eq!(y < size[1] / 2.0, top, "{id} keeps to its edge");
            } else {
                assert_eq!(y < size[1] / 2.0, top, "{id} keeps to its edge on a phone");
            }
        }
    }
}

#[test]
fn the_art_is_built_from_the_islands_the_rules_read() {
    // The art tool reads `islands.decay`; an island it was never run for has
    // no model, and the game would show the sea.
    let rules = std::fs::read_to_string(
        sindri_causeway::project_dir().join("assets/scripts/islands.decay"),
    )
    .expect("the islands");
    let count = rules.matches("IslandDef(").count();
    assert_eq!(count, 12, "twelve islands on the road");
    for file in [
        "saltmarsh",
        "fold",
        "quarry-rise",
        "lighthouse",
        "ashfall-shelf",
        "crucible",
        "glass-stair",
        "kilns-throat",
        "icebound-strand",
        "calving",
        "hoarwood",
        "white-barrow",
    ] {
        assert!(
            Path::new(&sindri_causeway::project_dir())
                .join(format!("assets/models/island-{file}.glb"))
                .is_file(),
            "island-{file}.glb is built"
        );
    }
}

#[test]
fn every_screen_between_islands_fits_a_desk_and_a_phone() {
    for size in [[1280.0, 720.0], [390.0, 844.0]] {
        for (screen, parts) in [
            (1.0, ["held-title", "held-button"]),
            (2.0, ["upgrade-title", "upgrade-card-3"]),
            (3.0, ["fallen-title", "fallen-button"]),
            (4.0, ["crossed-title", "crossed-button"]),
            (5.0, ["muster-card-5", "muster-button"]),
            (6.0, ["route-text", "route-card-1"]),
        ] {
            let mut run = open(size);
            into_battle(&mut run);
            run.set_board("preview_screen", screen);
            for _ in 0..90 {
                let report = run.step(1.0 / 60.0).expect("steps");
                assert!(report.notes().is_empty(), "{:?}", report.notes());
            }
            for id in parts {
                let [x, y] = centre(&run, id);
                assert!(
                    (0.0..=size[0]).contains(&x) && (0.0..=size[1]).contains(&y),
                    "screen {screen}'s {id} is on a {size:?} screen at ({x}, {y})"
                );
            }
        }
    }
}
