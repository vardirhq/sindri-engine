//! The phone lists the scenes that the same manifest gives the runtime.

use std::path::Path;

use sindri_mobile::{OpenedScene, listed_scenes};

#[test]
fn lamplighter_lists_and_opens_its_manifest_scenes() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let scenes = listed_scenes(&project).expect("the manifest reads");
    assert_eq!(scenes.iter().filter(|scene| scene.main).count(), 1);
    assert_eq!(scenes[0].name, "title.scene");
    assert!(scenes.iter().any(|scene| scene.name == "world.scene"));
    assert!(scenes.iter().any(|scene| scene.name == "village.scene"));
    for scene in scenes {
        let opened = OpenedScene::open(&project, &scene.name).expect("a listed scene opens");
        assert_eq!(opened.name, scene.name);
    }
}

#[test]
fn ambiguous_scene_file_names_are_rejected() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("sindri.toml"),
        "format_version = 1\n[project]\nname = 'Test'\nmain_scene = 'assets/world.scene'\nscenes = ['other/world.scene']\n",
    ).unwrap();
    assert!(listed_scenes(project.path()).is_err());
}
