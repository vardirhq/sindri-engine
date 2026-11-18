use std::fs;

use super::*;

/// A small project: a scene with a nested tree, a prefab placed twice, a
/// script and a picture.
fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("sindri.toml"),
        "[project]\nname = \"probe\"\nmain_scene = \"assets/main.scene\"\n",
    )
    .unwrap();
    let assets = dir.path().join("assets");
    fs::create_dir_all(assets.join("scripts")).unwrap();
    fs::create_dir_all(assets.join("textures")).unwrap();
    fs::write(
        assets.join("main.scene"),
        r#"{"format_version": 10, "entities": [
            {"id": "hero", "name": "Hero", "components": {
                "sindri.sprite": {"texture": "textures/hero.png"},
                "sindri.script": {"source": "scripts/hero.decay", "script": "Hero"}}},
            {"id": "hat", "parent": "hero", "components": {"sindri.sprite": {"texture": "textures/hero.png#hat"}}},
            {"id": "lamp-1", "prefab": {"source": "lamp.prefab"}},
            {"id": "lamp-2", "prefab": {"source": "lamp.prefab"}, "disabled": true}
        ]}"#,
    )
    .unwrap();
    fs::write(
        assets.join("lamp.prefab"),
        r#"{"format_version": 1, "entities": [
            {"id": "lamp", "components": {"sindri.light2d": {"radius": 3.0}}}
        ]}"#,
    )
    .unwrap();
    fs::write(assets.join("broken.scene"), "{ nope").unwrap();
    fs::write(
        assets.join("scripts/hero.decay"),
        "script Hero {\n    let look: string = \"textures/hero.png\";\n    fn update(dt: number) {}\n}\n",
    )
    .unwrap();
    fs::write(assets.join("textures/hero.png"), b"not really a png").unwrap();
    dir
}

#[test]
fn an_overview_says_what_the_project_holds() {
    let dir = project();
    let overview = overview(&Project::read(dir.path()));
    assert_eq!(overview.main_scene.as_deref(), Some("main.scene"));
    let scenes: Vec<_> = overview
        .scenes
        .iter()
        .map(|scene| (scene.id.as_str(), scene.entities))
        .collect();
    assert_eq!(scenes, [("broken.scene", 0), ("main.scene", 4)]);
    assert!(overview.scenes[0].error.is_some(), "a broken scene says so");
    assert_eq!(overview.prefabs[0].id, "lamp.prefab");
    assert_eq!(overview.scripts["scripts/hero.decay"], ["Hero"]);
    assert_eq!(overview.components["sindri.sprite"], 2);
    assert_eq!(overview.files["png"], 1);
}

#[test]
fn an_outline_is_the_scene_as_a_tree() {
    let dir = project();
    let project = Project::read(dir.path());
    let tree = outline(&project, None).unwrap();
    assert_eq!(tree.scene, "main.scene");
    let roots: Vec<_> = tree.roots.iter().map(|node| node.id.as_str()).collect();
    assert_eq!(roots, ["hero", "lamp-1", "lamp-2"]);
    assert_eq!(tree.roots[0].children[0].id, "hat");
    assert!(
        tree.roots[0]
            .components
            .contains(&"sindri.script(Hero)".to_owned())
    );
    let human = tree.human();
    assert!(
        human.contains("hero \"Hero\": script(Hero), sprite"),
        "{human}"
    );
    assert!(human.contains("  hat: sprite"), "{human}");
    assert!(human.contains("lamp-2 <lamp.prefab> (off)"), "{human}");
    assert!(outline(&project, Some("broken")).is_err());
    assert!(
        outline(&project, Some("lamp")).is_err(),
        "a prefab is named with its suffix"
    );
    assert_eq!(outline(&project, Some("lamp.prefab")).unwrap().entities, 1);
}

#[test]
fn find_matches_every_filter_given() {
    let dir = project();
    let project = Project::read(dir.path());
    let ids = |filter: &Filter<'_>| -> Vec<String> {
        find(&project, filter)
            .found
            .into_iter()
            .map(|found| found.id)
            .collect()
    };
    assert_eq!(
        ids(&Filter {
            component: Some("sindri.sprite"),
            ..Filter::default()
        }),
        ["hero", "hat"]
    );
    assert_eq!(
        ids(&Filter {
            script: Some("Hero"),
            ..Filter::default()
        }),
        ["hero"]
    );
    assert_eq!(
        ids(&Filter {
            script: Some("hero.decay"),
            ..Filter::default()
        }),
        ["hero"]
    );
    assert_eq!(
        ids(&Filter {
            prefab: Some("lamp.prefab"),
            ..Filter::default()
        }),
        ["lamp-1", "lamp-2"]
    );
    assert_eq!(
        ids(&Filter {
            name: Some("LAMP"),
            ..Filter::default()
        }),
        ["lamp", "lamp-1", "lamp-2"]
    );
    assert_eq!(
        ids(&Filter {
            name: Some("lamp"),
            scene: Some("main"),
            ..Filter::default()
        }),
        ["lamp-1", "lamp-2"]
    );
}

#[test]
fn show_gives_one_entity_and_its_children() {
    let dir = project();
    let project = Project::read(dir.path());
    let shown = show(&project, "hero", None).unwrap();
    assert_eq!(shown.scene, "main.scene");
    assert_eq!(shown.children, ["hat"]);
    assert!(shown.entity.components.contains_key("sindri.script"));
    assert!(show(&project, "nobody", None).is_err());
}

#[test]
fn uses_finds_every_reference_to_a_file() {
    let dir = project();
    let project = Project::read(dir.path());
    let at = |asset: &str| -> Vec<String> {
        uses(&project, asset)
            .uses
            .into_iter()
            .map(|found| format!("{}:{}", found.entity.unwrap_or_default(), found.at))
            .collect()
    };
    assert_eq!(
        at("textures/hero.png"),
        [
            "hero:sindri.sprite.texture",
            "hat:sindri.sprite.texture",
            ":line 2"
        ],
        "a region of it, and a script naming it, are uses too"
    );
    assert_eq!(
        at("assets/scripts/hero.decay"),
        ["hero:sindri.script.source"]
    );
    assert_eq!(at("lamp.prefab"), ["lamp-1:prefab", "lamp-2:prefab"]);
    assert!(at("textures/nothing.png").is_empty());
}
