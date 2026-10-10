//! Scene lifecycle and next-step ownership, independent of game input policy.
use super::*;
use crate::{RigidBody3dComponent, SceneExtractor, ScenePhysics3d};
use serde_json::json;
use sindri_core::{EntityData, SceneComponent, Transform3D};
use sindri_physics::{ColliderShape3d, PhysicsPose3d, RaycastFilter3d, RigidBodyKind};
use std::time::Duration;

const STEP: Duration = Duration::from_millis(10);

fn spawn(world: &mut World, position: [f32; 3], collider: Collider3d, character: bool) -> EntityId {
    let mut data = EntityData {
        transform_3d: Some(Transform3D {
            position,
            ..Transform3D::default()
        }),
        ..EntityData::default()
    };
    data.components
        .insert(Collider3dComponent::TYPE_NAME.into(), json!(collider));
    if character {
        data.components
            .insert(Character3dComponent::TYPE_NAME.into(), json!({}));
    }
    world.spawn(data)
}

fn step(physics: &mut ScenePhysics3d, world: &mut World) {
    physics
        .step(world, SceneExtractor::new().unwrap().components(), STEP)
        .unwrap();
}

fn near(actual: [f32; 3], expected: [f32; 3]) {
    for (a, b) in actual.into_iter().zip(expected) {
        assert!((a - b).abs() < 0.02, "{actual:?} != {expected:?}");
    }
}

#[test]
fn pending_input_replaces_once_and_has_no_solver_gravity_or_velocity() {
    let mut world = World::default();
    let actor = spawn(&mut world, [0.0; 3], Collider3d::sphere(0.5), true);
    let payload = world.get(actor).unwrap().components.clone();
    let mut physics = ScenePhysics3d::new([0.0, -9.81, 0.0]).unwrap();
    assert!(physics.character_motion(actor).is_none());
    physics
        .character_requests()
        .move_character(actor, [99.0; 3], false)
        .unwrap();
    physics
        .character_requests()
        .move_character(actor, [1.0, 2.0, 3.0], false)
        .unwrap();
    assert!(
        physics
            .character_requests()
            .move_character(actor, [f32::NAN; 3], false)
            .is_err()
    );
    assert!(
        physics
            .character_requests()
            .move_character(actor, [f32::MAX; 3], false)
            .is_err()
    );
    let mut replay = physics.clone();
    let mut replay_world = world.clone();
    step(&mut physics, &mut world);
    step(&mut replay, &mut replay_world);
    near(
        world.world_transform(actor).unwrap().position,
        [1.0, 2.0, 3.0],
    );
    assert_eq!(
        physics.character_motion(actor),
        replay.character_motion(actor)
    );
    near(
        physics.character_motion(actor).unwrap().translation,
        [1.0, 2.0, 3.0],
    );
    assert_eq!(
        physics.world().body_kind(actor).unwrap(),
        RigidBodyKind::KinematicVelocity
    );
    physics
        .world_mut()
        .set_linear_velocity(actor, [30.0; 3])
        .unwrap();
    physics
        .world_mut()
        .set_angular_velocity(actor, [30.0; 3])
        .unwrap();
    step(&mut physics, &mut world);
    near(
        world.world_transform(actor).unwrap().position,
        [1.0, 2.0, 3.0],
    );
    near(
        physics.character_motion(actor).unwrap().translation,
        [0.0; 3],
    );
    near(physics.world().linear_velocity(actor).unwrap(), [0.0; 3]);
    assert_eq!(world.get(actor).unwrap().components, payload);
}

