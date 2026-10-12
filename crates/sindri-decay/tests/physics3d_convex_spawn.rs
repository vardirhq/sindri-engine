//! Convex-only spawn-window requests share the ordered body-control queue.
#[path = "physics3d_controls/support.rs"]
mod support;

use decay_runtime::Value;
use serde_json::json;
use sindri_core::{AssetId, EntityId, SceneComponent};
use sindri_scene::{
    Collider3dComponent, ModelConvexCollider3dComponent, ModelConvexGeometry3d,
    RigidBody3dComponent,
};
use std::sync::Arc;
use support::{Fixture, near, reference};

fn hull(fixture: &mut Fixture, kind: &str) -> EntityId {
    let actor = fixture.actor(kind, false);
    let data = fixture.world.get_mut(actor).unwrap();
    data.components.remove(Collider3dComponent::TYPE_NAME);
    data.components.insert(
        ModelConvexCollider3dComponent::TYPE_NAME.into(),
        json!({"asset":"models/prop.glb"}),
    );
    fixture
        .physics
        .bind_model_convex_collision(AssetId::new("models/prop.glb").unwrap(), geometry());
    actor
}
fn geometry() -> Arc<ModelConvexGeometry3d> {
    let points = [-0.5, 0.5]
        .into_iter()
        .flat_map(|x| {
            [-0.5, 0.5]
                .into_iter()
                .flat_map(move |y| [-0.5, 0.5].into_iter().map(move |z| [x, y, z]))
        })
        .collect();
    Arc::new(ModelConvexGeometry3d::new(points).unwrap())
}
#[test]
fn queued_hull_motion_uses_scaled_mass_and_replays_without_reapplying() {
    let mut fixture = Fixture::new();
    let actor = hull(&mut fixture, "dynamic");
    fixture
        .world
        .get_mut(actor)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .scale = [2.0, 1.0, 1.0];
    let authored = fixture.world.get(actor).unwrap().components.clone();
    for (call, value) in [
        ("set_velocity", [1.0, 0.0, 0.0]),
        ("apply_impulse", [2.0, 0.0, 0.0]),
        ("set_angular_velocity", [0.0, 1.0, 0.0]),
    ] {
        fixture
            .call(actor, call, &[reference(actor), Value::Vec3(value)], true)
            .unwrap();
    }
    assert_eq!(
        fixture
            .call(actor, "velocity", &[reference(actor)], true)
            .unwrap(),
        Value::Vec3([1.0, 0.0, 0.0])
    );
    let replay = fixture.physics.clone();
    let initial = fixture.world.clone();
    fixture.step();
    near(
        fixture.physics.world().linear_velocity(actor).unwrap(),
        [2.0, 0.0, 0.0],
    );
    assert!((fixture.physics.world().mass(actor).unwrap() - 2.0).abs() < 0.001);
    assert_eq!(fixture.world.get(actor).unwrap().components, authored);
    fixture.step();
    let pose = fixture.world.world_transform(actor).unwrap();
    assert!((pose.position[0] - 0.04).abs() < 0.001);
    fixture.physics = replay;
    fixture.world = initial;
    fixture.step();
    fixture.step();
    assert_eq!(fixture.world.world_transform(actor), Some(pose));
}

#[test]
fn invalid_hull_authoring_preserves_the_last_valid_pending_request() {
    let mut fixture = Fixture::new();
    let actor = hull(&mut fixture, "dynamic");
    fixture
        .call(
            actor,
            "set_velocity",
            &[reference(actor), Value::Vec3([1.0, 0.0, 0.0])],
            true,
        )
        .unwrap();
    let original = fixture.world.get(actor).unwrap().clone();
    let mut invalid_body = original.components[RigidBody3dComponent::TYPE_NAME].clone();
    invalid_body["linear_damping"] = json!(-1.0);
    for (name, value) in [
        (RigidBody3dComponent::TYPE_NAME, invalid_body),
        (
            ModelConvexCollider3dComponent::TYPE_NAME,
            json!({"asset":"../escape.glb"}),
        ),
        (
            ModelConvexCollider3dComponent::TYPE_NAME,
            json!({"asset":"models/prop.glb", "friction":-1.0}),
        ),
        (
            ModelConvexCollider3dComponent::TYPE_NAME,
            json!({"asset":"models/prop.glb", "restitution":2.0}),
        ),
        (Collider3dComponent::TYPE_NAME, json!({"pieces":[]})),
        ("sindri.physics3d.character", json!({})),
        ("sindri.physics3d.voxel_collider", json!({})),
        (
            "sindri.physics3d.model_collider",
            json!({"asset":"models/prop.glb"}),
        ),
        ("sindri.physics2d.collider", json!({})),
    ] {
        fixture
            .world
            .get_mut(actor)
            .unwrap()
            .components
            .insert(name.into(), value);
        assert!(
            fixture
                .call(
                    actor,
                    "set_velocity",
                    &[reference(actor), Value::Vec3([99.0; 3])],
                    true
                )
                .is_err()
        );
        fixture.world.get_mut(actor).unwrap().clone_from(&original);
    }
    for scale in [[-1.0; 3], [0.0; 3], [f32::INFINITY; 3]] {
        fixture
            .world
            .get_mut(actor)
            .unwrap()
            .transform_3d
            .as_mut()
            .unwrap()
            .scale = scale;
        assert!(
            fixture
                .call(actor, "velocity", &[reference(actor)], true)
                .is_err()
        );
        fixture.world.get_mut(actor).unwrap().clone_from(&original);
    }
    fixture.step();
    near(
        fixture.physics.world().linear_velocity(actor).unwrap(),
        [1.0, 0.0, 0.0],
    );
}

