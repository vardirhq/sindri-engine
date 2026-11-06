use sindri_mobile::{Editing, OpenedScene};
use std::path::Path;

#[test]
fn add_collider_is_checked_and_undoable() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let mut opened = OpenedScene::open(&project, "world.scene").unwrap();
    let mut editing = Editing::for_scene(&opened).unwrap();
    let entity = editing.add_basic(&mut opened.world, "empty").unwrap();
    let kind = "sindri.physics2d.collider";
    assert!(
        editing.inspector(&opened.world, &opened.extractor, &opened.assets.sheets)["addable"]
            .as_array()
            .unwrap()
            .iter()
            .any(|choice| choice["type"] == kind)
    );
    editing
        .add_component(&mut opened.world, &opened.extractor, kind)
        .unwrap();
    assert!(
        opened
            .world
            .get(entity)
            .unwrap()
            .components
            .contains_key(kind)
    );
    assert!(
        editing
            .add_component(&mut opened.world, &opened.extractor, kind)
            .is_err()
    );
    assert!(
        editing
            .add_component(&mut opened.world, &opened.extractor, "unknown")
            .is_err()
    );
    editing.undo(&mut opened.world).unwrap();
    assert!(
        !opened
            .world
            .get(entity)
            .unwrap()
            .components
            .contains_key(kind)
    );
    editing.redo(&mut opened.world).unwrap();
    assert!(
        opened
            .world
            .get(entity)
            .unwrap()
            .components
            .contains_key(kind)
    );
}

#[test]
fn a_loaded_texture_can_be_added_as_an_undoable_sprite() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let mut opened = OpenedScene::open(&project, "world.scene").unwrap();
    let texture = opened
        .assets
        .textures
        .first()
        .unwrap()
        .0
        .as_str()
        .to_owned();
    let mut editing = Editing::for_scene(&opened).unwrap();
    assert!(editing.add_sprite(&mut opened, "missing.png").is_err());
    let entity = editing.add_sprite(&mut opened, &texture).unwrap();
    assert_eq!(
        opened.world.get(entity).unwrap().components["sindri.sprite"]["texture"].as_str(),
        Some(texture.as_str())
    );
    editing.undo(&mut opened.world).unwrap();
    assert!(opened.world.get(entity).is_none());
    editing.redo(&mut opened.world).unwrap();
    assert!(opened.world.get(entity).is_some());
}

#[test]
fn sprite_component_can_be_added_and_changed_without_replacing_the_object() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let mut opened = OpenedScene::open(&project, "world.scene").unwrap();
    let first = opened.assets.textures[0].0.as_str().to_owned();
    let second = opened.assets.textures[1].0.as_str().to_owned();
    let mut editing = Editing::for_scene(&opened).unwrap();
    let entity = editing.add_basic(&mut opened.world, "empty").unwrap();
    assert!(editing.inspector(&opened.world, &opened.extractor, &opened.assets.sheets)["addable"].as_array().unwrap().iter().any(|choice| choice["type"] == "sindri.sprite"));
    assert!(editing.set_sprite(&mut opened, "missing.png").is_err());
    editing.set_sprite(&mut opened, &first).unwrap();
    editing.set_field(&mut opened.world, &opened.extractor, "sindri.sprite", "layer", serde_json::json!(7), false).unwrap();
    editing.set_sprite(&mut opened, &second).unwrap();
    let sprite = &opened.world.get(entity).unwrap().components["sindri.sprite"];
    assert_eq!(sprite["texture"].as_str(), Some(second.as_str()));
    assert_eq!(sprite["layer"].as_i64(), Some(7));
    assert_eq!(editing.selected, Some(entity));
    editing.undo(&mut opened.world).unwrap();
    let sprite = &opened.world.get(entity).unwrap().components["sindri.sprite"];
    assert_eq!(sprite["texture"].as_str(), Some(first.as_str()));
    assert_eq!(sprite["layer"].as_i64(), Some(7));
}
