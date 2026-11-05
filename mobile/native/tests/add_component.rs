use std::path::Path;
use sindri_mobile::{Editing, OpenedScene};

#[test]
fn add_collider_is_checked_and_undoable() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let mut opened = OpenedScene::open(&project, "world.scene").unwrap();
    let mut editing = Editing::for_scene(&opened).unwrap();
    let entity = editing.add_basic(&mut opened.world, "empty").unwrap();
    let kind = "sindri.physics2d.collider";
    assert!(editing.inspector(&opened.world, &opened.extractor, &opened.assets.sheets)["addable"].as_array().unwrap().iter().any(|choice| choice["type"] == kind));
    editing.add_component(&mut opened.world, &opened.extractor, kind).unwrap();
    assert!(opened.world.get(entity).unwrap().components.contains_key(kind));
    assert!(editing.add_component(&mut opened.world, &opened.extractor, kind).is_err());
    assert!(editing.add_component(&mut opened.world, &opened.extractor, "unknown").is_err());
    editing.undo(&mut opened.world).unwrap();
    assert!(!opened.world.get(entity).unwrap().components.contains_key(kind));
    editing.redo(&mut opened.world).unwrap();
    assert!(opened.world.get(entity).unwrap().components.contains_key(kind));
}
