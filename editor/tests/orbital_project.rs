//! What the editor loads when it opens the acceptance project.
//!
//! The lesson `gather_project.rs` records, one layer down: every unit test
//! around script loading builds a scene for the occasion, and a scene built for
//! the occasion is one whose scripts are named directly by entities in it.
//! Orbital Last Stand is not that. Its director spawns enemies from prefabs it
//! names in Decay, its screens are switched off until something shows them, and
//! opening it in the editor produced two hundred errors -- no prefab loaded, no
//! script for the HUD -- while the exported build of the same project played
//! perfectly.
//!
//! So this opens the project the way the editor does and asks what arrived.
//! Nothing here needs a GPU or a window.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sindri_core::{ComponentSchemaRegistry, World};
use sindri_editor::native::scene_extractor;
use sindri_editor::scene_file::SceneFile;
use sindri_editor::scripts::SceneScripts;

const REQUIRED_PREFABS: [&str; 3] = [
    "prefabs/drifter.prefab",
    "prefabs/charger.prefab",
    "prefabs/splitter.prefab",
];

/// The acceptance project's scene, from this crate's own directory.
fn scene_path() -> PathBuf {
    game_scene("orbital-baked")
}

/// A game's main scene under `games/`, from this crate's own directory.
fn game_scene(game: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the editor crate sits in the workspace")
        .join("games")
        .join(game)
        .join("assets/orbital.scene")
}

/// Opens the scene and lets the loader settle, as an editor frame loop does.
///
/// Bounded by wall clock rather than by a fixed number of turns: the loader is
/// asynchronous, and a test that gave up after N frames would be measuring this
/// machine rather than the engine.
fn opened() -> (SceneScripts, World, ComponentSchemaRegistry) {
    opened_at(&scene_path())
}

fn opened_at(scene: &Path) -> (SceneScripts, World, ComponentSchemaRegistry) {
    let file = SceneFile::open(scene).expect("the acceptance project opens");
    let extractor = scene_extractor();
    let world = World::from_scene(file.document())
        .expect("the acceptance scene loads")
        .world;
    let components = extractor.components().clone();
    let mut scripts = SceneScripts::for_scene(Some(scene));

    // Kept turning after the scripts compile, because that is when the
    // prefabs first become askable: an ask, a load, and a compile each happen
    // on a different turn, and stopping at the first clean compile stops one
    // turn before the prefabs are visible.
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        scripts.request(&world, &components);
        scripts.poll();
        let settled = scripts.compile(&world, &components).is_empty()
            && REQUIRED_PREFABS
                .iter()
                .all(|prefab| scripts.has_prefab(prefab));
        if settled {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    (scripts, world, components)
}

#[test]
fn every_script_the_scene_names_is_loaded() {
    let (mut scripts, world, components) = opened();
    let failures = scripts.compile(&world, &components);
    assert!(
        failures.is_empty(),
        "the editor could not compile the project's scripts: {failures:#?}"
    );
}

/// Why the editor used to find none of them, and why one ask now finds all.
///
/// A prefab was only asked for once a compiled script's export named it, so
/// finding one took a load, a compile and a second ask, and the second ask sat
/// behind "has the world changed", which compiling never did. An enemy's
/// drop, named only inside another prefab, was never asked for at all.
///
/// Opening a project now asks for every script, prefab and profile in it, so
/// the one ask made when the scene opens is enough: polling alone brings every
/// prefab in, with no further request.
#[test]
fn one_ask_finds_every_prefab() {
    let file = SceneFile::open(scene_path()).expect("the acceptance project opens");
    let extractor = scene_extractor();
    let world = World::from_scene(file.document())
        .expect("the acceptance scene loads")
        .world;
    let components = extractor.components().clone();
    let mut scripts = SceneScripts::for_scene(Some(&scene_path()));

    scripts.request(&world, &components);
    let deadline = Instant::now() + Duration::from_secs(10);
    while scripts.loading() && Instant::now() < deadline {
        scripts.poll();
        std::thread::sleep(Duration::from_millis(5));
    }
    for prefab in REQUIRED_PREFABS {
        assert!(
            scripts.has_prefab(prefab),
            "one ask did not bring in {prefab}"
        );
    }
}

#[test]
fn every_prefab_the_scripts_spawn_is_loaded() {
    // The director spawns these by name. A prefab that is not loaded is an
    // enemy that never arrives, which is what the editor showed.
    let (scripts, _world, _components) = opened();
    for prefab in REQUIRED_PREFABS {
        assert!(
            scripts.has_prefab(prefab),
            "the editor never loaded {prefab}"
        );
    }
}

/// Orbital Baked's asteroids and enemies drew as the magenta missing checker
/// in the editor.
///
/// Their textures are named only by the prefabs a script spawns, and the
/// editor asked only for what the scene's own entities draw with. The
/// prefabs are spawned into a world of their own so their textures can be
/// asked for alongside the scene's, before anything spawns them.
#[test]
fn every_texture_a_spawned_enemy_draws_with_is_asked_for() {
    let (scripts, world, _components) = opened_at(&game_scene("orbital-baked"));
    let from_scene = sindri_scene::referenced_textures(&world);
    let from_prefabs = sindri_scene::referenced_textures(&sindri_editor::textures::prefab_world(
        scripts.prefabs(),
    ));
    for texture in [
        "textures/asteroid.png",
        "textures/drifter.png",
        "textures/charger.png",
    ] {
        assert!(
            !from_scene.contains(texture),
            "{texture} is named by the scene itself, so this would prove nothing"
        );
        assert!(
            from_prefabs.contains(texture),
            "{texture} is drawn by a spawned enemy but never asked for"
        );
    }
}
