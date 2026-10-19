//! Material components, inactive scenes and expanded prefabs all ship dependencies.
use sindri_assets::AssetKind;
use sindri_export::ProjectExport;
use std::path::{Path, PathBuf};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

#[test]
fn all_authored_references_ship_one_typed_profile_and_invalid_edits_fail_export() {
    let root = Scratch(std::env::temp_dir().join(format!(
        "sindri-render-material-export-{}",
        std::process::id()
    )));
    let _ = std::fs::remove_dir_all(&root.0);
    write(
        &root.0,
        "sindri.toml",
        "format_version = 1\n[project]\nname = \"Materials\"\nmain_scene = \"assets/main.scene\"\nscenes = [\"assets/other.scene\"]\n",
    );
    write(
        &root.0,
        "assets/main.scene",
        r#"{"format_version":10,"entities":[
        {"id":"one","components":{"sindri.material":{"profile":"gold.profile"}}},
        {"id":"placed","prefab":{"source":"part.prefab"}}]}"#,
    );
    write(
        &root.0,
        "assets/other.scene",
        r#"{"format_version":10,"entities":[{"id":"later","disabled":true,"components":{"sindri.material":{"profile":"gold.profile"}}}]}"#,
    );
    write(
        &root.0,
        "assets/part.prefab",
        r#"{"format_version":1,"entities":[{"id":"part","components":{"sindri.material":{"profile":"gold.profile"}}}]}"#,
    );
    let mut profile = sindri_scene::render_material_template();
    write(
        &root.0,
        "assets/gold.profile",
        &profile.to_canonical_json().unwrap(),
    );
    let exported = ProjectExport::gather(&root.0).unwrap();
    let materials: Vec<_> = exported
        .assets
        .iter()
        .filter(|a| a.kind == AssetKind::Profile)
        .collect();
    assert_eq!(materials.len(), 1);
    assert_eq!(materials[0].id, "gold.profile");
    assert_eq!(
        materials[0].bytes,
        profile.to_canonical_json().unwrap().as_bytes()
    );
    profile.profile_type = "game_tuning".into();
    write(
        &root.0,
        "assets/gold.profile",
        &profile.to_canonical_json().unwrap(),
    );
    let error = ProjectExport::gather(&root.0).unwrap_err().to_string();
    assert!(
        error.contains("gold.profile") && error.contains("render_material"),
        "{error}"
    );
    profile.profile_type = "render_material".into();
    profile
        .values
        .insert("roughness".into(), serde_json::json!(-1));
    write(
        &root.0,
        "assets/gold.profile",
        &profile.to_canonical_json().unwrap(),
    );
    let error = ProjectExport::gather(&root.0).unwrap_err().to_string();
    assert!(
        error.contains("gold.profile") && error.contains("roughness"),
        "{error}"
    );
    std::fs::remove_file(root.0.join("assets/gold.profile")).unwrap();
    assert!(
        ProjectExport::gather(&root.0)
            .unwrap_err()
            .to_string()
            .contains("gold.profile")
    );
}

#[test]
fn lighting_lab_discovers_three_shared_material_assets_without_include_lists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighting3d");
    let exported = ProjectExport::gather(&root).unwrap();
    let profiles: Vec<_> = exported
        .assets
        .iter()
        .filter(|a| a.kind == AssetKind::Profile)
        .collect();
    assert_eq!(profiles.len(), 3);
    for profile in profiles {
        let document =
            sindri_core::ProfileDocument::from_json(std::str::from_utf8(&profile.bytes).unwrap())
                .unwrap();
        assert!(
            sindri_scene::render_material_profile(&profile.id, &document)
                .unwrap()
                .is_some()
        );
    }
}
