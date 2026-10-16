//! Inline geometry revisions survive draw-state changes and checked edits.
use super::support::{VIEWPORT, animated_bindings, scene, world_from};
use sindri_core::{
    CommandBuffer, CommandHistory, SceneEntityId, SpriteSheetDocument, WorldCommand,
};
use sindri_render::{FrameCommand, RenderMesh, TextureId};
use sindri_scene::{CameraView, SceneExtractor, TextureBindings};
use std::sync::Arc;

fn fixture() -> sindri_core::World {
    world_from(&scene(
        r#", {"id":"mesh", "transform_3d":{}, "components":{
        "sindri.mesh":{"primitive":"surface", "texture":"sheet.png#0", "surface":{
            "vertices":[[-1,-1,0],[1,-1,0],[0,1,0]],
            "uvs":[[0,0],[1,0],[0.5,1]], "indices":[0,1,2]}}
    }}"#,
    ))
}

fn geometry(
    extractor: &SceneExtractor,
    world: &sindri_core::World,
    textures: &TextureBindings,
) -> Arc<RenderMesh> {
    let frame = extractor
        .extract(world, VIEWPORT, CameraView::default(), textures)
        .unwrap();
    let FrameCommand::RetainedTexturedMesh { geometry, .. } = &frame.passes()[0].command else {
        panic!("retained surface expected");
    };
    Arc::clone(geometry)
}

#[test]
fn draw_state_reuses_geometry_but_atlas_layout_rebuilds_it() {
    let mut world = fixture();
    let extractor = SceneExtractor::new().unwrap();
    let mut textures = animated_bindings();
    let original = geometry(&extractor, &world, &textures);
    let entity = world
        .entity_for_source_id(&SceneEntityId::new("mesh").unwrap())
        .unwrap();
    assert!(Arc::ptr_eq(
        &original,
        &geometry(&extractor, &world, &textures)
    ));
    world
        .get_mut(entity)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position[0] = 4.0;
    world
        .get_mut(entity)
        .unwrap()
        .components
        .get_mut("sindri.mesh")
        .unwrap()["layer"] = 7.into();
    world.get_mut(entity).unwrap().components.insert(
        "sindri.material".into(),
        serde_json::json!({"metallic":0.5,"sidedness":"double"}),
    );
    assert!(Arc::ptr_eq(
        &original,
        &geometry(&extractor, &world, &textures)
    ));
    textures.bind("sheet.png", TextureId::new(2));
    assert!(Arc::ptr_eq(
        &original,
        &geometry(&extractor, &world, &textures)
    ));
    textures
        .bind_sheet("sheet.png", &SpriteSheetDocument::from_grid(4, 2))
        .unwrap();
    let changed = geometry(&extractor, &world, &textures);
    assert!(!Arc::ptr_eq(&original, &changed));
    assert_ne!(
        original.vertices()[1].uv.map(f32::to_bits),
        changed.vertices()[1].uv.map(f32::to_bits)
    );
}

