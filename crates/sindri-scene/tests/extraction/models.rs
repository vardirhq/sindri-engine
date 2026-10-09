use std::sync::Arc;

use glam::Vec3;
use sindri_core::{AssetId, SceneComponent, SceneDocument, UnknownComponentPolicy};
use sindri_render::{FrameCommand, ModelData, RenderModel};
use sindri_scene::{
    CameraView, ModelComponent, SceneExtractError, SceneExtractor, TextureBindings,
    referenced_models,
};

use crate::support::{VIEWPORT, scene, world_from};

const MODELS: &str = r#",
    { "id": "parent", "transform_3d": { "position": [2,3,4], "scale": [2,2,2] } },
    { "id": "one", "parent": "parent", "transform_3d": { "position": [1,0,0] },
      "components": { "sindri.model": { "asset": "models/test.glb", "layer": 4 } } },
    { "id": "two", "transform_3d": { "position": [-2,0,0] },
      "components": { "sindri.model": { "asset": "models/test.glb", "layer": 8 } } }"#;

#[test]
fn model_references_share_resources_and_apply_world_transforms() {
    let world = world_from(&scene(MODELS));
    let asset = Arc::new(RenderModel::new(ModelData::default()).unwrap());
    let mut extractor = SceneExtractor::new().unwrap();
    let id = AssetId::new("models/test.glb").unwrap();
    extractor.bind_model(id.clone(), Arc::clone(&asset));
    let frame = extractor
        .extract(
            &world,
            VIEWPORT,
            CameraView::default(),
            &TextureBindings::new(),
        )
        .unwrap();
    assert_eq!(frame.passes().len(), 2);
    let FrameCommand::Model {
        model,
        asset: first,
    } = &frame.passes()[0].command
    else {
        panic!("model pass expected");
    };
    assert!(
        model
            .transform_point3(Vec3::ZERO)
            .abs_diff_eq(Vec3::new(4.0, 3.0, 4.0), 1e-6)
    );
    assert!(
        model
            .transform_vector3(Vec3::X)
            .abs_diff_eq(Vec3::X * 2.0, 1e-6)
    );
    let FrameCommand::Model { asset: second, .. } = &frame.passes()[1].command else {
        panic!("model pass expected");
    };
    assert!(Arc::ptr_eq(first, second) && Arc::ptr_eq(first, &asset));
    assert_eq!(frame.passes()[0].layer.0, 4);
    assert_eq!(frame.passes()[1].layer.0, 8);
    extractor.unbind_model(&id);
    assert!(
        extractor
            .extract(
                &world,
                VIEWPORT,
                CameraView::default(),
                &TextureBindings::new()
            )
            .is_err()
    );
    assert!(
        Arc::ptr_eq(first, &asset),
        "already extracted frames retain resources"
    );
}

#[test]
fn missing_model_diagnostic_names_the_logical_asset() {
    let world = world_from(&scene(MODELS));
    let error = SceneExtractor::new()
        .unwrap()
        .extract(
            &world,
            VIEWPORT,
            CameraView::default(),
            &TextureBindings::new(),
        )
        .unwrap_err();
    assert!(
        matches!(error, SceneExtractError::MissingModel(ref id) if id.as_str() == "models/test.glb")
    );
    assert!(error.to_string().contains("models/test.glb"));
}

#[test]
fn model_schema_validates_asset_paths_without_inventing_a_default() {
    let extractor = SceneExtractor::new().unwrap();
    assert!(
        extractor
            .components()
            .default_payload(ModelComponent::TYPE_NAME)
            .is_none()
    );
    let document = SceneDocument::from_json(&scene(
        r#",
        { "id": "bad", "components": { "sindri.model": { "asset": "../escape.glb" } } }"#,
    ))
    .unwrap();
    assert!(
        extractor
            .validate(&document, UnknownComponentPolicy::Reject)
            .is_err()
    );
    assert_eq!(
        referenced_models(&world_from(&scene(MODELS)))
            .into_iter()
            .collect::<Vec<_>>(),
        ["models/test.glb"]
    );
}

#[test]
fn disabled_models_are_discovered_for_loading_but_not_drawn() {
    let world = world_from(&scene(
        r#",
        { "id": "hidden", "disabled": true,
          "components": { "sindri.model": { "asset": "models/hidden.glb" } } }"#,
    ));
    assert_eq!(
        referenced_models(&world).into_iter().collect::<Vec<_>>(),
        ["models/hidden.glb"]
    );
    let frame = SceneExtractor::new()
        .unwrap()
        .extract(
            &world,
            VIEWPORT,
            CameraView::default(),
            &TextureBindings::new(),
        )
        .unwrap();
    assert!(frame.passes().is_empty());
}