#[test]
fn scaled_offset_probe_blocks_and_writes_world_motion_under_a_rotated_parent() {
    let mut world = World::default();
    let parent = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [10.0, 0.0, 0.0],
            rotation: glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2).to_array(),
            scale: [2.0; 3],
            ..Transform3D::default()
        }),
        ..EntityData::default()
    });
    let actor = spawn(
        &mut world,
        [0.0; 3],
        Collider3d {
            offset: [0.0, 0.0, 0.5],
            ..Collider3d::cuboid([0.25; 3])
        },
        true,
    );
    world.set_parent(actor, Some(parent)).unwrap();
    let rotation = world.world_transform(actor).unwrap().rotation;
    let wall = spawn(
        &mut world,
        [14.0, 0.0, 0.0],
        Collider3d::cuboid([0.5, 4.0, 4.0]),
        false,
    );
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    physics
        .character_requests()
        .move_character(actor, [10.0, 0.0, 1.0], false)
        .unwrap();
    step(&mut physics, &mut world);
    let position = world.world_transform(actor).unwrap().position;
    near(position, [11.99, 0.0, 1.0]);
    near(world.world_transform(actor).unwrap().scale, [2.0; 3]);
    let actual_rotation = world.world_transform(actor).unwrap().rotation;
    assert!(
        actual_rotation
            .into_iter()
            .zip(rotation)
            .all(|(a, b)| (a - b).abs() < 1.0e-6)
    );
    assert!(
        physics
            .character_motion(actor)
            .unwrap()
            .collisions
            .iter()
            .any(|c| c.hit.entity == wall)
    );
    near(physics.world().pose(actor).unwrap().position, position);
}

#[test]
fn exactly_one_scaled_solid_uses_its_filter_and_ignores_sensors() {
    let mut world = World::default();
    let actor = spawn(&mut world, [0.0; 3], Collider3d::sphere(0.5), true);
    let sensor = Collider3d {
        sensor: true,
        ..Collider3d::sphere(10.0)
    };
    let solid = Collider3d {
        layers: sindri_physics::CollisionLayers {
            memberships: 1,
            filter: 2,
        },
        ..Collider3d::sphere(0.5)
    };
    world.get_mut(actor).unwrap().components.insert(
        Collider3dComponent::TYPE_NAME.into(),
        json!({"pieces": [sensor, solid]}),
    );
    spawn(
        &mut world,
        [2.0, 0.0, 0.0],
        Collider3d {
            layers: sindri_physics::CollisionLayers {
                memberships: 4,
                filter: u32::MAX,
            },
            ..Collider3d::cuboid([0.5; 3])
        },
        false,
    );
    spawn(&mut world, [3.0, 0.0, 0.0], sensor, false);
    let wall = spawn(
        &mut world,
        [5.0, 0.0, 0.0],
        Collider3d {
            layers: sindri_physics::CollisionLayers {
                memberships: 2,
                filter: u32::MAX,
            },
            ..Collider3d::cuboid([0.5; 3])
        },
        false,
    );
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    physics
        .character_requests()
        .move_character(actor, [10.0, 0.0, 0.0], false)
        .unwrap();
    step(&mut physics, &mut world);
    near(
        world.world_transform(actor).unwrap().position,
        [3.99, 0.0, 0.0],
    );
    assert!(
        physics
            .character_motion(actor)
            .unwrap()
            .collisions
            .iter()
            .all(|c| c.hit.entity == wall)
    );
}

#[test]
fn invalid_authored_batch_and_timestep_preserve_body_and_pending_input() {
    let mut world = World::default();
    let actor = spawn(&mut world, [0.0; 3], Collider3d::sphere(0.5), true);
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    step(&mut physics, &mut world);
    physics
        .character_requests()
        .move_character(actor, [1.0, 2.0, 3.0], false)
        .unwrap();
    let registry = SceneExtractor::new().unwrap();
    assert!(
        physics
            .step(&mut world, registry.components(), Duration::ZERO)
            .is_err()
    );
    for case in 0..6 {
        let bad = spawn(&mut world, [5.0; 3], Collider3d::sphere(0.5), true);
        let data = world.get_mut(bad).unwrap();
        match case {
            0 => {
                data.components.insert(
                    RigidBody3dComponent::TYPE_NAME.into(),
                    json!(sindri_physics::RigidBody3d::default()),
                );
            }
            1 => {
                data.transform_3d = None;
            }
            2 => {
                data.components.remove(Collider3dComponent::TYPE_NAME);
            }
            3 => {
                data.components.insert(
                    Collider3dComponent::TYPE_NAME.into(),
                    json!({"pieces": [Collider3d::sphere(0.5), Collider3d::sphere(0.5)]}),
                );
            }
            4 => {
                data.components.insert(
                    Character3dComponent::TYPE_NAME.into(),
                    json!({"skin": -1.0}),
                );
            }
            _ => {
                data.transform_3d.as_mut().unwrap().z_locked = true;
            }
        }
        assert!(
            physics
                .step(&mut world, registry.components(), STEP)
                .is_err(),
            "case {case}"
        );
        assert!(!physics.world().contains(bad));
        near(physics.world().pose(actor).unwrap().position, [0.0; 3]);
        world.despawn_recursive(bad).unwrap();
    }
    step(&mut physics, &mut world);
    near(
        world.world_transform(actor).unwrap().position,
        [1.0, 2.0, 3.0],
    );
}

