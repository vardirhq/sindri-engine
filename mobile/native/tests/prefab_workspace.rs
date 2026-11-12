//! A prefab opened in the object workspace: edited on its own, saved through
//! the desktop editor's prefab path, and followed by the open scene's
//! instances, which keep what they override.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sindri_core::{EntityId, World};
use sindri_mobile::{Editing, OpenedScene, PrefabSession};

const SCENE: &str = "world.scene";
const PREFAB: &str = "prefabs/lamp.prefab";

/// A copy of Lamplighter with one lamp prefab added, so the checked-in game is
/// never touched.
fn lamplighter_with_a_prefab() -> (tempfile::TempDir, PathBuf) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let dir = tempfile::tempdir().expect("a temporary directory");
    let root = dir.path().join("lamplighter");
    std::fs::create_dir_all(&root).expect("the copy's folder");
    std::fs::copy(source.join("sindri.toml"), root.join("sindri.toml")).expect("the manifest");
    copy_tree(&source.join("assets"), &root.join("assets"));
    let prefab = json!({
        "format_version": 1,
        "metadata": { "name": "Lamp" },
        "entities": [{
            "id": "lamp",
            "name": "Lamp",
            "transform_3d": { "position": [0.0, 0.0, 0.5] },
            "components": {
                "sindri.sprite": { "texture": "textures/objects.png#lamp-post-unlit", "size": [1.0, 3.0] },
                "sindri.light2d": { "radius": 4.0, "intensity": 1.0 }
            }
        }]
    });
    std::fs::create_dir_all(root.join("assets/prefabs")).expect("a prefabs folder");
    std::fs::write(root.join("assets").join(PREFAB), prefab.to_string()).expect("the prefab");
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

fn component(world: &World, entity: EntityId, type_name: &str) -> Value {
    world.get(entity).expect("the entity is there").components[type_name].clone()
}

/// Places two lamps, the second overriding its prefab's intensity, and saves.
fn two_lamps_one_brighter(opened: &mut OpenedScene, editing: &mut Editing) -> (EntityId, EntityId) {
    let plain = editing
        .place_prefab(opened, PREFAB)
        .expect("a lamp is placed");
    let bright = editing.place_prefab(opened, PREFAB).expect("another lamp");
    editing.selected = Some(bright);
    editing
        .set_field(
            &mut opened.world,
            &opened.extractor,
            "sindri.light2d",
            "intensity",
            json!(2.5),
            false,
        )
        .expect("one lamp is brighter than its prefab");
    opened.save().expect("the scene saves");
    editing.saved();
    (plain, bright)
}

/// The prefab file holds the change, and the scene reopens with it in both lamps.
fn on_disk_both_lamps_have_the_new_art(project: &Path) {
    let written: Value = serde_json::from_str(
        &std::fs::read_to_string(project.join("assets").join(PREFAB)).expect("the prefab reads"),
    )
    .expect("the prefab is JSON");
    assert_eq!(
        written["entities"][0]["components"]["sindri.sprite"]["size"],
        json!([2.0, 4.0])
    );
    let reopened = OpenedScene::open(project, SCENE).expect("the scene reopens");
    let lamps: Vec<EntityId> = reopened
        .world
        .entities()
        .filter(|(_, data)| {
            data.prefab
                .as_ref()
                .is_some_and(|link| link.root && link.source == PREFAB)
        })
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(lamps.len(), 2);
    for lamp in lamps {
        assert_eq!(
            component(&reopened.world, lamp, "sindri.sprite")["size"],
            json!([2.0, 4.0])
        );
    }
}

#[test]
fn a_prefab_edited_in_the_workspace_updates_every_instance_and_keeps_overrides() {
    let (_dir, project) = lamplighter_with_a_prefab();
    let mut opened = OpenedScene::open(&project, SCENE).expect("the scene opens");
    let mut editing = Editing::for_scene(&opened).expect("editing starts");
    let (plain, bright) = two_lamps_one_brighter(&mut opened, &mut editing);

    // The prefab opens on its own, its root chosen, as the workspace shows it.
    let mut session = PrefabSession::open(&opened, PREFAB).expect("the prefab opens");
    assert_eq!(session.id, PREFAB);
    let root = session.editing.selected.expect("the root is chosen");
    assert_eq!(session.world.entities().count(), 1);

    // Size the art and move the light, as the workspace's handles do.
    session
        .editing
        .set_field(
            &mut session.world,
            &opened.extractor,
            "sindri.sprite",
            "size",
            json!([2.0, 4.0]),
            false,
        )
        .expect("the art is sized");
    session
        .editing
        .set_field(
            &mut session.world,
            &opened.extractor,
            "sindri.light2d",
            "offset",
            json!([0.0, 1.5]),
            false,
        )
        .expect("the light is moved");
    assert!(session.editing.unsaved());
    assert!(
        session
            .save(&mut opened, &mut editing)
            .expect("the prefab saves"),
        "the scene's instances changed"
    );
    assert!(editing.unsaved(), "the scene is left for its host to save");
    opened.save().expect("the scene saves");
    editing.saved();
    assert!(!session.editing.unsaved());
    assert_eq!(
        component(&session.world, root, "sindri.sprite")["size"],
        json!([2.0, 4.0])
    );

    // Both instances follow, and the brighter one is still brighter.
    for lamp in [plain, bright] {
        assert_eq!(
            component(&opened.world, lamp, "sindri.sprite")["size"],
            json!([2.0, 4.0])
        );
        assert_eq!(
            component(&opened.world, lamp, "sindri.light2d")["offset"],
            json!([0.0, 1.5])
        );
    }
    assert_eq!(
        component(&opened.world, bright, "sindri.light2d")["intensity"],
        json!(2.5)
    );

    on_disk_both_lamps_have_the_new_art(&project);

    // The scene's step is one undo: both lamps go back to the old art at once.
    editing.undo(&mut opened.world).expect("the update undoes");
    for lamp in [plain, bright] {
        assert_eq!(
            component(&opened.world, lamp, "sindri.sprite")["size"],
            json!([1.0, 3.0])
        );
    }
}

#[test]
fn the_inspector_names_an_instances_prefab_for_edit_prefab() {
    let (_dir, project) = lamplighter_with_a_prefab();
    let mut opened = OpenedScene::open(&project, SCENE).expect("the scene opens");
    let mut editing = Editing::for_scene(&opened).expect("editing starts");
    let lamp = editing
        .place_prefab(&mut opened, PREFAB)
        .expect("a lamp is placed");
    editing.selected = Some(lamp);
    let shown = editing.inspector(&opened.world, &opened.extractor, &opened.assets.sheets);
    assert_eq!(shown["prefab"], json!(PREFAB));
    let id = shown["prefab"].as_str().expect("an ID");
    assert!(
        PrefabSession::open(&opened, id).is_ok(),
        "what it names opens"
    );
    editing.selected = opened
        .world
        .entities()
        .find(|(_, data)| data.prefab.is_none() && data.parent.is_none())
        .map(|(entity, _)| entity);
    let plain = editing.inspector(&opened.world, &opened.extractor, &opened.assets.sheets);
    assert_eq!(plain["prefab"], Value::Null);
}

#[test]
fn a_prefab_the_project_does_not_hold_is_refused() {
    let (_dir, project) = lamplighter_with_a_prefab();
    let opened = OpenedScene::open(&project, SCENE).expect("the scene opens");
    let error = PrefabSession::open(&opened, "prefabs/nothing.prefab")
        .err()
        .expect("refused");
    assert!(error.contains("not in the project"), "{error}");
}
