//! Hinges constrain anchors, bound rotation and drive with capped torque.

use sindri_core::EntityId;
use sindri_physics::{
    Collider2d, HingeJoint2d, HingeSettings2d, PhysicsPose2d, PhysicsWorld2d, RigidBody2d,
    RigidBodyKind,
};
use std::time::Duration;

const STEP: Duration = Duration::from_nanos(16_666_667);

fn world() -> (PhysicsWorld2d, HingeJoint2d, EntityId) {
    let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    let first = EntityId::from_bits(1 << 32);
    let second = EntityId::from_bits(2 << 32);
    let owner = EntityId::from_bits(3 << 32);
    world
        .insert_body(
            first,
            RigidBody2d {
                kind: RigidBodyKind::Static,
                ..RigidBody2d::default()
            },
            &[Collider2d::circle(0.1)],
        )
        .unwrap();
    world
        .insert_body(
            second,
            RigidBody2d {
                pose: PhysicsPose2d {
                    position: [1.0, 0.0],
                    rotation: 0.0,
                },
                ..RigidBody2d::default()
            },
            &[Collider2d::circle(0.5)],
        )
        .unwrap();
    (
        world,
        HingeJoint2d {
            first,
            second,
            settings: HingeSettings2d {
                second_anchor: [-1.0, 0.0],
                motor_enabled: true,
                motor_velocity: 2.0,
                motor_max_torque: 10.0,
                ..HingeSettings2d::default()
            },
        },
        owner,
    )
}

#[test]
fn motor_turns_around_local_anchors_and_edits_preserve_motion() {
    let (mut world, mut joint, owner) = world();
    for _ in 0..180 {
        world.set_hinge_joint(owner, joint).unwrap();
        world.step(STEP).unwrap();
        let pose = world.pose(joint.second).unwrap();
        assert!(
            (pose.position[0] - pose.rotation.cos()).hypot(pose.position[1] - pose.rotation.sin())
                < 0.035
        );
        assert_eq!(world.joint_count(), 1);
    }
    let before = world.angular_velocity(joint.second).unwrap();
    assert!(before > 1.5);
    joint.settings.motor_velocity = -2.0;
    world.set_hinge_joint(owner, joint).unwrap();
    assert!((world.angular_velocity(joint.second).unwrap() - before).abs() < 1e-5);
    for _ in 0..240 {
        world.step(STEP).unwrap();
    }
    assert!(world.angular_velocity(joint.second).unwrap() < -1.5);
    joint.settings.motor_enabled = false;
    world.set_hinge_joint(owner, joint).unwrap();
    for _ in 0..60 {
        world.step(STEP).unwrap();
    }
    assert!(
        world.angular_velocity(joint.second).unwrap() < -1.0,
        "disabling the motor coasts rather than brakes"
    );
    joint.settings.motor_enabled = true;
    joint.settings.motor_velocity = 0.0;
    world.set_hinge_joint(owner, joint).unwrap();
    for _ in 0..240 {
        world.step(STEP).unwrap();
    }
    assert!(
        world.angular_velocity(joint.second).unwrap().abs() < 0.05,
        "a powered zero-speed motor brakes rather than coasts"
    );
    assert!(world.remove(joint.first));
    assert_eq!(world.joint_count(), 0);
    assert!(!world.remove_owned_joint(owner));
}

#[test]
fn limits_stop_a_driven_hinge_and_low_torque_limits_acceleration() {
    let (mut bounded, mut joint, owner) = world();
    joint.settings.limits_enabled = true;
    joint.settings.lower_angle = -0.25;
    joint.settings.upper_angle = 0.5;
    bounded.set_hinge_joint(owner, joint).unwrap();
    for _ in 0..240 {
        bounded.step(STEP).unwrap();
        let angle = bounded.pose(joint.second).unwrap().rotation;
        assert!((-0.28..=0.53).contains(&angle), "angle {angle}");
    }
    assert!((bounded.pose(joint.second).unwrap().rotation - 0.5).abs() < 0.03);
    let (mut weak, mut joint, owner) = world();
    joint.settings.motor_max_torque = 0.001;
    weak.set_hinge_joint(owner, joint).unwrap();
    for _ in 0..12 {
        weak.step(STEP).unwrap();
    }
    let speed = weak.angular_velocity(joint.second).unwrap();
    assert!(speed > 0.0 && speed < 0.1, "uncapped motor speed {speed}");
}

#[test]
fn invalid_hinge_edits_leave_the_existing_constraint_untouched() {
    let (mut world, joint, owner) = world();
    world.set_hinge_joint(owner, joint).unwrap();
    for settings in [
        HingeSettings2d {
            first_anchor: [f32::NAN, 0.0],
            ..joint.settings
        },
        HingeSettings2d {
            motor_velocity: f32::INFINITY,
            ..joint.settings
        },
        HingeSettings2d {
            motor_max_torque: -1.0,
            ..joint.settings
        },
        HingeSettings2d {
            limits_enabled: true,
            lower_angle: 1.0,
            upper_angle: 0.0,
            ..joint.settings
        },
        HingeSettings2d {
            limits_enabled: true,
            upper_angle: 7.0,
            ..joint.settings
        },
    ] {
        assert!(
            world
                .set_hinge_joint(owner, HingeJoint2d { settings, ..joint })
                .is_err()
        );
        assert_eq!(world.joint_count(), 1);
    }
    assert!(
        world
            .set_hinge_joint(
                owner,
                HingeJoint2d {
                    second: joint.first,
                    ..joint
                }
            )
            .is_err()
    );
    assert!(
        world
            .set_hinge_joint(
                owner,
                HingeJoint2d {
                    second: EntityId::from_bits(9 << 32),
                    ..joint
                }
            )
            .is_err()
    );
    for _ in 0..180 {
        world.step(STEP).unwrap();
    }
    assert!(world.angular_velocity(joint.second).unwrap() > 1.5);
}

#[path = "hinges/position.rs"]
mod position;
