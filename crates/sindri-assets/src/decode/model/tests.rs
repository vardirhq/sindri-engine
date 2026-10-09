use super::*;
use crate::{AssetBytes, AssetDecoder};
use sindri_core::{AssetId, AssetLoadErrorKind};

const FIXTURE: &[u8] = include_bytes!("../../../tests/fixtures/models/static-model.glb");

fn bytes(data: Vec<u8>) -> AssetBytes {
    AssetBytes::new(AssetId::new("models/test.glb").unwrap(), data)
}

fn modified(edit: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let glb = gltf::binary::Glb::from_slice(FIXTURE).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&glb.json).unwrap();
    edit(&mut json);
    let mut encoded = serde_json::to_vec(&json).unwrap();
    while !encoded.len().is_multiple_of(4) {
        encoded.push(b' ');
    }
    let bin = glb.bin.as_deref().unwrap();
    let mut result = Vec::new();
    for word in [
        0x4654_6c67_u32,
        2,
        u32::try_from(28 + encoded.len() + bin.len()).unwrap(),
        u32::try_from(encoded.len()).unwrap(),
        0x4e4f_534a,
    ] {
        result.extend(word.to_le_bytes());
    }
    result.extend(encoded);
    result.extend(u32::try_from(bin.len()).unwrap().to_le_bytes());
    result.extend(0x004e_4942_u32.to_le_bytes());
    result.extend(bin);
    result
}

#[test]
fn preserves_hierarchy_materials_attributes_and_index_widths() {
    let model = ModelAssetDecoder.decode(bytes(FIXTURE.to_vec())).unwrap();
    assert_eq!(model.roots, [0]);
    assert_eq!(model.nodes.len(), 4);
    assert_eq!(model.nodes[0].children, [1, 2]);
    assert_eq!(model.nodes[1].name.as_deref(), Some("child"));
    assert_eq!(model.nodes[1].mesh, model.nodes[2].mesh);
    assert!((model.nodes[0].local_transform[3][1] - 2.0).abs() < f32::EPSILON);
    assert!((model.nodes[1].local_transform[1][1] - 3.0).abs() < f32::EPSILON);
    assert_eq!(model.meshes[0].primitives.len(), 2);
    assert!(
        matches!(&model.meshes[0].primitives[0].indices, ModelIndices::U16(v) if v == &[0,1,2])
    );
    assert!(
        matches!(&model.meshes[0].primitives[1].indices, ModelIndices::U32(v) if v == &[2,1,0])
    );
    assert_eq!(model.meshes[1].primitives[0].indices.to_u32(), [0, 1, 2]);
    assert!((model.meshes[1].primitives[0].normals[0][2] - 1.0).abs() < f32::EPSILON);
    assert!((model.materials[0].metallic - 0.3).abs() < f32::EPSILON);
    assert!((model.materials[0].roughness - 0.7).abs() < f32::EPSILON);
    assert_eq!(model.materials[1].base_color_texture, Some(0));
    assert_eq!(model.textures[0].image.rgba8(), [255, 128, 0, 255]);
}

#[test]
fn malformed_data_reports_the_asset_id_and_kind() {
    for data in [
        b"glTFbroken".to_vec(),
        FIXTURE[..FIXTURE.len() - 12].to_vec(),
        modified(|j| j["bufferViews"][0]["byteLength"] = 999_999.into()),
        modified(|j| j["accessors"][0]["count"] = 999_999.into()),
    ] {
        let error = ModelAssetDecoder.decode(bytes(data)).unwrap_err();
        assert_eq!(error.id().as_str(), "models/test.glb");
        assert_eq!(error.kind(), AssetLoadErrorKind::InvalidData);
    }
}

#[test]
fn deferred_features_have_useful_diagnostics() {
    for (data, message) in [
        (
            modified(|j| j["meshes"][0]["primitives"][0]["mode"] = 1.into()),
            "triangle",
        ),
        (
            modified(|j| j["materials"][0]["alphaMode"] = "BLEND".into()),
            "alpha blending",
        ),
        (
            modified(|j| j["images"][0] = serde_json::json!({"uri":"external.png"})),
            "external images",
        ),
        (
            modified(|j| {
                j["extensionsRequired"] = serde_json::json!(["KHR_draco_mesh_compression"]);
            }),
            "extension",
        ),
    ] {
        let error = ModelAssetDecoder.decode(bytes(data)).unwrap_err();
        assert_eq!(error.kind(), AssetLoadErrorKind::UnsupportedFormat);
        assert!(error.message().contains(message), "{error}");
    }
    let model = ModelAssetDecoder
        .decode(bytes(modified(|j| {
            j["extensionsUsed"] = serde_json::json!(["KHR_lights_punctual"]);
        })))
        .unwrap();
    assert!(model.warnings[0].contains("KHR_lights_punctual"));
}

#[test]
fn hierarchy_cycles_and_duplicate_parents_fail() {
    for data in [
        modified(|j| j["nodes"][3]["children"] = serde_json::json!([0])),
        modified(|j| j["nodes"][1]["children"] = serde_json::json!([3])),
    ] {
        assert_eq!(
            ModelAssetDecoder.decode(bytes(data)).unwrap_err().kind(),
            AssetLoadErrorKind::InvalidData
        );
    }
}

#[test]
fn malformed_attribute_types_fail_without_entering_unchecked_library_readers() {
    for (accessor, component, dimensions) in [
        (0, 5123, "VEC3"),
        (1, 5126, "SCALAR"),
        (2, 5122, "VEC2"),
        (3, 5126, "SCALAR"),
    ] {
        let data = modified(|json| {
            json["accessors"][accessor]["componentType"] = component.into();
            json["accessors"][accessor]["type"] = dimensions.into();
        });
        assert_eq!(
            ModelAssetDecoder.decode(bytes(data)).unwrap_err().kind(),
            AssetLoadErrorKind::InvalidData
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn repeated_model_requests_share_one_load_and_decoded_value() {
    use crate::{AssetLoadQueueConfig, AssetLoader, MemoryAssetSource};
    let id = AssetId::new("models/test.glb").unwrap();
    let mut source = MemoryAssetSource::default();
    source.insert(bytes(FIXTURE.to_vec()));
    let mut loader =
        AssetLoader::new(source, AssetLoadQueueConfig::default(), ModelAssetDecoder).unwrap();
    loader.request(id.clone()).unwrap();
    loader.request(id.clone()).unwrap();
    assert_eq!(loader.requested().len(), 1);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while loader.get(&id).is_none() && std::time::Instant::now() < deadline {
        assert!(
            loader
                .poll()
                .iter()
                .all(|outcome| !matches!(outcome, crate::AssetLoadOutcome::Failed(_)))
        );
        std::thread::yield_now();
    }
    let address = std::ptr::from_ref(loader.get(&id).unwrap());
    loader.request(id.clone()).unwrap();
    assert_eq!(address, std::ptr::from_ref(loader.get(&id).unwrap()));
    assert_eq!(loader.outstanding(), 0);
}
