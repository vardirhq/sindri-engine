//! Translation follows body-local axes, with travel and force bounds.
use sindri_core::EntityId;
use sindri_physics::{
    Collider2d, PhysicsPose2d, PhysicsWorld2d, RigidBody2d, RigidBodyKind, SliderJoint2d,
    SliderSettings2d,
};
use std::time::Duration;
const STEP: Duration = Duration::from_nanos(16_666_667);
fn setup(rotation: f32) -> (PhysicsWorld2d, SliderJoint2d, EntityId) {
    let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    let first = EntityId::from_bits(1 << 32);
    let second = EntityId::from_bits(2 << 32);
    world
        .insert_body(
            first,
            RigidBody2d {
                kind: RigidBodyKind::Static,
                pose: PhysicsPose2d {
                    rotation,
                    ..PhysicsPose2d::default()
                },
                ..RigidBody2d::default()
            },
            &[Collider2d::circle(0.5)],
        )
        .unwrap();
    world
        .insert_body(
            second,
            RigidBody2d {
                pose: PhysicsPose2d {
                    rotation,
                    ..PhysicsPose2d::default()
                },
                ..RigidBody2d::default()
            },
            &[Collider2d::circle(0.5)],
        )
        .unwrap();
    (
        world,
        SliderJoint2d {
            first,
            second,
            settings: SliderSettings2d {
                motor_enabled: true,
                motor_velocity: 2.0,
                motor_max_force: 10.0,
                ..SliderSettings2d::default()
            },
        },
        EntityId::from_bits(3 << 32),
    )
}
#[test]
fn rotated_rail_preserves_axis_and_rotation_and_stops_at_limits() {
    let rotation = std::f32::consts::FRAC_PI_4;
    let (mut world, mut joint, owner) = setup(rotation);
    joint.settings.limits_enabled = true;
    joint.settings.lower_distance = -0.5;
    joint.settings.upper_distance = 1.0;
    for _ in 0..180 {
        world.set_slider_joint(owner, joint).unwrap();
        world.step(STEP).unwrap();
        let pose = world.pose(joint.second).unwrap();
        let along = pose.position[0] * rotation.cos() + pose.position[1] * rotation.sin();
        let across = -pose.position[0] * rotation.sin() + pose.position[1] * rotation.cos();
        assert!(across.abs() < 0.02);
        assert!((pose.rotation - rotation).abs() < 0.02);
        assert!((-0.53..=1.03).contains(&along), "travel {along}");
        assert_eq!(world.joint_count(), 1);
    }
    let pose = world.pose(joint.second).unwrap();
    assert!((pose.position[0].hypot(pose.position[1]) - 1.0).abs() < 0.03);
    joint.settings.motor_velocity = -2.0;
    world.set_slider_joint(owner, joint).unwrap();
    for _ in 0..180 {
        world.step(STEP).unwrap();
    }
    assert!(world.pose(joint.second).unwrap().position[0] < -0.3);
    assert!(world.remove_owned_joint(owner));
    assert_eq!(world.joint_count(), 0);
}
#[test]
fn slider_motor_force_cap_and_coast_brake_semantics_are_measured() {
    let (mut world, mut joint, owner) = setup(0.0);
    joint.settings.motor_max_force = 0.1;
    world.set_slider_joint(owner, joint).unwrap();
    for _ in 0..12 {
        world.step(STEP).unwrap();
    }
    let speed = world.linear_velocity(joint.second).unwrap()[0];
    // Unit density circle: mass = pi/4. The capped force acts for 0.2 seconds.
    assert!((speed - 0.1 * 0.2 / std::f32::consts::FRAC_PI_4).abs() < 0.002);
    joint.settings.motor_max_force = 10.0;
    world.set_slider_joint(owner, joint).unwrap();
    for _ in 0..180 {
        world.step(STEP).unwrap();
    }
    assert!(world.linear_velocity(joint.second).unwrap()[0] > 1.8);
    joint.settings.motor_enabled = false;
    world.set_slider_joint(owner, joint).unwrap();
    for _ in 0..60 {
        world.step(STEP).unwrap();
    }
    assert!(world.linear_velocity(joint.second).unwrap()[0] > 1.8);
    joint.settings.motor_enabled = true;
    joint.settings.motor_velocity = 0.0;
    world.set_slider_joint(owner, joint).unwrap();
    for _ in 0..240 {
        world.step(STEP).unwrap();
    }
    assert!(world.linear_velocity(joint.second).unwrap()[0].abs() < 0.03);
}
#[test]
fn invalid_slider_edits_preserve_the_existing_constraint() {
    let (mut world, joint, owner) = setup(0.0);
    world.set_slider_joint(owner, joint).unwrap();
    for settings in [
        SliderSettings2d {
            first_axis: [0.0, 0.0],
            ..joint.settings
        },
        SliderSettings2d {
            second_axis: [2.0, 0.0],
            ..joint.settings
        },
        SliderSettings2d {
            first_axis: [f32::NAN, 0.0],
            ..joint.settings
        },
        SliderSettings2d {
            motor_max_force: -1.0,
            ..joint.settings
        },
        SliderSettings2d {
            limits_enabled: true,
            lower_distance: 1.0,
            upper_distance: 0.0,
            ..joint.settings
        },
    ] {
        assert!(
            world
                .set_slider_joint(owner, SliderJoint2d { settings, ..joint })
                .is_err()
        );
        assert_eq!(world.joint_count(), 1);
    }
    for _ in 0..60 {
        world.step(STEP).unwrap();
    }
    assert!(world.linear_velocity(joint.second).unwrap()[0] > 1.0);
    assert!(world.remove(joint.second));
    assert_eq!(world.joint_count(), 0);
}

#[test]
fn distinct_local_axes_define_the_locked_relative_orientation() {
    let (mut world, mut joint, owner) = setup(0.0);
    joint.settings.second_axis = [0.0, 1.0];
    joint.settings.motor_enabled = false;
    world.set_slider_joint(owner, joint).unwrap();
    for _ in 0..120 {
        world.step(STEP).unwrap();
    }
    let pose = world.pose(joint.second).unwrap();
    assert!((pose.rotation + std::f32::consts::FRAC_PI_2).abs() < 0.03);
    assert!(pose.position[1].abs() < 0.02);
}

#[path = "sliders/position.rs"]
mod position;
