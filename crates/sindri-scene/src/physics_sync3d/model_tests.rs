use super::tests::{components, spawn};
use super::*;
use serde_json::json;
use sindri_core::{AssetId, EntityData, SceneComponent, Transform3D};
use sindri_physics::RaycastFilter3d;
use std::sync::Arc;

fn floor(height: f32) -> Arc<crate::ModelCollisionGeometry3d> {
    Arc::new(
        crate::ModelCollisionGeometry3d::new(
            vec![
                [-5.0, height, -5.0],
                [-5.0, height, 5.0],
                [5.0, height, 5.0],
                [5.0, height, -5.0],
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        )
        .unwrap(),
    )
}
fn id() -> AssetId {
    AssetId::new("models/floor.glb").unwrap()
}
fn model(world: &mut World) -> EntityId {
    world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [(
            crate::ModelCollider3dComponent::TYPE_NAME.into(),
            json!({"asset": id()}),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    })
}
fn ray(physics: &ScenePhysics3d, x: f32) -> Option<sindri_physics::RayHit3d> {
    physics
        .world()
        .raycast(
            [x, 10.0, 0.0],
            [0.0, -1.0, 0.0],
            20.0,
            RaycastFilter3d::default(),
        )
        .unwrap()
}
fn step(physics: &mut ScenePhysics3d, world: &mut World) -> Result<(), PhysicsSyncError> {
    physics.step(world, &components(), Duration::from_secs_f32(1.0 / 60.0))
}

#[test]
fn model_floor_supports_a_dynamic_body_and_rebinding_rebuilds_once() {
    let mut world = World::default();
    let owner = model(&mut world);
    let actor = spawn(
        &mut world,
        [0.0, 2.0, 0.0],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.5),
    );
    let mut physics = ScenePhysics3d::new([0.0, -9.81, 0.0]).unwrap();
    physics.models.bind(id(), floor(0.0));
    for _ in 0..180 {
        step(&mut physics, &mut world).unwrap();
    }
    assert!((world.world_transform(actor).unwrap().position[1] - 0.5).abs() < 0.02);
    assert_eq!(ray(&physics, 2.0).unwrap().entity, owner);
    physics.models.bind(id(), floor(1.0));
    step(&mut physics, &mut world).unwrap();
    assert!((ray(&physics, 2.0).unwrap().point[1] - 1.0).abs() < 0.001);
    let held = physics.registered[&owner].model.clone().unwrap();
    step(&mut physics, &mut world).unwrap();
    assert!(held == physics.registered[&owner].model.clone().unwrap());
}

#[test]
fn inherited_scale_teleports_rotation_and_lifecycle_use_current_geometry() {
    let mut world = World::default();
    let parent = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [20.0, 0.0, 0.0],
            scale: [2.0, 3.0, 4.0],
            ..Transform3D::default()
        }),
        ..EntityData::default()
    });
    let owner = model(&mut world);
    world.get_mut(owner).unwrap().parent = Some(parent);
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    physics.models.bind(id(), floor(1.0));
    step(&mut physics, &mut world).unwrap();
    assert!(ray(&physics, 0.0).is_none());
    assert!((ray(&physics, 29.0).unwrap().point[1] - 3.0).abs() < 0.001);
    world
        .get_mut(parent)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position[0] = 40.0;
    step(&mut physics, &mut world).unwrap();
    assert!(ray(&physics, 29.0).is_none());
    assert!(ray(&physics, 49.0).is_some());
    let half = std::f32::consts::FRAC_PI_4;
    world
        .get_mut(parent)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .rotation = [half.sin(), 0.0, 0.0, half.cos()];
    step(&mut physics, &mut world).unwrap();
    assert!(
        physics
            .world()
            .raycast(
                [40.0, 0.0, 10.0],
                [0.0, 0.0, -1.0],
                20.0,
                RaycastFilter3d::default()
            )
            .unwrap()
            .is_some()
    );
    world.get_mut(owner).unwrap().disabled = true;
    step(&mut physics, &mut world).unwrap();
    assert!(!physics.world().contains(owner));
    world.get_mut(owner).unwrap().disabled = false;
    step(&mut physics, &mut world).unwrap();
    assert!(physics.world().contains(owner));
    world.get_mut(owner).unwrap().components.clear();
    step(&mut physics, &mut world).unwrap();
    assert!(!physics.world().contains(owner));
}

