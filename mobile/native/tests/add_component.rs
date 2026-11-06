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
    assert!(
        editing.inspector(&opened.world, &opened.extractor, &opened.assets.sheets)["addable"]
            .as_array()
            .unwrap()
            .iter()
            .any(|choice| choice["type"] == "sindri.sprite")
    );
    assert!(editing.set_sprite(&mut opened, "missing.png").is_err());
    editing.set_sprite(&mut opened, &first).unwrap();
    editing
        .set_field(
            &mut opened.world,
            &opened.extractor,
            "sindri.sprite",
            "layer",
            serde_json::json!(7),
            false,
        )
        .unwrap();
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

#[test]
fn legacy_collider_is_shown_and_compound_edits_undo() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let mut opened = OpenedScene::open(&project, "world.scene").unwrap();
    let mut editing = Editing::for_scene(&opened).unwrap();
    let entity = editing.add_basic(&mut opened.world, "empty").unwrap();
    let kind = "sindri.physics2d.collider";
    let default = opened.extractor.components().default_payload(kind).unwrap();
    let mut legacy = default["pieces"][0].clone();
    legacy["shape"] = serde_json::json!({"shape":"circle", "radius": 2.0});
    opened.world.get_mut(entity).unwrap().components.insert(kind.to_owned(), legacy);
    let inspector = editing.inspector(&opened.world, &opened.extractor, &opened.assets.sheets);
    let component = inspector["components"].as_array().unwrap().iter().find(|item| item["type"] == kind).unwrap();
    assert_eq!(component["values"]["shape"]["shape"].as_str(), Some("circle"));
    assert!(component["values"].get("pieces").is_none());
    editing.set_field(&mut opened.world, &opened.extractor, kind, "shape.radius", serde_json::json!(3.0), false).unwrap();
    assert!((opened.world.get(entity).unwrap().components[kind]["shape"]["radius"].as_f64().unwrap() - 3.0).abs() < 1e-6);
    editing.undo(&mut opened.world).unwrap();
    editing.edit_collider_pieces(&mut opened.world, &opened.extractor, 0, true).unwrap();
    assert_eq!(opened.world.get(entity).unwrap().components[kind]["pieces"].as_array().unwrap().len(), 2);
    editing.edit_collider_pieces(&mut opened.world, &opened.extractor, 1, false).unwrap();
    assert_eq!(opened.world.get(entity).unwrap().components[kind]["pieces"].as_array().unwrap().len(), 1);
    editing.undo(&mut opened.world).unwrap();
    editing.undo(&mut opened.world).unwrap();
    assert_eq!(opened.world.get(entity).unwrap().components[kind]["shape"]["shape"].as_str(), Some("circle"));
}