#[test]
fn inactivity_removal_and_rebuilds_discard_stale_state_but_keep_same_step_input() {
    let mut world = World::default();
    let actor = spawn(&mut world, [0.0; 3], Collider3d::sphere(0.5), true);
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    step(&mut physics, &mut world);
    let snapshot = physics.character_motion(actor).unwrap().clone();
    physics
        .character_requests()
        .move_character(actor, [1.0, 0.0, 0.0], false)
        .unwrap();
    world
        .get_mut(actor)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [10.0; 3];
    world.get_mut(actor).unwrap().components.insert(
        Collider3dComponent::TYPE_NAME.into(),
        json!(Collider3d::sphere(0.25)),
    );
    step(&mut physics, &mut world);
    near(
        world.world_transform(actor).unwrap().position,
        [11.0, 10.0, 10.0],
    );
    near(snapshot.translation, [0.0; 3]);
    world.get_mut(actor).unwrap().disabled = true;
    physics
        .character_requests()
        .move_character(actor, [99.0; 3], false)
        .unwrap();
    step(&mut physics, &mut world);
    assert!(physics.character_motion(actor).is_none());
    assert!(!physics.world().contains(actor));
    world.get_mut(actor).unwrap().disabled = false;
    step(&mut physics, &mut world);
    near(
        world.world_transform(actor).unwrap().position,
        [11.0, 10.0, 10.0],
    );
    world
        .get_mut(actor)
        .unwrap()
        .components
        .remove(Character3dComponent::TYPE_NAME);
    physics
        .character_requests()
        .move_character(actor, [99.0; 3], false)
        .unwrap();
    step(&mut physics, &mut world);
    assert!(physics.character_motion(actor).is_none());
    assert_eq!(
        physics.world().body_kind(actor).unwrap(),
        RigidBodyKind::Static
    );
    world.despawn_recursive(actor).unwrap();
    step(&mut physics, &mut world);
    assert!(physics.world().is_empty());
}

#[test]
fn current_solved_platforms_block_without_implicit_carry_and_caches_are_borrowed() {
    let mut world = World::default();
    let platform = spawn(
        &mut world,
        [0.0, -0.5, 0.0],
        Collider3d::cuboid([5.0, 0.5, 5.0]),
        false,
    );
    world.get_mut(platform).unwrap().components.insert(
        RigidBody3dComponent::TYPE_NAME.into(),
        json!(sindri_physics::RigidBody3d {
            kind: RigidBodyKind::KinematicPosition,
            ..sindri_physics::RigidBody3d::default()
        }),
    );
    let actor = spawn(&mut world, [0.0, 0.51, 0.0], Collider3d::sphere(0.5), true);
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    step(&mut physics, &mut world);
    assert!(physics.character_motion(actor).unwrap().grounded);
    physics
        .world_mut()
        .set_kinematic_target(
            platform,
            PhysicsPose3d {
                position: [1.0, -0.5, 0.0],
                ..PhysicsPose3d::default()
            },
        )
        .unwrap();
    step(&mut physics, &mut world);
    near(
        world.world_transform(actor).unwrap().position,
        [0.0, 0.51, 0.0],
    );
    let (_, _, requests, motions) = physics.for_scripts_with_characters();
    let cached = motions.get(actor).unwrap().clone();
    requests
        .move_character(actor, [1.0, 0.0, 0.0], false)
        .unwrap();
    assert_eq!(motions.get(actor), Some(&cached));
    step(&mut physics, &mut world);
    near(
        world.world_transform(actor).unwrap().position,
        [1.0, 0.51, 0.0],
    );
    let overlap = physics
        .world()
        .overlap(
            ColliderShape3d::Sphere { radius: 0.1 },
            PhysicsPose3d {
                position: [1.0, 0.51, 0.0],
                ..PhysicsPose3d::default()
            },
            RaycastFilter3d::default(),
        )
        .unwrap();
    assert!(
        overlap.contains(&actor),
        "query index sees applied movement immediately"
    );
}
