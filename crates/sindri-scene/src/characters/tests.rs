//! Scene fixed-step, transform and support ownership regression coverage.
mod lifecycle;
mod support;

use crate::{Character2dComponent, RigidBody2dComponent};
use serde_json::json;
use sindri_core::SceneComponent;
use sindri_physics::PhysicsPose2d;
use support::{Fixture, STEP, near, position};

#[test]
fn first_step_materializes_and_moves_a_spawned_controller_once() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 1.0]);
    let wall = f.box_collider([2.0, 0.0], [0.1, 3.0]);
    f.queue(hero, [4.0, 0.0], false);
    f.step();
    near(position(&f.world, hero)[0], 1.39);
    let motion = f.physics.character_motion(hero).unwrap();
    assert_eq!(motion.slide.collisions[0].entity, wall);
    f.step();
    near(position(&f.world, hero)[0], 1.39);
    assert!(
        f.physics
            .character_motion(hero)
            .unwrap()
            .slide
            .collisions
            .is_empty()
    );
    assert!(
        !f.world
            .get(hero)
            .unwrap()
            .components
            .contains_key(RigidBody2dComponent::TYPE_NAME)
    );
}

#[test]
fn requests_replace_and_reject_bad_values_without_losing_valid_input() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 1.0]);
    f.queue(hero, [0.2, 0.0], false);
    f.queue(hero, [0.4, 0.0], false);
    assert!(
        f.physics
            .character_requests()
            .move_character(hero, [f32::NAN, 0.0], false)
            .is_err()
    );
    assert!(
        f.physics
            .character_requests()
            .move_character(hero, [f32::MAX; 2], false)
            .is_err()
    );
    assert!(
        f.physics
            .step(&mut f.world, &f.components, std::time::Duration::ZERO)
            .is_err()
    );
    f.step();
    near(position(&f.world, hero)[0], 0.4);
    f.step();
    near(position(&f.world, hero)[0], 0.4);
}

#[test]
fn platform_motion_from_the_current_solve_is_carried_once_without_input() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 0.61]);
    let platform = f.box_collider([0.0, 0.0], [2.0, 0.1]);
    f.body(platform, "kinematic_velocity", [1.0, 0.0]);
    f.step();
    assert!(f.physics.character_motion(hero).unwrap().grounded);
    f.step();
    near(position(&f.world, hero)[0], 2.0 * STEP.as_secs_f32());
    let motion = f.physics.character_motion(hero).unwrap();
    near(
        motion.platform.as_ref().unwrap().motion.translation[0],
        STEP.as_secs_f32(),
    );
    f.step();
    near(position(&f.world, hero)[0], 3.0 * STEP.as_secs_f32());
}

#[test]
fn rotation_point_carry_uses_the_current_synchronized_support_pose() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 0.61]);
    let platform = f.box_collider([0.0, 0.0], [2.0, 0.1]);
    f.body(platform, "kinematic_velocity", [0.0; 2]);
    f.step();
    f.physics
        .world_mut()
        .set_angular_velocity(platform, 0.6)
        .unwrap();
    f.step();
    let motion = f.physics.character_motion(hero).unwrap();
    let carry = motion.platform.as_ref().unwrap();
    near(carry.current_pose.rotation, 0.6 * STEP.as_secs_f32());
    assert!(carry.motion.translation[0] < 0.0);
    assert!(motion.grounded);
}

#[test]
fn gravity_and_external_solver_velocity_cannot_add_controller_movement() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 2.0]);
    f.step();
    f.physics.world_mut().set_gravity([0.0, -100.0]).unwrap();
    f.physics
        .world_mut()
        .set_linear_velocity(hero, [100.0, -100.0])
        .unwrap();
    f.physics
        .world_mut()
        .set_angular_velocity(hero, 100.0)
        .unwrap();
    f.queue(hero, [0.2, -0.1], false);
    f.step();
    near(position(&f.world, hero)[0], 0.2);
    near(position(&f.world, hero)[1], 1.9);
    near(f.physics.world().pose(hero).unwrap().rotation, 0.0);
}

