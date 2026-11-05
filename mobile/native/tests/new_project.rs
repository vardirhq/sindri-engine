//! The blank scene shipped by Android opens and saves through the shared authoring path.

use std::fs;
use sindri_mobile::{Editing, OpenedScene};

#[test]
fn mobile_blank_scene_opens_and_reopens() {
    let source = include_str!("../../android/app/src/main/kotlin/dev/sindri/mobile/LocalProjects.kt");
    let scene = source.split("private const val SCENE = \"\"\"").nth(1).unwrap().split("\"\"").next().unwrap();
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("assets")).unwrap();
    fs::write(directory.path().join("sindri.toml"), "format_version = 1\n[project]\nname = 'New game'\nmain_scene = 'assets/world.scene'\n").unwrap();
    fs::write(directory.path().join("assets/world.scene"), scene).unwrap();
    let mut opened = OpenedScene::open(directory.path(), "world.scene").unwrap();
    let _editing = Editing::for_scene(&opened).unwrap();
    opened.save().unwrap();
    assert_eq!(OpenedScene::open(directory.path(), "world.scene").unwrap().world.len(), 1);
}
