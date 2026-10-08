//! What the editor loads when it opens the Physics Playground.
//!
//! The playground's director spawns every loose thing from prefabs it names in
//! Decay, the probe and the debug overlay spawn their marks the same way, and
//! the shell prefab carries a script of its own. A prefab the editor never
//! loaded is a SPAWN button, a cannon or a DEBUG overlay that does nothing in
//! Play while the exported build works, so this opens the project the way the
//! editor does and asks what arrived. Nothing here needs a GPU or a window.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sindri_core::World;
use sindri_editor::native::scene_extractor;
use sindri_editor::scene_file::SceneFile;
use sindri_editor::scripts::SceneScripts;

const SPAWNED_PREFABS: [&str; 11] = [
    "prefabs/ball.prefab",
    "prefabs/crate.prefab",
    "prefabs/plank.prefab",
    "prefabs/anvil.prefab",
    "prefabs/bowling-ball.prefab",
    "prefabs/hand.prefab",
    "prefabs/shell.prefab",
    "prefabs/debug-rect.prefab",
    "prefabs/debug-ring.prefab",
    "prefabs/debug-bar.prefab",
    "prefabs/debug-dot.prefab",
];

fn scene_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the editor crate sits in the workspace")
        .join("examples/physics/assets/playground.scene")
}

#[test]
fn the_editor_compiles_every_script_and_loads_every_spawned_prefab() {
    let scene = scene_path();
    let file = SceneFile::open(&scene).expect("the playground opens");
    let extractor = scene_extractor();
    let world = World::from_scene(file.document())
        .expect("the playground scene loads")
        .world;
    let components = extractor.components().clone();
    let mut scripts = SceneScripts::for_scene(Some(&scene));

    // Bounded by wall clock: the loader is asynchronous, and asks, loads and
    // compiles each take a turn.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut failures = Vec::new();
    while Instant::now() < deadline {
        scripts.request(&world, &components);
        scripts.poll();
        failures = scripts.compile(&world, &components);
        if failures.is_empty() && SPAWNED_PREFABS.iter().all(|p| scripts.has_prefab(p)) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        failures.is_empty(),
        "the editor could not compile the playground's scripts: {failures:#?}"
    );
    for prefab in SPAWNED_PREFABS {
        assert!(
            scripts.has_prefab(prefab),
            "the editor never loaded {prefab}"
        );
    }
}
