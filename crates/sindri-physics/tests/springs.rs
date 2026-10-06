//! Force-based springs pull and push towards rest length, with damping.
use sindri_core::EntityId;
use sindri_physics::{
    Collider2d, PhysicsPose2d, PhysicsWorld2d, RigidBody2d, RigidBodyKind, SpringJoint2d,
    SpringSettings2d,
};
use std::time::Duration;
const STEP: Duration = Duration::from_nanos(16_666_667);
fn setup(distance: f32, damping: f32) -> (PhysicsWorld2d, SpringJoint2d, EntityId) {
    let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    let first = EntityId::from_bits(1 << 32);
    let second = EntityId::from_bits(2 << 32);
    world
        .insert_body(
            first,
            RigidBody2d {
                kind: RigidBodyKind::Static,
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
                    position: [distance, 0.0],
                    rotation: 0.0,
                },
                ..RigidBody2d::default()
            },
            &[Collider2d::circle(0.5)],
        )
        .unwrap();
    (
        world,
        SpringJoint2d {
            first,
            second,
            settings: SpringSettings2d {
                rest_length: 1.0,
                stiffness: 10.0,
                damping,
                ..SpringSettings2d::default()
            },
        },
        EntityId::from_bits(3 << 32),
    )
}
#[test]
fn damped_spring_relaxes_from_extension_and_compression_and_retunes_live() {
    for distance in [0.5, 2.0] {
        let (mut world, mut joint, owner) = setup(distance, 4.0);
        for _ in 0..300 {
            world.set_spring_joint(owner, joint).unwrap();
            world.step(STEP).unwrap();
            assert_eq!(world.joint_count(), 1);
        }
        assert!((world.pose(joint.second).unwrap().position[0] - 1.0).abs() < 0.05);
        let velocity = world.linear_velocity(joint.second).unwrap();
        joint.settings.rest_length = 1.5;
        world.set_spring_joint(owner, joint).unwrap();
        let retained = world.linear_velocity(joint.second).unwrap();
        assert!(
            (retained[0] - velocity[0]).abs() < 1e-6 && (retained[1] - velocity[1]).abs() < 1e-6
        );
        for _ in 0..300 {
            world.step(STEP).unwrap();
        }
        assert!((world.pose(joint.second).unwrap().position[0] - 1.5).abs() < 0.05);
        world.remove(joint.first);
        assert_eq!(world.joint_count(), 0);
    }
}
#[test]
fn damping_reduces_oscillation_and_invalid_edits_leave_spring_live() {
    let (mut damped, joint, owner) = setup(2.0, 4.0);
    let (mut undamped, undamped_joint, _) = setup(2.0, 0.0);
    damped.set_spring_joint(owner, joint).unwrap();
    undamped.set_spring_joint(owner, undamped_joint).unwrap();
    for settings in [
        SpringSettings2d {
            rest_length: 0.0,
            ..joint.settings
        },
        SpringSettings2d {
            stiffness: -1.0,
            ..joint.settings
        },
        SpringSettings2d {
            damping: -1.0,
            ..joint.settings
        },
        SpringSettings2d {
            second_anchor: [f32::INFINITY, 0.0],
            ..joint.settings
        },
    ] {
        assert!(
            damped
                .set_spring_joint(owner, SpringJoint2d { settings, ..joint })
                .is_err()
        );
        assert_eq!(damped.joint_count(), 1);
    }
    let mut damped_excursion = 0.0_f32;
    let mut undamped_excursion = 0.0_f32;
    for frame in 0..300 {
        damped.step(STEP).unwrap();
        undamped.step(STEP).unwrap();
        if frame > 180 {
            damped_excursion =
                damped_excursion.max((damped.pose(joint.second).unwrap().position[0] - 1.0).abs());
            undamped_excursion = undamped_excursion
                .max((undamped.pose(joint.second).unwrap().position[0] - 1.0).abs());
        }
    }
    assert!(undamped_excursion > 0.2);
    assert!(damped_excursion < undamped_excursion * 0.2);
}

#[test]
fn force_based_spring_supports_weight_with_hooke_extension() {
    let (mut world, joint, owner) = setup(1.0, 4.0);
    world.set_gravity([0.0, -10.0]).unwrap();
    world
        .move_to(
            joint.second,
            PhysicsPose2d {
                position: [0.0, -1.0],
                rotation: 0.0,
            },
        )
        .unwrap();
    world.set_spring_joint(owner, joint).unwrap();
    for _ in 0..600 {
        world.step(STEP).unwrap();
    }
    let expected = joint.settings.rest_length
        + world.mass(joint.second).unwrap() * 10.0 / joint.settings.stiffness;
    let pose = world.pose(joint.second).unwrap();
    assert!(
        (pose.position[1] + expected).abs() < 0.05,
        "force-based spring length {} differs from weight support {expected}",
        -pose.position[1]
    );
}
