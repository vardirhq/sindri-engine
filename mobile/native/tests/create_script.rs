//! Creating a behavior keeps the scene and script usable across undo and reopening.

use sindri_mobile::{Editing, OpenedScene};
use std::fs;

#[test]
fn new_behavior_attaches_saves_and_undo_keeps_the_file() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("assets/scripts")).unwrap();
    fs::write(
        directory.path().join("sindri.toml"),
        "format_version = 1\n[project]\nname = 'New game'\nmain_scene = 'assets/world.scene'\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("assets/world.scene"),
        r#"{"format_version":10,"entities":[]}"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("assets/scripts/other.decay"),
        "script MobileBehavior1 { fn start() {} }\n",
    )
    .unwrap();
    let mut opened = OpenedScene::open(directory.path(), "world.scene").unwrap();
    let mut editing = Editing::for_scene(&opened).unwrap();
    let entity = editing.add_basic(&mut opened.world, "empty").unwrap();
    let source = editing.create_script(&mut opened).unwrap();
    assert_eq!(source, "scripts/behavior-2.decay");
    let path = directory.path().join("assets").join(&source);
    assert!(path.is_file());
    assert_eq!(
        opened.world.get(entity).unwrap().components["sindri.script"]["source"].as_str(),
        Some(source.as_str())
    );
    // A second runs beside it, as a named part.
    let second = editing.create_script(&mut opened).unwrap();
    assert_eq!(second, "scripts/behavior-3.decay");
    assert_eq!(
        opened.world.get(entity).unwrap().components["sindri.script#script-2"]["source"].as_str(),
        Some(second.as_str())
    );
    editing.undo(&mut opened.world).unwrap();
    editing.undo(&mut opened.world).unwrap();
    assert!(
        !opened
            .world
            .get(entity)
            .unwrap()
            .components
            .contains_key("sindri.script")
    );
    assert!(path.is_file());
    editing.redo(&mut opened.world).unwrap();
    opened.save().unwrap();
    let reopened = OpenedScene::open(directory.path(), "world.scene").unwrap();
    assert!(reopened.world.entities().any(|(_, data)| {
        data.components
            .get("sindri.script")
            .and_then(|script| script.get("source"))
            .and_then(serde_json::Value::as_str)
            == Some(source.as_str())
    }));
}
