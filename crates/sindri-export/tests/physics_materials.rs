//! Component references ship reusable physics profiles without include lists.

use std::path::Path;

use sindri_assets::AssetKind;
use sindri_export::ProjectExport;

#[test]
fn platformer_exports_the_profile_shared_by_its_crate_and_planks_once() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/platformer");
    let export = ProjectExport::gather(&root).unwrap();
    let materials: Vec<_> = export
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::Profile)
        .collect();
    assert_eq!(materials.len(), 1);
    assert_eq!(materials[0].id, "materials/wood.profile");
    let profile =
        sindri_core::ProfileDocument::from_json(std::str::from_utf8(&materials[0].bytes).unwrap())
            .unwrap();
    assert!(
        sindri_scene::physics_material_profile(&materials[0].id, &profile)
            .unwrap()
            .is_some()
    );
}

#[test]
fn invalid_materials_fail_export_before_any_output_is_written() {
    let root = std::env::temp_dir().join(format!("sindri-invalid-material-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::write(
        root.join("sindri.toml"),
        "format_version = 1\n[project]\nname = \"Materials\"\nmain_scene = \"assets/main.scene\"\n",
    )
    .unwrap();
    std::fs::write(root.join("assets/main.scene"), r#"{"format_version":10,"entities":[{"id":"floor","components":{"sindri.physics2d.material":{"profile":"bad.profile"}}}]}"#).unwrap();
    std::fs::write(root.join("assets/bad.profile"), r#"{"format_version":1,"type":"physics_material","values":{"friction":-0.1,"restitution":0}}"#).unwrap();
    let error = match ProjectExport::gather(&root) {
        Ok(_) => panic!("invalid material exported"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("bad.profile"), "{error}");
    assert!(error.to_string().contains("non-negative"), "{error}");
    std::fs::write(
        root.join("assets/bad.profile"),
        r#"{"format_version":1,"type":"game_stats","values":{}}"#,
    )
    .unwrap();
    let error = match ProjectExport::gather(&root) {
        Ok(_) => panic!("wrong profile type exported"),
        Err(error) => error,
    };
    std::fs::remove_dir_all(root).unwrap();
    assert!(error.to_string().contains("physics_material"), "{error}");
}