#[test]
fn hull_spawn_reads_respect_locks_and_impulses_reject_kinematic_bodies() {
    let mut fixture = Fixture::new();
    let actor = hull(&mut fixture, "kinematic_velocity");
    fixture
        .world
        .get_mut(actor)
        .unwrap()
        .components
        .get_mut(RigidBody3dComponent::TYPE_NAME)
        .unwrap()["lock_rotation"] = json!(true);
    fixture
        .call(
            actor,
            "set_angular_velocity",
            &[reference(actor), Value::Vec3([1.0; 3])],
            true,
        )
        .unwrap();
    assert_eq!(
        fixture
            .call(actor, "angular_velocity", &[reference(actor)], true)
            .unwrap(),
        Value::Vec3([0.0; 3])
    );
    assert!(
        fixture
            .call(
                actor,
                "apply_impulse",
                &[reference(actor), Value::Vec3([1.0; 3])],
                true
            )
            .is_err()
    );
    fixture
        .call(
            actor,
            "set_velocity",
            &[reference(actor), Value::Vec3([1.0, 0.0, 0.0])],
            true,
        )
        .unwrap();
    fixture.step();
    near(
        fixture.physics.world().angular_velocity(actor).unwrap(),
        [0.0; 3],
    );
    near(
        fixture.physics.world().linear_velocity(actor).unwrap(),
        [1.0, 0.0, 0.0],
    );
}

#[test]
fn hull_and_primitive_impulses_use_combined_mass() {
    let mut fixture = Fixture::new();
    let actor = hull(&mut fixture, "dynamic");
    let piece = fixture
        .extractor
        .components()
        .default_payload(Collider3dComponent::TYPE_NAME)
        .unwrap()
        .clone();
    fixture
        .world
        .get_mut(actor)
        .unwrap()
        .components
        .insert(Collider3dComponent::TYPE_NAME.into(), piece);
    fixture
        .call(
            actor,
            "apply_impulse",
            &[reference(actor), Value::Vec3([2.0, 0.0, 0.0])],
            true,
        )
        .unwrap();
    fixture.step();
    near(
        fixture.physics.world().linear_velocity(actor).unwrap(),
        [1.0, 0.0, 0.0],
    );
    assert!((fixture.physics.world().mass(actor).unwrap() - 2.0).abs() < 0.001);
}

#[test]
fn missing_geometry_fails_synchronization_and_retains_queued_controls() {
    let mut fixture = Fixture::new();
    let actor = hull(&mut fixture, "dynamic");
    let id = AssetId::new("models/prop.glb").unwrap();
    fixture.physics.unbind_model_convex_collision(&id);
    fixture
        .call(
            actor,
            "set_velocity",
            &[reference(actor), Value::Vec3([1.0, 0.0, 0.0])],
            true,
        )
        .unwrap();
    assert!(
        fixture
            .physics
            .step(
                &mut fixture.world,
                fixture.extractor.components(),
                std::time::Duration::from_millis(10)
            )
            .is_err()
    );
    assert!(!fixture.physics.world().contains(actor));
    fixture.physics.bind_model_convex_collision(id, geometry());
    fixture.step();
    near(
        fixture.physics.world().linear_velocity(actor).unwrap(),
        [1.0, 0.0, 0.0],
    );
}
