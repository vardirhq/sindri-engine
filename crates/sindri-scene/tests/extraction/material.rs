//! Per-entity factors cross extraction without changing shared model resources.

use crate::support::{VIEWPORT, scene, world_from};
use sindri_core::{AssetId, CommandBuffer, CommandHistory, SceneEntityId, WorldCommand};
use sindri_render::{FrameCommand, MaterialFactors, ModelData, RenderModel};
use sindri_scene::{CameraView, SceneExtractor, TextureBindings};
use std::sync::Arc;

#[test]
fn material_edits_undo_and_restore_mesh_and_shared_model_factors() {
    let mut world = world_from(&scene(
        r#",
        {"id":"mesh", "components":{"sindri.mesh":{"primitive":"cube","texture":"t"},
            "sindri.material":{"metallic":0.8,"roughness":0.2,"emissive":[0,0.5,1]}}},
        {"id":"model", "components":{"sindri.model":{"asset":"test.glb"}, "sindri.material":{"metallic":0.4}}},
        {"id":"plain", "components":{"sindri.model":{"asset":"test.glb"}}}"#,
    ));
    let asset = Arc::new(RenderModel::new(ModelData::default()).unwrap());
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.bind_model(AssetId::new("test.glb").unwrap(), Arc::clone(&asset));
    let bindings = TextureBindings::new();
    let before = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &bindings)
        .unwrap();
    assert_eq!(
        before.passes()[0].material,
        Some(MaterialFactors {
            metallic: 0.8,
            roughness: 0.2,
            emissive: [0.0, 0.5, 1.0],
            ..Default::default()
        })
    );
    assert_eq!(
        before.passes()[1].material,
        Some(MaterialFactors {
            metallic: 0.4,
            ..Default::default()
        })
    );
    assert_eq!(before.passes()[2].material, None);
    for pass in &before.passes()[1..] {
        let FrameCommand::Model { asset: bound, .. } = &pass.command else {
            panic!("model expected");
        };
        assert!(Arc::ptr_eq(bound, &asset));
    }
    let entity = world
        .entity_for_source_id(&SceneEntityId::new("mesh").unwrap())
        .unwrap();
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::RemoveComponent {
        entity,
        type_name: "sindri.material".into(),
    });
    let mut history = CommandHistory::default();
    history
        .apply(commands.into_transaction("Remove Material"), &mut world)
        .unwrap();
    let removed = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &bindings)
        .unwrap();
    assert_eq!(removed.passes()[0].material, None);
    history.undo(&mut world).unwrap();
    let restored = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &bindings)
        .unwrap();
    assert_eq!(restored.passes()[0].material, before.passes()[0].material);
}
