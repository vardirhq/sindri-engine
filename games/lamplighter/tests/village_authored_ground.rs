//! Village terrain is authored, matches the old painter, and stays editable.

use std::{fs, path::Path};

use lamplighter::{Run, project};
use serde_json::{Value, json};
use sindri_runtime::{ProjectRun, STEP};

const LAYERS: [&str; 4] = ["ground", "soil", "cobble", "flag"];

fn sprites(run: &ProjectRun, id: &str) -> Vec<Option<String>> {
    let entity = run.entity(id).expect("layer exists");
    let payload = &run.world.get(entity).expect("entity exists").components["sindri.tilemap"];
    let palette = payload["palette"].as_array().expect("palette");
    payload["tiles"].as_array().expect("tiles").iter().map(|tile| {
        tile.as_u64().map(|index| {
            let index = usize::try_from(index).expect("palette index fits");
            palette[index].as_str().expect("sprite name").to_owned()
        })
    }).collect()
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create folder");
    for entry in fs::read_dir(from).expect("read folder") {
        let entry = entry.expect("entry");
        let destination = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).expect("copy asset");
        }
    }
}

/// Restore the old empty maps and the original script on an isolated project.
fn legacy_project(root: &Path) {
    fs::copy(project().join("sindri.toml"), root.join("sindri.toml")).expect("manifest");
    copy_tree(&project().join("assets"), &root.join("assets"));
    fs::copy(
        project().join("tests/fixtures/village_ground.decay"),
        root.join("assets/scripts/village_ground.decay"),
    ).expect("legacy script");
    let path = root.join("assets/village.scene");
    let mut scene: Value = serde_json::from_slice(&fs::read(&path).expect("scene")).expect("json");
    for entity in scene["entities"].as_array_mut().expect("entities") {
        let id = entity["id"].as_str().unwrap_or("").to_owned();
        if !LAYERS.contains(&id.as_str()) { continue; }
        let components = entity["components"].as_object_mut().expect("components");
        let map = components.get_mut("sindri.tilemap").expect("tilemap");
        if id == "ground" {
            map["tiles"] = json!(vec![0; 46 * 38]);
            components.insert("sindri.script".to_owned(), json!({
                "source": "scripts/village_ground.decay", "script": "VillageGround",
                "properties": {"left": -23.0, "top": 19.0}
            }));
        } else {
            map["palette"] = json!([]);
            map["tiles"] = json!(vec![Value::Null; 46 * 38]);
        }
    }
    fs::write(path, serde_json::to_vec(&scene).expect("serialize")).expect("write scene");
}

#[test]
fn authored_village_matches_the_original_runtime_painter() {
    let authored = Run::open_scene("village.scene").expect("authored scene opens");
    let directory = tempfile::tempdir().expect("temporary project");
    legacy_project(directory.path());
    let mut legacy = ProjectRun::open_scene(directory.path(), "village.scene", [960.0, 540.0])
        .expect("legacy scene opens");
    for _ in 0..5 {
        let report = legacy.step(STEP).expect("legacy scene steps");
        assert!(report.notes().is_empty(), "legacy scripts: {:?}", report.notes());
    }
    for layer in LAYERS {
        assert_eq!(sprites(&authored, layer), sprites(&legacy, layer), "{layer}");
    }
}

#[test]
fn play_does_not_repaint_the_authored_ground() {
    let mut run = Run::open_scene("village.scene").expect("village opens");
    let before: Vec<_> = LAYERS.iter().map(|id| sprites(&run, id)).collect();
    assert!(before[2].iter().any(Option::is_some), "the authored cobbles exist");
    assert!(before[3].iter().any(Option::is_some), "the authored flagstones exist");
    for _ in 0..5 { assert!(run.step(STEP).is_empty()); }
    for (index, id) in LAYERS.iter().enumerate() {
        assert_eq!(before[index], sprites(&run, id), "Play changed {id}");
    }
}
