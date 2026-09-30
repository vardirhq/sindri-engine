//! A scene that places prefab instances ships the prefabs, and what they draw.
//!
//! An instance's sprite is named in its prefab, not in the scene, so an export
//! that walked only the scene would ship a world of placed things drawn with
//! textures nobody downloaded.

use std::path::{Path, PathBuf};

use sindri_assets::AssetKind;
use sindri_export::ProjectExport;

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("a file has a directory"))
        .expect("the directory is made");
    std::fs::write(path, text).expect("the file is written");
}

/// A scene placing a chest, whose prefab places a coin.
fn project() -> Scratch {
    let root = std::env::temp_dir().join(format!("sindri-placed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    write(
        &root,
        "sindri.toml",
        "format_version = 1\n\n[project]\nname = \"Placed\"\nmain_scene = \"assets/main.scene\"\n",
    );
    write(
        &root,
        "assets/main.scene",
        r#"{
          "format_version": 10,
          "entities": [{
            "id": "chest-1",
            "transform_3d": {"position": [2,0,0], "rotation": [0,0,0,1], "scale": [1,1,1]},
            "prefab": {"source": "prefabs/chest.prefab"}
          }]
        }"#,
    );
    write(
        &root,
        "assets/prefabs/chest.prefab",
        r#"{
          "format_version": 1,
          "entities": [
            {"id": "chest", "components": {"sindri.sprite": {"texture": "textures/chest.png"}}},
            {"id": "loot", "parent": "chest", "prefab": {"source": "prefabs/coin.prefab"}}
          ]
        }"#,
    );
    write(
        &root,
        "assets/prefabs/coin.prefab",
        r#"{
          "format_version": 1,
          "entities": [
            {"id": "coin", "components": {"sindri.sprite": {"texture": "textures/coin.png"}}}
          ]
        }"#,
    );
    // The exporter carries bytes and never decodes them.
    write(&root, "assets/textures/chest.png", "chest");
    write(&root, "assets/textures/coin.png", "coin");
    Scratch(root)
}

fn carried(export: &ProjectExport, kind: AssetKind) -> Vec<&str> {
    export
        .assets
        .iter()
        .filter(|asset| asset.kind == kind)
        .map(|asset| asset.id.as_str())
        .collect()
}

#[test]
fn a_placed_prefab_ships_with_the_prefabs_nested_in_it_and_their_textures() {
    let scratch = project();
    let export = ProjectExport::gather(&scratch.0).expect("the project gathers");
    assert_eq!(
        carried(&export, AssetKind::Prefab),
        ["prefabs/chest.prefab", "prefabs/coin.prefab"]
    );
    assert_eq!(
        carried(&export, AssetKind::Texture),
        ["textures/chest.png", "textures/coin.png"]
    );
}