#[test]
fn timed_drop_cancels_carry_preserves_floors_and_expires_or_cancels() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 0.61]);
    let platform = f.box_collider([0.0, 0.0], [2.0, 0.1]);
    f.body(platform, "kinematic_velocity", [1.0, 0.0]);
    f.world
        .get_mut(platform)
        .unwrap()
        .components
        .insert("sindri.physics2d.one_way".into(), json!({}));
    let floor = f.box_collider([0.0, -2.0], [4.0, 0.1]);
    f.step();
    f.physics
        .character_requests()
        .drop_through(hero, 0.001)
        .unwrap();
    f.queue(hero, [0.0, -4.0], false);
    f.step();
    let motion = f.physics.character_motion(hero).unwrap();
    assert!(motion.platform.is_none());
    assert_eq!(motion.ground.hit.unwrap().entity, floor);
    near(position(&f.world, hero)[0], STEP.as_secs_f32());
    // The duration expired after the preceding pass. Teleport above the plank.
    f.set_position(hero, [0.0, 1.0]);
    f.queue(hero, [0.0, -1.0], false);
    f.step();
    assert_eq!(
        f.physics
            .character_motion(hero)
            .unwrap()
            .ground
            .hit
            .unwrap()
            .entity,
        platform
    );
    f.physics
        .character_requests()
        .drop_through(hero, 1.0)
        .unwrap();
    assert!(
        f.physics
            .character_requests()
            .drop_through(hero, -1.0)
            .is_err()
    );
    f.physics
        .character_requests()
        .drop_through(hero, 0.0)
        .unwrap();
    f.step();
    assert!(f.physics.character_motion(hero).unwrap().grounded);
}

#[test]
fn offset_and_rotated_probe_move_the_body_origin_by_the_same_translation() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 1.0]);
    let collider = f
        .world
        .get_mut(hero)
        .unwrap()
        .components
        .get_mut("sindri.physics2d.collider")
        .unwrap();
    collider["offset"] = json!([0.3, 0.0]);
    collider["rotation"] = json!(0.2);
    f.box_collider([2.0, 0.0], [0.1, 3.0]);
    f.queue(hero, [4.0, 0.0], false);
    f.step();
    near(position(&f.world, hero)[0], 1.09);
    near(position(&f.world, hero)[1], 1.0);
}

#[test]
fn controller_writeback_preserves_parent_space_z_scale_and_rotation() {
    let mut f = Fixture::new();
    let parent = f.empty([10.0, 0.0]);
    let hero = f.character([0.0, 1.0]);
    f.world.set_parent(hero, Some(parent)).unwrap();
    let transform = f
        .world
        .get_mut(hero)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap();
    transform.position[2] = 3.0;
    transform.scale = [2.0, 3.0, 4.0];
    transform.set_rotation_z_radians(0.25);
    f.queue(hero, [0.2, 0.0], false);
    f.step();
    near(position(&f.world, hero)[0], 10.2);
    let local = f.world.get(hero).unwrap().transform_3d.unwrap();
    near(local.position[0], 0.2);
    near(local.position[2], 3.0);
    for (actual, expected) in local.scale.into_iter().zip([2.0, 3.0, 4.0]) {
        near(actual, expected);
    }
    near(local.rotation_z_radians(), 0.25);
}

