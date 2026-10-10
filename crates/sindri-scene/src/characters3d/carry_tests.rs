//! Shared-step carry, classified support and lifecycle regressions.
use super::*;
use crate::{SceneExtractor, ScenePhysics3d};
use serde_json::json;
use sindri_core::{EntityData, SceneComponent, Transform3D};
use sindri_physics::{PhysicsPose3d, RigidBody3d, RigidBodyKind};
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
fn fixture(height: f32) -> (World, ScenePhysics3d, EntityId, EntityId) {
    let mut world = World::default();
    let platform = spawn(
        &mut world,
        [0.0, -0.5, 0.0],
        Collider3d::cuboid([5.0, 0.5, 5.0]),
        false,
    );
    world.get_mut(platform).unwrap().components.insert(
        RigidBody3dComponent::TYPE_NAME.into(),
        json!(RigidBody3d {
            kind: RigidBodyKind::KinematicPosition,
            ..RigidBody3d::default()
        }),
    );
    let actor = spawn(
        &mut world,
        [0.0, height, 0.0],
        Collider3d::sphere(0.5),
        true,
    );
    (
        world,
        ScenePhysics3d::new([0.0; 3]).unwrap(),
        platform,
        actor,
    )
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
fn stationary_riders_follow_solved_motion_once_and_clones_replay_it() {
    let (mut world, mut physics, platform, actor) = fixture(0.51);
    step(&mut physics, &mut world);
    physics
        .world_mut()
        .set_kinematic_target(
            platform,
            PhysicsPose3d {
                position: [1.0, -0.3, 2.0],
                ..PhysicsPose3d::default()
            },
        )
        .unwrap();
    let mut copy = physics.clone();
    let mut copy_world = world.clone();
    step(&mut physics, &mut world);
    step(&mut copy, &mut copy_world);
    near(
        world.world_transform(actor).unwrap().position,
        [1.0, 0.71, 2.0],
    );
    assert_eq!(
        physics.character_motion(actor),
        copy.character_motion(actor)
    );
    let motion = physics.character_motion(actor).unwrap().clone();
    assert!(motion.grounded);
    near(
        motion.platform.as_ref().unwrap().motion.translation,
        [1.0, 0.2, 2.0],
    );
    near(motion.movement.translation, [0.0; 3]);
    step(&mut physics, &mut world);
    near(
        physics.character_motion(actor).unwrap().translation,
        [0.0; 3],
    );
    physics
        .world_mut()
        .set_kinematic_target(
            platform,
            PhysicsPose3d {
                position: [0.0, -0.5, 0.0],
                ..PhysicsPose3d::default()
            },
        )
        .unwrap();
    step(&mut physics, &mut world);
    near(
        world.world_transform(actor).unwrap().position,
        [0.0, 0.51, 0.0],
    );
    physics
        .character_requests()
        .move_character(actor, [0.0, 1.0, 0.0], false)
        .unwrap();
    step(&mut physics, &mut world);
    assert!(!physics.character_motion(actor).unwrap().grounded);
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
        [0.0, 1.51, 0.0],
    );
    assert!(physics.character_motion(actor).unwrap().platform.is_none());
}

#[test]
fn parented_rider_rotates_around_support_without_double_carry_or_probe_rotation() {
    let (mut world, mut physics, platform, actor) = fixture(1.01);
    world
        .get_mut(actor)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position[0] = 2.0;
    world.set_parent(actor, Some(platform)).unwrap();
    step(&mut physics, &mut world);
    let rotation = world.world_transform(actor).unwrap().rotation;
    physics
        .world_mut()
        .set_kinematic_target(
            platform,
            PhysicsPose3d {
                position: [1.0, -0.5, 0.0],
                rotation: glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2).to_array(),
            },
        )
        .unwrap();
    step(&mut physics, &mut world);
    near(
        world.world_transform(actor).unwrap().position,
        [1.0, 0.51, -2.0],
    );
    assert!(
        world
            .world_transform(actor)
            .unwrap()
            .rotation
            .into_iter()
            .zip(rotation)
            .all(|(a, b)| (a - b).abs() < 1e-6)
    );
    step(&mut physics, &mut world);
    near(
        physics.character_motion(actor).unwrap().translation,
        [0.0; 3],
    );
    near(
        world.world_transform(actor).unwrap().position,
        [1.0, 0.51, -2.0],
    );
}

