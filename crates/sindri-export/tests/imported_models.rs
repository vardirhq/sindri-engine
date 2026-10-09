use sindri_assets::{AssetKind, AssetManifest, MANIFEST_FILE_NAME};
use sindri_core::AssetId;
use sindri_export::ProjectExport;
use std::path::{Path, PathBuf};

const GLB: &[u8] = include_bytes!("../../sindri-assets/tests/fixtures/models/static-model.glb");
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn write(root: &Path, path: &str, bytes: &[u8]) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}
fn project(name: &str, rooted: bool) -> Scratch {
    let root =
        std::env::temp_dir().join(format!("sindri-model-export-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let prefix = if rooted { "" } else { "assets/" };
    write(&root, "sindri.toml", format!(
        "format_version = 1\n[project]\nname = \"Models\"\nmain_scene = \"{prefix}main.scene\"\nscenes = [\"{prefix}other.scene\"]\n"
    ).as_bytes());
    write(
        &root,
        &format!("{prefix}main.scene"),
        br#"{
      "format_version": 10, "entities": [
        {"id":"one", "components":{"sindri.model":{"asset":"models/reused.glb"}}},
        {"id":"two", "components":{"sindri.model":{"asset":"models/reused.glb"}}},
        {"id":"placed", "prefab":{"source":"prefabs/part.prefab"}}
      ]}"#,
    );
    write(
        &root,
        &format!("{prefix}other.scene"),
        br#"{
      "format_version":10, "entities":[{"id":"hidden", "disabled":true,
      "components":{"sindri.model":{"asset":"models/hidden.glb"}}}]}"#,
    );
    write(
        &root,
        &format!("{prefix}prefabs/part.prefab"),
        br#"{
      "format_version":1, "entities":[{"id":"part",
      "components":{"sindri.model":{"asset":"models/part.glb"}}}]}"#,
    );
    for name in ["reused", "hidden", "part"] {
        write(&root, &format!("{prefix}models/{name}.glb"), GLB);
    }
    Scratch(root)
}
#[test]
fn root_and_assets_projects_ship_deduplicated_models_from_scenes_and_prefabs() {
    for rooted in [true, false] {
        let scratch = project(if rooted { "root" } else { "assets" }, rooted);
        let export = ProjectExport::gather(&scratch.0).unwrap();
        let models: Vec<_> = export
            .assets
            .iter()
            .filter(|asset| asset.kind == AssetKind::Model)
            .collect();
        assert_eq!(models.len(), 3);
        assert!(models.iter().all(|asset| asset.bytes == GLB));
        let destination = scratch.0.join("export");
        export.write(&destination, "/").unwrap();
        let manifest = AssetManifest::from_json(
            &std::fs::read_to_string(destination.join("assets").join(MANIFEST_FILE_NAME)).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest.ids_of(AssetKind::Model).count(), 3);
        for model in models {
            let id = AssetId::new(model.id.clone()).unwrap();
            let bytes = std::fs::read(
                destination
                    .join("assets")
                    .join(manifest.content_root())
                    .join(id.as_str()),
            )
            .unwrap();
            assert_eq!(bytes, GLB);
            manifest.verify(&id, &bytes).unwrap();
        }
    }
}
#[test]
fn missing_and_malformed_models_fail_export_with_the_asset_named() {
    let scratch = project("bad", true);
    write(&scratch.0, "models/reused.glb", b"invalid binary glTF");
    let error = ProjectExport::gather(&scratch.0).unwrap_err().to_string();
    assert!(error.contains("models/reused.glb"), "{error}");
    std::fs::remove_file(scratch.0.join("models/reused.glb")).unwrap();
    let error = ProjectExport::gather(&scratch.0).unwrap_err().to_string();
    assert!(error.contains("models/reused.glb"), "{error}");
}