#[test]
fn queued_snap_permission_controls_landing_and_ascent_still_wins() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 0.9]);
    f.world
        .get_mut(hero)
        .unwrap()
        .components
        .get_mut(Character2dComponent::TYPE_NAME)
        .unwrap()["snap_distance"] = json!(0.5);
    f.box_collider([0.0, 0.0], [2.0, 0.1]);
    f.queue(hero, [0.0; 2], false);
    f.step();
    near(position(&f.world, hero)[1], 0.9);
    assert!(!f.physics.character_motion(hero).unwrap().grounded);
    f.queue(hero, [0.0; 2], true);
    f.step();
    near(position(&f.world, hero)[1], 0.61);
    f.queue(hero, [0.0, 0.1], true);
    f.step();
    near(position(&f.world, hero)[1], 0.71);
    assert!(!f.physics.character_motion(hero).unwrap().grounded);
}

#[test]
fn sensor_events_observe_the_controller_pose_at_the_next_solve() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 1.0]);
    let sensor = f.box_collider([2.0, 1.0], [0.2, 0.2]);
    f.world
        .get_mut(sensor)
        .unwrap()
        .components
        .get_mut("sindri.physics2d.collider")
        .unwrap()["sensor"] = json!(true);
    f.queue(hero, [2.0, 0.0], false);
    f.step();
    assert!(f.physics.events().is_empty());
    f.step();
    assert!(
        f.physics
            .events()
            .iter()
            .any(|event| event.first == hero && event.second == sensor
                || event.first == sensor && event.second == hero)
    );
}

#[test]
fn immediate_queries_see_applied_controller_collider_poses() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 1.0]);
    f.queue(hero, [2.0, 0.0], false);
    f.step();
    let hit = f
        .physics
        .world()
        .shape_cast(
            sindri_physics::ColliderShape2d::Circle { radius: 0.1 },
            PhysicsPose2d {
                position: [4.0, 1.0],
                rotation: 0.0,
            },
            [-1.0, 0.0],
            4.0,
            sindri_physics::RaycastFilter2d::default(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(hit.entity, hero);
    near(hit.distance, 1.4);
}

#[test]
fn initial_support_is_seeded_before_vertical_platforms_advance() {
    for velocity in [-1.0, 1.0] {
        let mut f = Fixture::new();
        let hero = f.character([0.0, 0.61]);
        let platform = f.box_collider([0.0, 0.0], [2.0, 0.1]);
        f.body(platform, "kinematic_velocity", [0.0, velocity]);
        f.step();
        near(
            position(&f.world, hero)[1],
            0.61 + velocity * STEP.as_secs_f32(),
        );
        let motion = f.physics.character_motion(hero).unwrap();
        assert!(motion.grounded && !motion.slide.started_penetrating);
        assert_eq!(motion.platform.as_ref().unwrap().entity, platform);
    }
}

#[test]
fn initial_overlap_is_reported_without_guessing_a_recovery() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 0.0]);
    f.box_collider([0.0, 0.0], [1.0, 1.0]);
    f.queue(hero, [2.0, 0.0], false);
    f.step();
    let motion = f.physics.character_motion(hero).unwrap();
    assert!(motion.slide.started_penetrating && !motion.grounded);
    near(position(&f.world, hero)[0], 0.0);
}

#[test]
fn controller_filters_use_the_solid_probe_mask_and_ignore_own_sensors() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 1.0]);
    let pieces = f.world.get(hero).unwrap().components["sindri.physics2d.collider"].clone();
    let mut sensor = pieces.clone();
    sensor["sensor"] = json!(true);
    f.world.get_mut(hero).unwrap().components.insert(
        "sindri.physics2d.collider".into(),
        json!({"pieces": [pieces, sensor]}),
    );
    let wall = f.box_collider([2.0, 0.0], [0.1, 3.0]);
    f.world
        .get_mut(wall)
        .unwrap()
        .components
        .get_mut("sindri.physics2d.collider")
        .unwrap()["layers"]["memberships"] = json!(2);
    f.queue(hero, [4.0, 0.0], false);
    f.step();
    near(position(&f.world, hero)[0], 4.0);
    assert!(
        f.physics
            .character_motion(hero)
            .unwrap()
            .slide
            .collisions
            .is_empty()
    );
}
