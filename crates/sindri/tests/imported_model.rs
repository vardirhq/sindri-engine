#![cfg(feature = "render")]

use sindri::assets::{AssetBytes, AssetDecoder, ModelAsset, ModelAssetDecoder};
use sindri::core::AssetId;
use sindri::render::ModelIndexData;

fn asset() -> ModelAsset {
    ModelAssetDecoder
        .decode(AssetBytes::new(
            AssetId::new("models/test.glb").unwrap(),
            include_bytes!("../../sindri-assets/tests/fixtures/models/static-model.glb").to_vec(),
        ))
        .unwrap()
}

#[test]
fn decoder_to_renderer_bridge_retains_hierarchy_indices_and_textures() {
    let prepared = sindri::model::prepare(asset()).unwrap();
    let data = prepared.resource.data();
    assert_eq!(data.nodes.len(), 4);
    assert_eq!(data.nodes[1].name.as_deref(), Some("child"));
    assert_eq!(data.nodes[0].children, [1, 2]);
    assert_eq!(data.meshes[0].len(), 2);
    assert!(matches!(data.meshes[0][0].indices, ModelIndexData::U16(_)));
    assert!(matches!(data.meshes[0][1].indices, ModelIndexData::U32(_)));
    assert_eq!(data.images[0].rgba, [255, 128, 0, 255]);
    assert_eq!(data.surfaces[1].texture, Some(0));
    assert!((data.surfaces[0].metallic - 0.3).abs() < 1e-6);
    assert!((data.surfaces[0].roughness - 0.7).abs() < 1e-6);
    let child = prepared
        .resource
        .instances()
        .iter()
        .find(|instance| instance.node == 1)
        .unwrap();
    assert!(
        (child.transform.w_axis.y - 3.0).abs() < 1e-6,
        "glTF stays Y-up"
    );
}

#[test]
fn bridge_reports_bad_attributes_and_preserves_import_warnings() {
    let mut bad = asset();
    bad.meshes[0].primitives[0].normals.clear();
    assert!(
        sindri::model::prepare(bad)
            .unwrap_err()
            .to_string()
            .contains("attribute counts")
    );
    let mut warned = asset();
    warned.warnings.push("unsupported optional feature".into());
    assert_eq!(
        sindri::model::prepare(warned).unwrap().warnings,
        ["unsupported optional feature"]
    );
}