#[test]
fn checked_edits_undo_and_old_frames_keep_independent_geometry() {
    let mut world = fixture();
    let extractor = SceneExtractor::new().unwrap();
    let textures = animated_bindings();
    let original = geometry(&extractor, &world, &textures);
    let entity = world
        .entity_for_source_id(&SceneEntityId::new("mesh").unwrap())
        .unwrap();
    let mut payload = world.get(entity).unwrap().components["sindri.mesh"].clone();
    payload["surface"]["vertices"][0][0] = (-2).into();
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetComponent {
        entity,
        type_name: "sindri.mesh".into(),
        payload,
    });
    let mut history = CommandHistory::default();
    history
        .apply(commands.into_transaction("Edit Surface"), &mut world)
        .unwrap();
    let changed = geometry(&extractor, &world, &textures);
    assert!(!Arc::ptr_eq(&original, &changed));
    assert_eq!(
        original.vertices()[0].position[0].to_bits(),
        (-1.0_f32).to_bits()
    );
    assert_eq!(
        changed.vertices()[0].position[0].to_bits(),
        (-2.0_f32).to_bits()
    );
    history.undo(&mut world).unwrap();
    assert_eq!(
        geometry(&extractor, &world, &textures)
            .vertices()
            .iter()
            .map(|v| v.position.map(f32::to_bits))
            .collect::<Vec<_>>(),
        original
            .vertices()
            .iter()
            .map(|v| v.position.map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    history.redo(&mut world).unwrap();
    assert_eq!(
        geometry(&extractor, &world, &textures)
            .vertices()
            .iter()
            .map(|v| v.position.map(f32::to_bits))
            .collect::<Vec<_>>(),
        changed
            .vertices()
            .iter()
            .map(|v| v.position.map(f32::to_bits))
            .collect::<Vec<_>>()
    );
}

#[test]
fn disabling_and_despawning_release_extractor_owners() {
    let mut world = fixture();
    let extractor = SceneExtractor::new().unwrap();
    let textures = animated_bindings();
    let entity = world
        .entity_for_source_id(&SceneEntityId::new("mesh").unwrap())
        .unwrap();
    let frame = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .unwrap();
    let FrameCommand::RetainedTexturedMesh {
        geometry: retained, ..
    } = &frame.passes()[0].command
    else {
        panic!()
    };
    let weak = Arc::downgrade(retained);
    world.get_mut(entity).unwrap().disabled = true;
    assert!(
        extractor
            .extract(&world, VIEWPORT, CameraView::default(), &textures)
            .unwrap()
            .passes()
            .is_empty()
    );
    assert!(
        weak.upgrade().is_some(),
        "old frame keeps its resource alive"
    );
    drop(frame);
    assert!(weak.upgrade().is_none());
    world.get_mut(entity).unwrap().disabled = false;
    let resource = Arc::downgrade(&geometry(&extractor, &world, &textures));
    world.despawn_recursive(entity).unwrap();
    extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .unwrap();
    assert!(resource.upgrade().is_none());
}

#[test]
fn surfaces_accept_indices_above_the_u16_range_and_reject_bad_triangles() {
    let mut world = fixture();
    let entity = world
        .entity_for_source_id(&SceneEntityId::new("mesh").unwrap())
        .unwrap();
    let payload = world
        .get_mut(entity)
        .unwrap()
        .components
        .get_mut("sindri.mesh")
        .unwrap();
    payload["surface"]["vertices"] = serde_json::json!(vec![[0.0; 3]; 65_537]);
    payload["surface"]["uvs"] = serde_json::json!(vec![[0.0; 2]; 65_537]);
    payload["surface"]["indices"] = serde_json::json!([65534, 65535, 65536]);
    let extractor = SceneExtractor::new().unwrap();
    let textures = animated_bindings();
    assert_eq!(
        geometry(&extractor, &world, &textures).indices(),
        &[65534, 65535, 65536]
    );
    world
        .get_mut(entity)
        .unwrap()
        .components
        .get_mut("sindri.mesh")
        .unwrap()["surface"]["indices"] = serde_json::json!([0, 1, 65537]);
    assert!(matches!(
        extractor.extract(&world, VIEWPORT, CameraView::default(), &textures),
        Err(sindri_scene::SceneExtractError::Mesh(
            sindri_render::MeshGeometryError::InvalidIndices
        ))
    ));
}

#[test]
fn sidedness_history_keeps_geometry_and_old_frame_policy_independent() {
    use sindri_render::MaterialSidedness;
    let mut world = fixture();
    let extractor = SceneExtractor::new().unwrap();
    let textures = animated_bindings();
    let original = geometry(&extractor, &world, &textures);
    let before = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .unwrap();
    let entity = world
        .entity_for_source_id(&SceneEntityId::new("mesh").unwrap())
        .unwrap();
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetComponent {
        entity,
        type_name: "sindri.material".into(),
        payload: serde_json::json!({"sidedness":"double"}),
    });
    let mut history = CommandHistory::default();
    history
        .apply(
            commands.into_transaction("Double-sided Surface"),
            &mut world,
        )
        .unwrap();
    for expected in [
        MaterialSidedness::Double,
        MaterialSidedness::Inherit,
        MaterialSidedness::Double,
    ] {
        let frame = extractor
            .extract(&world, VIEWPORT, CameraView::default(), &textures)
            .unwrap();
        assert_eq!(frame.passes()[0].sidedness, expected);
        assert_eq!(before.passes()[0].sidedness, MaterialSidedness::Inherit);
        let FrameCommand::RetainedTexturedMesh { geometry, .. } = &frame.passes()[0].command else {
            panic!()
        };
        assert!(Arc::ptr_eq(&original, geometry));
        if expected == MaterialSidedness::Double {
            history.undo(&mut world).unwrap();
        } else {
            history.redo(&mut world).unwrap();
        }
    }
}
