//! Placing objects remains an undoable, portable scene edit.
use std::path::Path;

use sindri_mobile::{Editing, OpenedScene};

fn project(root: &Path) {
    std::fs::create_dir_all(root.join("assets/prefabs")).unwrap();
    std::fs::write(root.join("sindri.toml"), "format_version = 1\n[project]\nname = \"Test\"\nmain_scene = \"assets/world.scene\"\n").unwrap();
    std::fs::write(root.join("assets/world.scene"), r#"{"format_version":10,"metadata":{"name":"Test"},"entities":[]}"#).unwrap();
    std::fs::write(root.join("assets/prefabs/marker.prefab"), r#"{"format_version":1,"metadata":{"name":"Marker"},"entities":[{"id":"marker","name":"Marker","transform_3d":{"position":[0,0,2],"rotation":[0,0,0,1],"scale":[1,1,1]},"components":{}}]}"#).unwrap();
}

#[test]
fn place_prefab_undo_redo_and_reopen_as_an_instance() {
    let dir = tempfile::tempdir().unwrap();
    project(dir.path());
    let mut opened = OpenedScene::open(dir.path(), "world.scene").unwrap();
    let mut editing = Editing::for_scene(&opened).unwrap();
    editing.canvas.center = [3.0, -4.0];
    let entity = editing.place_prefab(&mut opened, "prefabs/marker.prefab").unwrap();
    let data = opened.world.get(entity).unwrap();
    assert!(data.prefab.is_some());
    let position = data.transform_3d.unwrap().position;
    assert!((position[0] - 3.0).abs() < f32::EPSILON);
    assert!((position[1] + 4.0).abs() < f32::EPSILON);
    assert!((position[2] - 2.0).abs() < f32::EPSILON);
    editing.undo(&mut opened.world).unwrap();
    assert!(opened.world.get(entity).is_none());
    editing.redo(&mut opened.world).unwrap();
    opened.save().unwrap();
    let reopened = OpenedScene::open(dir.path(), "world.scene").unwrap();
    assert!(reopened.world.entities().any(|(_, data)| data.prefab.is_some()));
}

#[test]
fn add_empty_and_light_at_the_view_center_with_undo() {
    let dir = tempfile::tempdir().unwrap();
    project(dir.path());
    let mut opened = OpenedScene::open(dir.path(), "world.scene").unwrap();
    let mut editing = Editing::for_scene(&opened).unwrap();
    editing.canvas.center = [1.0, 2.0];
    let empty = editing.add_basic(&mut opened.world, "empty").unwrap();
    let light = editing.add_basic(&mut opened.world, "light2d").unwrap();
    let position = opened.world.get(empty).unwrap().transform_3d.unwrap().position;
    assert!((position[0] - 1.0).abs() < f32::EPSILON);
    assert!((position[1] - 2.0).abs() < f32::EPSILON);
    assert!(opened.world.get(light).unwrap().components.contains_key("sindri.light2d"));
    editing.undo(&mut opened.world).unwrap();
    assert!(opened.world.get(light).is_none());
}
