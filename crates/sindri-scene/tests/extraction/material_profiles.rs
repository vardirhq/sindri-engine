//! Reusable values change drawn state without replacing inline/model geometry.
use crate::support::{VIEWPORT, scene, world_from};
use serde_json::json;
use sindri_core::{AssetId, CommandBuffer, CommandHistory, SceneEntityId, WorldCommand};
use sindri_render::{FrameCommand, MaterialSidedness, ModelData, RenderModel};
use sindri_scene::{
    CameraView, RenderMaterialSources, SceneExtractor, TextureBindings, render_material_template,
};
use std::sync::Arc;

#[test]
fn shared_profiles_reload_across_geometry_and_undo_preserves_older_frames() {
    let mut world = world_from(&scene(
        r#",
      {"id":"cube","components":{"sindri.mesh":{"primitive":"cube","texture":"t"},"sindri.material":{"profile":"gold.profile"}}},
      {"id":"surface","components":{"sindri.mesh":{"primitive":"surface","texture":"t","surface":{"vertices":[[0,0,0],[1,0,0],[0,1,0]],"uvs":[[0,0],[1,0],[0,1]],"indices":[0,1,2]}},"sindri.material":{"profile":"gold.profile"}}},
      {"id":"model","components":{"sindri.model":{"asset":"prop.glb"},"sindri.material":{"profile":"gold.profile"}}}"#,
    ));
    let mut extractor = SceneExtractor::new().unwrap();
    let model = Arc::new(RenderModel::new(ModelData::default()).unwrap());
    extractor.bind_model(AssetId::new("prop.glb").unwrap(), Arc::clone(&model));
    let mut profile = render_material_template();
    profile.values.insert("metallic".into(), json!(0.8));
    extractor
        .set_materials(RenderMaterialSources::from_profiles([("gold.profile", &profile)]).unwrap());
    let textures = TextureBindings::new();
    let before = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .unwrap();
    assert_eq!(before.passes().len(), 3);
    let material = before.passes()[0].material.unwrap();
    assert!(before.passes().iter().all(|p| p.material == Some(material)));
    profile.values.insert("sidedness".into(), json!("double"));
    profile.values.insert("emissive".into(), json!([0, 1, 0]));
    extractor
        .set_materials(RenderMaterialSources::from_profiles([("gold.profile", &profile)]).unwrap());
    let after = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .unwrap();
    for (old, new) in before.passes().iter().zip(after.passes()) {
        assert_eq!(old.sidedness, MaterialSidedness::Inherit);
        assert_eq!(new.sidedness, MaterialSidedness::Double);
        assert_eq!(old.material, Some(material));
        assert!(
            new.material
                .unwrap()
                .emissive
                .iter()
                .zip([0.0, 1.0, 0.0])
                .all(|(a, b)| (a - b).abs() < f32::EPSILON)
        );
        match (&old.command, &new.command) {
            (
                FrameCommand::RetainedTexturedMesh { geometry: a, .. },
                FrameCommand::RetainedTexturedMesh { geometry: b, .. },
            ) => assert!(Arc::ptr_eq(a, b)),
            (FrameCommand::Model { asset: a, .. }, FrameCommand::Model { asset: b, .. }) => {
                assert!(Arc::ptr_eq(a, b));
            }
            _ => {}
        }
    }
    let entity = world
        .entity_for_source_id(&SceneEntityId::new("surface").unwrap())
        .unwrap();
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::RemoveComponent {
        entity,
        type_name: "sindri.material".into(),
    });
    let mut history = CommandHistory::default();
    history
        .apply(
            commands.into_transaction("Remove shared material"),
            &mut world,
        )
        .unwrap();
    let removed = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .unwrap();
    assert_eq!(removed.passes()[1].material, None);
    history.undo(&mut world).unwrap();
    assert_eq!(
        extractor
            .extract(&world, VIEWPORT, CameraView::default(), &textures)
            .unwrap()
            .passes()[1]
            .material,
        after.passes()[1].material
    );
    history.redo(&mut world).unwrap();
    assert_eq!(
        extractor
            .extract(&world, VIEWPORT, CameraView::default(), &textures)
            .unwrap()
            .passes()[1]
            .material,
        None
    );
}

#[test]
fn missing_profiles_fail_strict_extraction_and_are_reported_in_tolerant_views() {
    let world = world_from(&scene(
        r#",{"id":"cube","components":{"sindri.mesh":{"primitive":"cube","texture":"t"},"sindri.material":{"profile":"missing.profile"}}}"#,
    ));
    let mut extractor = SceneExtractor::new().unwrap();
    let textures = TextureBindings::new();
    assert!(
        extractor
            .extract(&world, VIEWPORT, CameraView::default(), &textures)
            .unwrap_err()
            .to_string()
            .contains("missing.profile")
    );
    extractor.tolerate_invalid_components();
    let frame = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .unwrap();
    assert!(frame.passes().is_empty());
    assert!(!extractor.problems().is_empty());
}
