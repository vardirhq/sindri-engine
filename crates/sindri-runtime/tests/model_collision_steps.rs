//! Decoded GLB collision enters the shared Session without renderer ownership.
use serde_json::json;
use sindri_assets::{AssetBytes, AssetDecoder, ModelAsset, ModelAssetDecoder};
use sindri_core::{AssetId, EntityData, EntityId, SceneComponent, Transform3D, World};
use sindri_decay::ScriptSources;
use sindri_platform::InputState;
use sindri_runtime::Session;
use sindri_scene::{
    Character3dComponent, Collider3dComponent, ModelCollider3dComponent, SceneExtractor,
};

fn id() -> AssetId {
    AssetId::new("models/static.glb").unwrap()
}
fn fixture() -> ModelAsset {
    ModelAssetDecoder
        .decode(AssetBytes::new(
            id(),
            include_bytes!("../../sindri-assets/tests/fixtures/models/static-model.glb").to_vec(),
        ))
        .unwrap()
}
fn setup() -> (Session, World, EntityId) {
    let extractor = SceneExtractor::new().unwrap();
    let session = Session::with_sources(extractor.components().clone(), ScriptSources::new());
    let mut world = World::default();
    let owner = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [(
            ModelCollider3dComponent::TYPE_NAME.into(),
            json!({"asset": id()}),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    (session, world, owner)
}
fn step(session: &mut Session, world: &mut World) {
    session
        .step(world, &InputState::default(), (640.0, 360.0), 0.01)
        .unwrap();
}
// The filter type belongs to physics; this integration test keeps runtime
// dependencies at the scene seam instead of adding a direct physics dependency.
#[allow(clippy::default_trait_access)]
fn hit(session: &Session, x: f32) -> Option<EntityId> {
    session
        .physics3d()
        .world()
        .raycast([x, 3.25, 10.0], [0.0, 0.0, -1.0], 20.0, Default::default())
        .unwrap()
        .map(|hit| hit.entity)
}

#[test]
fn glb_triangles_block_character_motion_and_checkpoint_replays() {
    let (mut session, mut world, owner) = setup();
    session.bind_model_collision(id(), &fixture()).unwrap();
    let mut collider = SceneExtractor::new()
        .unwrap()
        .components()
        .default_payload(Collider3dComponent::TYPE_NAME)
        .unwrap()
        .clone();
    collider["pieces"][0]["shape"]["half_extents"] = json!([0.1, 0.1, 0.1]);
    let actor = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [1.25, 3.25, 5.0],
            ..Transform3D::default()
        }),
        components: [
            (Character3dComponent::TYPE_NAME.into(), json!({})),
            (Collider3dComponent::TYPE_NAME.into(), collider),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    session
        .character_requests3d()
        .move_character(actor, [0.0, 0.0, -5.0], false)
        .unwrap();
    let checkpoint = session.checkpoint();
    let initial = world.clone();
    step(&mut session, &mut world);
    let pose = world.world_transform(actor).unwrap();
    assert!(
        pose.position[2] > 3.09 && pose.position[2] < 3.2,
        "{pose:?}"
    );
    let motion = session.physics3d().character_motion(actor).unwrap().clone();
    assert!(
        motion
            .movement
            .collisions
            .iter()
            .any(|collision| collision.hit.entity == owner)
    );
    session.restore(&checkpoint);
    world = initial;
    step(&mut session, &mut world);
    assert_eq!(world.world_transform(actor), Some(pose));
    assert_eq!(session.physics3d().character_motion(actor), Some(&motion));
}

#[test]
fn restore_keeps_latest_host_geometry_and_failed_binding_keeps_previous() {
    let (mut session, mut world, owner) = setup();
    let mut asset = fixture();
    session.bind_model_collision(id(), &asset).unwrap();
    step(&mut session, &mut world);
    assert_eq!(hit(&session, 1.25), Some(owner));
    let checkpoint = session.checkpoint();
    let initial = world.clone();
    asset.nodes[asset.roots[0]].local_transform[3][0] += 10.0;
    session.bind_model_collision(id(), &asset).unwrap();
    session.restore(&checkpoint);
    world = initial;
    step(&mut session, &mut world);
    assert!(hit(&session, 1.25).is_none());
    assert_eq!(hit(&session, 11.25), Some(owner));
    asset.nodes[asset.roots[0]].local_transform[0][0] = 0.0;
    assert!(session.bind_model_collision(id(), &asset).is_err());
    step(&mut session, &mut world);
    assert_eq!(hit(&session, 11.25), Some(owner));
    session.unbind_model_collision(&id());
    assert!(
        session
            .step(&mut world, &InputState::default(), (640.0, 360.0), 0.01)
            .is_err()
    );
    assert_eq!(hit(&session, 11.25), Some(owner));
}