#[test]
fn collision_clipped_carry_reports_separate_hits_and_does_not_retry_old_travel() {
    let (mut world, mut physics, platform, actor) = fixture(0.51);
    let wall = spawn(
        &mut world,
        [2.0, 2.0, 0.0],
        Collider3d::cuboid([0.25, 2.0, 2.0]),
        false,
    );
    step(&mut physics, &mut world);
    physics
        .world_mut()
        .set_kinematic_target(
            platform,
            PhysicsPose3d {
                position: [3.0, -0.5, 0.0],
                ..PhysicsPose3d::default()
            },
        )
        .unwrap();
    step(&mut physics, &mut world);
    let motion = physics.character_motion(actor).unwrap();
    near(motion.translation, [1.24, 0.0, 0.0]);
    assert!(
        motion
            .platform
            .as_ref()
            .unwrap()
            .motion
            .collisions
            .iter()
            .any(|c| c.hit.entity == wall)
    );
    assert!(motion.movement.collisions.is_empty());
    step(&mut physics, &mut world);
    near(
        physics.character_motion(actor).unwrap().translation,
        [0.0; 3],
    );
}

#[test]
fn lifecycle_changes_never_reuse_prior_support_motion() {
    for case in 0..5 {
        let (mut world, mut physics, platform, actor) = fixture(0.51);
        step(&mut physics, &mut world);
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
        match case {
            0 => {
                world.despawn_recursive(platform).unwrap();
            }
            1 => {
                world.get_mut(platform).unwrap().disabled = true;
            }
            2 => {
                world.get_mut(platform).unwrap().components.insert(
                    Collider3dComponent::TYPE_NAME.into(),
                    json!(Collider3d {
                        sensor: true,
                        ..Collider3d::cuboid([5.0, 0.5, 5.0])
                    }),
                );
            }
            3 => {
                world.get_mut(actor).unwrap().components.insert(
                    Character3dComponent::TYPE_NAME.into(),
                    json!({"carry_platforms": false}),
                );
            }
            _ => {
                world
                    .get_mut(actor)
                    .unwrap()
                    .transform_3d
                    .as_mut()
                    .unwrap()
                    .position = [20.0, 2.0, 0.0];
            }
        }
        step(&mut physics, &mut world);
        let motion = physics.character_motion(actor).unwrap();
        assert!(motion.platform.is_none(), "case {case}");
        near(motion.translation, [0.0; 3]);
    }
}

#[test]
fn raw_ground_prediction_never_seeds_platform_carry() {
    let (mut world, mut physics, platform, actor) = fixture(0.54);
    step(&mut physics, &mut world);
    let motion = physics.character_motion(actor).unwrap();
    assert!(motion.movement.grounded && !motion.grounded);
    physics
        .world_mut()
        .set_kinematic_target(
            platform,
            PhysicsPose3d {
                position: [2.0, -0.5, 0.0],
                ..PhysicsPose3d::default()
            },
        )
        .unwrap();
    step(&mut physics, &mut world);
    assert!(physics.character_motion(actor).unwrap().platform.is_none());
    near(
        world.world_transform(actor).unwrap().position,
        [0.0, 0.54, 0.0],
    );
}

#[test]
fn flat_authoring_round_trips_carry_policy_and_rejects_invalid_geometry() {
    for (payload, expected) in [
        (json!({}), true),
        (json!({"carry_platforms":false,"skin":0.02}), false),
    ] {
        let component: Character3dComponent = serde_json::from_value(payload).unwrap();
        assert_eq!(component.carry_platforms, expected);
        let written = serde_json::to_value(component).unwrap();
        assert_eq!(written["carry_platforms"], json!(expected));
        assert!(written.get("movement").is_none());
        assert_eq!(
            serde_json::from_value::<Character3dComponent>(written).unwrap(),
            component
        );
    }
    assert!(serde_json::from_value::<Character3dComponent>(json!({"skin":-1.0})).is_err());
}
