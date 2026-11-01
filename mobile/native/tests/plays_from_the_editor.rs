//! Slice step 4 without a phone: Play from the editor runs Lamplighter's real
//! session, the stick walks Wick, Pause holds the game, and Stop leaves the
//! editor's scene as it was.

use std::path::{Path, PathBuf};

use sindri_core::{EntityId, SceneEntityId, World};
use sindri_mobile::{Editing, OpenedScene, Playing};

/// The Pixel 8's screen, held upright.
const SIZE: [f32; 2] = [1080.0, 2400.0];
const FRAME: f32 = 1.0 / 60.0;

fn lamplighter() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter")
}

fn wick(world: &World) -> EntityId {
    world
        .entity_for_source_id(&SceneEntityId::new("wick".to_owned()).expect("a valid ID"))
        .expect("the field has Wick")
}

fn at(world: &World, entity: EntityId) -> [f32; 2] {
    let position = world
        .get(entity)
        .and_then(|data| data.transform_3d)
        .expect("placed")
        .position;
    [position[0], position[1]]
}

/// Runs `frames` frames of a sixtieth of a second.
fn run_for(playing: &mut Playing, frames: u32) {
    for _ in 0..frames {
        playing.advance(FRAME).expect("the game steps");
    }
}

#[test]
fn the_stick_walks_wick_in_the_running_game_and_the_editor_is_untouched() {
    let opened = OpenedScene::open(&lamplighter(), "world.scene").expect("the field opens");
    let mut playing = Playing::start(&lamplighter(), "world.scene", SIZE).expect("the game starts");
    // Settle first: a game's first frames set it up.
    run_for(&mut playing, 30);
    let world = &playing.run().world;
    let start = at(world, wick(world));

    playing.stick(Some([1.0, 0.0]));
    run_for(&mut playing, 60);
    let world = &playing.run().world;
    let walked = at(world, wick(world));
    assert!(
        walked[0] > start[0] + 0.5,
        "Wick walks right: {start:?} to {walked:?}"
    );

    playing.stick(None);
    run_for(&mut playing, 30);
    let world = &playing.run().world;
    let stopped = at(world, wick(world));
    run_for(&mut playing, 30);
    let world = &playing.run().world;
    let still = at(world, wick(world));
    assert!(
        (still[0] - stopped[0]).abs() < 0.05,
        "let go, Wick stops: {stopped:?} to {still:?}"
    );

    assert_eq!(
        at(&opened.world, wick(&opened.world)).map(f32::to_bits),
        {
            let fresh = OpenedScene::open(&lamplighter(), "world.scene").expect("the field opens");
            at(&fresh.world, wick(&fresh.world)).map(f32::to_bits)
        },
        "the editor's world is not the game's"
    );
}

#[test]
fn pause_holds_the_game_and_step_moves_it_one_frame() {
    let mut playing = Playing::start(&lamplighter(), "world.scene", SIZE).expect("the game starts");
    run_for(&mut playing, 15);
    playing.set_paused(true);
    let seconds = playing.seconds();
    run_for(&mut playing, 30);
    assert!(
        (playing.seconds() - seconds).abs() < f32::EPSILON,
        "paused, the clock holds"
    );
    playing.step_once().expect("one step");
    assert!(
        (playing.seconds() - seconds - FRAME).abs() < 1e-4,
        "a step is one frame"
    );
    playing.set_paused(false);
    run_for(&mut playing, 15);
    assert!(playing.seconds() > seconds + 0.2);
}

#[test]
fn a_saved_edit_is_what_plays() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let root = dir.path().join("lamplighter");
    copy_tree(&lamplighter().join("assets"), &root.join("assets"));
    std::fs::copy(lamplighter().join("sindri.toml"), root.join("sindri.toml"))
        .expect("the manifest");

    let mut opened = OpenedScene::open(&root, "world.scene").expect("the copy opens");
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    let wick_id = wick(&opened.world);
    editing.selected = Some(wick_id);
    let from = at(&opened.world, wick_id);
    editing
        .set_place(
            &mut opened.world,
            sindri_mobile::Place::Across(from[0] + 3.0),
            false,
        )
        .expect("the move applies");
    opened.save().expect("the edit saves");

    let playing = Playing::start(&root, "world.scene", SIZE).expect("the game starts");
    let world = &playing.run().world;
    let starts = at(world, wick(world));
    assert!(
        (starts[0] - (from[0] + 3.0)).abs() < 1e-4,
        "Wick starts where he was put: {starts:?}"
    );
}

fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).expect("a folder in the copy");
    for entry in std::fs::read_dir(source).expect("the folder reads") {
        let entry = entry.expect("an entry");
        let into = target.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &into);
        } else {
            std::fs::copy(entry.path(), &into).expect("a file copies");
        }
    }
}