#[test]
fn missing_binding_invalid_scale_properties_and_conflicts_are_atomic() {
    let mut world = World::default();
    let owner = model(&mut world);
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    assert!(
        matches!(step(&mut physics, &mut world), Err(PhysicsSyncError::MissingModelCollision(entity, _)) if entity == owner)
    );
    physics.models.bind(id(), floor(0.0));
    step(&mut physics, &mut world).unwrap();
    for scale in [[-1.0, 1.0, 1.0], [0.0, 1.0, 1.0], [f32::MAX; 3]] {
        world
            .get_mut(owner)
            .unwrap()
            .transform_3d
            .as_mut()
            .unwrap()
            .scale = scale;
        assert!(step(&mut physics, &mut world).is_err());
        assert!(ray(&physics, 0.0).is_some());
    }
    world
        .get_mut(owner)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .scale = [1.0; 3];
    world
        .get_mut(owner)
        .unwrap()
        .components
        .get_mut(crate::ModelCollider3dComponent::TYPE_NAME)
        .unwrap()["restitution"] = json!(2.0);
    assert!(step(&mut physics, &mut world).is_err());
    assert!(ray(&physics, 0.0).is_some());
    world
        .get_mut(owner)
        .unwrap()
        .components
        .get_mut(crate::ModelCollider3dComponent::TYPE_NAME)
        .unwrap()["restitution"] = json!(0.0);
    world.get_mut(owner).unwrap().components.insert(
        Collider3dComponent::TYPE_NAME.into(),
        json!(Collider3d::sphere(1.0)),
    );
    assert!(matches!(
        step(&mut physics, &mut world),
        Err(PhysicsSyncError::InvalidModelCollider(..))
    ));
    world
        .get_mut(owner)
        .unwrap()
        .components
        .remove(Collider3dComponent::TYPE_NAME);
    physics.models.unbind(&id());
    assert!(step(&mut physics, &mut world).is_err());
    assert!(ray(&physics, 0.0).is_some());
}

#[test]
fn model_to_primitive_and_back_transitions_share_body_ownership() {
    let mut world = World::default();
    let owner = model(&mut world);
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    physics.models.bind(id(), floor(0.0));
    step(&mut physics, &mut world).unwrap();
    world.get_mut(owner).unwrap().components.clear();
    world.get_mut(owner).unwrap().components.insert(
        Collider3dComponent::TYPE_NAME.into(),
        json!(Collider3d::sphere(1.0)),
    );
    step(&mut physics, &mut world).unwrap();
    assert!((ray(&physics, 0.0).unwrap().point[1] - 1.0).abs() < 0.001);
    world.get_mut(owner).unwrap().components.clear();
    world.get_mut(owner).unwrap().components.insert(
        crate::ModelCollider3dComponent::TYPE_NAME.into(),
        json!({"asset": id()}),
    );
    step(&mut physics, &mut world).unwrap();
    assert!(ray(&physics, 0.0).unwrap().point[1].abs() < 0.001);
}

#[test]
fn model_schema_requires_an_asset_and_dimension_conflicts_precede_binding() {
    let registry = components();
    let name = crate::ModelCollider3dComponent::TYPE_NAME;
    assert!(registry.default_payload(name).is_none());
    assert!(registry.validate_payload(name, &json!({})).is_err());
    assert!(
        registry
            .validate_payload(name, &json!({"asset": "../escape.glb"}))
            .is_err()
    );
    registry
        .validate_payload(name, &json!({"asset": id()}))
        .unwrap();
    let mut world = World::default();
    let owner = model(&mut world);
    world.get_mut(owner).unwrap().components.insert(
        crate::Collider2dComponent::TYPE_NAME.into(),
        registry
            .default_payload(crate::Collider2dComponent::TYPE_NAME)
            .unwrap()
            .clone(),
    );
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    assert!(
        matches!(step(&mut physics, &mut world), Err(PhysicsSyncError::ConflictingDimensions(entity)) if entity == owner)
    );
    assert!(physics.world().is_empty());
}
