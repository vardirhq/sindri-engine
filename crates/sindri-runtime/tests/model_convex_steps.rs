//! Actual GLB hierarchy binding, live bodies and checkpoint host replacement.
use serde_json::json;
use sindri_assets::{AssetBytes, AssetDecoder, ModelAsset, ModelAssetDecoder};
use sindri_core::{AssetId, EntityData, SceneComponent, Transform3D, World};
use sindri_decay::ScriptSources;
use sindri_platform::InputState;
use sindri_runtime::Session;
use sindri_scene::{ModelConvexCollider3dComponent, RigidBody3dComponent, SceneExtractor};

fn id() -> AssetId {
    AssetId::new("models/prop.glb").unwrap()
}
fn fixture() -> ModelAsset {
    let mut asset = ModelAssetDecoder
        .decode(AssetBytes::new(
            id(),
            include_bytes!("../../sindri-assets/tests/fixtures/models/static-model.glb").to_vec(),
        ))
        .unwrap();
    // The source fixture's instances lie in one plane. Reorient one actual
    // decoded node so its selected hierarchy is volumetric without new bytes.
    asset.nodes[2].local_transform[0] = [0.0, 0.0, -1.0, 0.0];
    asset.nodes[2].local_transform[2] = [1.0, 0.0, 0.0, 0.0];
    asset
}
fn step(session: &mut Session, world: &mut World) {
    session
        .step(world, &InputState::default(), (640.0, 360.0), 0.01)
        .unwrap();
}
// Physics owns the filter type; keep this runtime test at the scene seam
// without adding a direct runtime dependency on the physics crate.
#[allow(clippy::default_trait_access)]
fn hit(session: &Session, x: f32) -> bool {
    session
        .physics3d()
        .world()
        .raycast([x, 3.25, 10.0], [0.0, 0.0, -1.0], 20.0, Default::default())
        .unwrap()
        .is_some()
}
#[test]
fn decoded_hull_moves_replays_and_failed_replacement_is_atomic() {
    let registry = SceneExtractor::new().unwrap().components().clone();
    let mut body = registry
        .default_payload(RigidBody3dComponent::TYPE_NAME)
        .unwrap()
        .clone();
    body["linear_velocity"] = json!([1.0, 0.0, 0.0]);
    let mut world = World::default();
    let actor = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [
            (
                ModelConvexCollider3dComponent::TYPE_NAME.into(),
                json!({"asset": id()}),
            ),
            (RigidBody3dComponent::TYPE_NAME.into(), body),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut session = Session::with_sources(registry, ScriptSources::new());
    let mut asset = fixture();
    session.bind_model_convex_collision(id(), &asset).unwrap();
    step(&mut session, &mut world);
    let checkpoint = session.checkpoint();
    let initial = world.clone();
    for _ in 0..20 {
        step(&mut session, &mut world);
    }
    let pose = world.world_transform(actor).unwrap();
    assert!((pose.position[0] - 0.21).abs() < 0.001);
    session.restore(&checkpoint);
    world = initial.clone();
    for _ in 0..20 {
        step(&mut session, &mut world);
    }
    assert_eq!(world.world_transform(actor), Some(pose));
    asset.nodes[asset.roots[0]].local_transform[3][0] += 10.0;
    session.bind_model_convex_collision(id(), &asset).unwrap();
    session.restore(&checkpoint);
    world = initial;
    step(&mut session, &mut world);
    // Host replacement survives restoring the old solver and shifts local hull.
    assert!(hit(&session, 11.3));
    assert!(!hit(&session, 1.3));
    let planar = ModelAssetDecoder
        .decode(AssetBytes::new(
            id(),
            include_bytes!("../../sindri-assets/tests/fixtures/models/static-model.glb").to_vec(),
        ))
        .unwrap();
    assert!(session.bind_model_convex_collision(id(), &planar).is_err());
    step(&mut session, &mut world);
    assert!(hit(&session, 11.3));
    session.unbind_model_convex_collision(&id());
    assert!(
        session
            .step(&mut world, &InputState::default(), (640.0, 360.0), 0.01)
            .is_err()
    );
    assert!(hit(&session, 11.3));
}
