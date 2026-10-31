//! Slice step 3 without a phone: a touch edit on a copy of Lamplighter, undone
//! and redone, saved, and read back after a restart by the phone, by the
//! desktop editor's open path, and by a build.
//!
//! The phone and the desktop open and save through the same `sindri-authoring`
//! code, so "opens on desktop" is checked as what that code reads back and
//! what a build plays. Pulling the file off a real phone is the device check in
//! `mobile/first-slice.md`.

use std::path::{Path, PathBuf};

use serde_json::json;
use sindri_authoring::{SceneFile, load_world};
use sindri_core::{EntityId, SceneDocument, SceneEntityId, World};
use sindri_mobile::{Editing, OpenedScene};
use sindri_runtime::{ProjectRun, STEP};

const SIZE: [f32; 2] = [1080.0, 2400.0];
const LIGHT: &str = "sindri.light2d";
const LAMP: &str = "cover-17";
const SCENE: &str = "world.scene";

/// A copy of Lamplighter to save into, so the checked-in game is never touched.
fn lamplighter_copy() -> (tempfile::TempDir, PathBuf) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let dir = tempfile::tempdir().expect("a temporary directory");
    let root = dir.path().join("lamplighter");
    std::fs::create_dir_all(&root).expect("the copy's folder");
    std::fs::copy(source.join("sindri.toml"), root.join("sindri.toml")).expect("the manifest");
    copy_tree(&source.join("assets"), &root.join("assets"));
    (dir, root)
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

fn entity(world: &World, id: &str) -> EntityId {
    world
        .entity_for_source_id(&SceneEntityId::new(id.to_owned()).expect("a valid ID"))
        .unwrap_or_else(|| panic!("the scene has {id}"))
}

fn position(world: &World, id: &str) -> [f32; 3] {
    world
        .get(entity(world, id))
        .and_then(|data| data.transform_3d)
        .expect("a transform")
        .position
}

fn intensity(world: &World) -> f64 {
    world.get(entity(world, LAMP)).expect("the lamp").components[LIGHT]["intensity"]
        .as_f64()
        .expect("a number")
}

fn near(left: f32, right: f32) -> bool {
    (left - right).abs() < 1e-4
}

/// The world as it would be written, to compare one opening with another.
fn written(opened: &OpenedScene) -> SceneDocument {
    opened
        .world
        .to_scene_with_references(opened.file.prefabs(), opened.extractor.components())
        .expect("the world writes")
}

/// Drags the lamp 150 pixels right and edits its light, as the app's gestures
/// and inspector would; returns where it was moved to.
fn edit(opened: &mut OpenedScene, editing: &mut Editing) -> [f32; 3] {
    let lamp = entity(&opened.world, LAMP);
    let start = position(&opened.world, LAMP);
    editing.canvas.center = [start[0], start[1]];
    editing.selected = Some(lamp);
    let on = editing
        .selection_pixel(&opened.world, SIZE)
        .expect("the lamp is on the canvas");
    editing
        .press(&opened.world, &opened.extractor, on, SIZE)
        .expect("the press picks");
    editing
        .drag(&mut opened.world, [on[0] + 150.0, on[1]], SIZE)
        .expect("the move applies");
    editing.release();
    editing
        .set_field(
            &mut opened.world,
            &opened.extractor,
            LIGHT,
            "intensity",
            json!(1.5),
        )
        .expect("the edit applies");
    position(&opened.world, LAMP)
}

#[test]
fn a_touch_edit_saves_survives_a_restart_and_reads_back_on_desktop_and_in_a_build() {
    let (_dir, root) = lamplighter_copy();
    let file = root.join("assets").join(SCENE);

    let mut opened = OpenedScene::open(&root, SCENE).expect("the scene opens");
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    assert!(
        !editing.unsaved(),
        "a scene just opened has nothing to save"
    );
    let moved = edit(&mut opened, &mut editing);
    assert!(editing.unsaved());

    // Undo and redo on the way, as a person trying the edit out would.
    editing.undo(&mut opened.world).expect("undo applies");
    editing.undo(&mut opened.world).expect("undo applies");
    assert!(
        !editing.unsaved(),
        "undoing back to the file is saved again"
    );
    editing.redo(&mut opened.world).expect("redo applies");
    editing.redo(&mut opened.world).expect("redo applies");
    assert!(near(position(&opened.world, LAMP)[0], moved[0]));
    let edited = written(&opened);

    opened.save().expect("the scene saves");
    editing.saved();
    assert!(!editing.unsaved(), "a saved scene has nothing left to save");
    let saved_bytes = std::fs::read(&file).expect("the file reads");
    drop(editing);
    drop(opened);

    // Restart: the phone opens the project again from its files alone.
    let reopened = OpenedScene::open(&root, SCENE).expect("the scene reopens");
    assert_eq!(written(&reopened), edited, "the phone reads back its edit");
    assert!((intensity(&reopened.world) - 1.5).abs() < 1e-9);

    // Desktop: the editor's open path, which is this same code.
    let desktop = SceneFile::open(&file).expect("desktop opens the file");
    let world = load_world(
        &sindri_runtime::scene_extractor().expect("schemas"),
        &desktop,
    )
    .expect("desktop builds the world");
    let at = position(&world, LAMP);
    assert!(near(at[0], moved[0]) && near(at[1], moved[1]), "{at:?}");
    assert!((intensity(&world) - 1.5).abs() < 1e-9);

    // A build plays the saved edit.
    let mut run = ProjectRun::open_scene(&root, SCENE, SIZE).expect("the build opens");
    let lamp = run.entity(LAMP).expect("the build has the lamp");
    let played = run.position(lamp);
    assert!(
        near(played[0], moved[0]) && near(played[1], moved[1]),
        "{played:?}"
    );
    run.step(STEP).expect("the build steps");

    // Saving again with nothing changed writes the same bytes, so a save
    // carries only what was edited.
    let mut again = OpenedScene::open(&root, SCENE).expect("the scene reopens");
    again.save().expect("the scene saves again");
    assert_eq!(std::fs::read(&file).expect("the file reads"), saved_bytes);
}

#[test]
fn a_failed_save_leaves_the_file_and_the_unsaved_state_as_they_were() {
    let (_dir, root) = lamplighter_copy();
    let file = root.join("assets").join(SCENE);
    let before = std::fs::read(&file).expect("the file reads");
    let mut opened = OpenedScene::open(&root, SCENE).expect("the scene opens");
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    edit(&mut opened, &mut editing);

    // The folder goes away underneath the app, as a cleared storage would.
    std::fs::remove_dir_all(root.join("assets")).expect("the folder goes");
    assert!(opened.save().is_err(), "a save with nowhere to go fails");
    assert!(editing.unsaved(), "the edit is still unsaved");
    assert!(!file.exists() || std::fs::read(&file).expect("reads") == before);
}
