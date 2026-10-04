//! Support-side contacts, CCD ascent and timed drop-through lifecycle.

use sindri_core::EntityId;
use sindri_physics::{
    Collider2d, OneWay2d, PhysicsPose2d, PhysicsWorld2d, RigidBody2d, RigidBodyKind,
};
use std::time::Duration;

const STEP: Duration = Duration::from_nanos(16_666_667);
fn entity(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}

fn setup(kind: RigidBodyKind, rotation: f32) -> PhysicsWorld2d {
    let mut world = PhysicsWorld2d::new([0.0, -10.0]).unwrap();
    world
        .insert_body(
            entity(0),
            RigidBody2d {
                kind,
                pose: PhysicsPose2d {
                    position: [0.0, 0.0],
                    rotation,
                },
                ..RigidBody2d::default()
            },
            &[Collider2d::rectangle([2.0, 0.1])],
        )
        .unwrap();
    world
        .set_one_way(entity(0), Some(OneWay2d::default()))
        .unwrap();
    world
        .insert_body(
            entity(1),
            RigidBody2d {
                pose: PhysicsPose2d {
                    position: [0.0, -1.0],
                    rotation: 0.0,
                },
                linear_velocity: [0.0, 8.0],
                continuous_collision: true,
                lock_rotation: true,
                ..RigidBody2d::default()
            },
            &[Collider2d::circle(0.2)],
        )
        .unwrap();
    world
        .insert_static_collider(
            entity(2),
            PhysicsPose2d {
                position: [0.0, -2.0],
                rotation: 0.0,
            },
            &[Collider2d::rectangle([2.0, 0.1])],
        )
        .unwrap();
    world
}

#[test]
fn rises_through_lands_drops_to_solid_floor_and_lands_again() {
    for kind in [RigidBodyKind::Static, RigidBodyKind::KinematicVelocity] {
        let mut world = setup(kind, 0.0);
        let mut highest = -1.0_f32;
        for _ in 0..120 {
            world.step(STEP).unwrap();
            highest = highest.max(world.pose(entity(1)).unwrap().position[1]);
        }
        assert!(highest > 1.0, "passed upward: {highest}");
        assert!(
            (world.pose(entity(1)).unwrap().position[1] - 0.3).abs() < 0.06,
            "landed: {:?}",
            world.pose(entity(1))
        );
        world.drop_through(entity(1), 0.7).unwrap();
        for _ in 0..90 {
            world.step(STEP).unwrap();
        }
        assert!(
            (world.pose(entity(1)).unwrap().position[1] + 1.7).abs() < 0.06,
            "solid floor still blocks"
        );
        world.set_linear_velocity(entity(1), [0.0, 8.0]).unwrap();
        for _ in 0..140 {
            world.step(STEP).unwrap();
        }
        assert!(
            (world.pose(entity(1)).unwrap().position[1] - 0.3).abs() < 0.06,
            "timer expired"
        );
    }
}

#[test]
fn fast_ccd_ascent_passes_and_descent_is_stopped() {
    let mut world = setup(RigidBodyKind::KinematicVelocity, 0.0);
    world.set_linear_velocity(entity(1), [0.0, 240.0]).unwrap();
    world.step(STEP).unwrap();
    assert!(world.pose(entity(1)).unwrap().position[1] > 2.0);
    world
        .move_to(
            entity(1),
            PhysicsPose2d {
                position: [0.0, 3.0],
                rotation: 0.0,
            },
        )
        .unwrap();
    world.set_linear_velocity(entity(1), [0.0, -240.0]).unwrap();
    world.step(STEP).unwrap();
    assert!(
        world.pose(entity(1)).unwrap().position[1] >= 0.24,
        "fast descent lands on the support side"
    );
}

#[test]
fn rotated_piece_supports_on_its_local_normal() {
    let mut world = setup(RigidBodyKind::Static, std::f32::consts::FRAC_PI_2);
    world.set_gravity([10.0, 0.0]).unwrap();
    world
        .move_to(
            entity(1),
            PhysicsPose2d {
                position: [-1.0, 0.0],
                rotation: 0.0,
            },
        )
        .unwrap();
    world.set_linear_velocity(entity(1), [0.0, 0.0]).unwrap();
    for _ in 0..90 {
        world.step(STEP).unwrap();
    }
    assert!((world.pose(entity(1)).unwrap().position[0] + 0.3).abs() < 0.06);
}

#[test]
fn cancellation_validation_pending_and_removal() {
    let mut world = setup(RigidBodyKind::Static, 0.0);
    world
        .move_to(
            entity(1),
            PhysicsPose2d {
                position: [0.0, 0.3],
                rotation: 0.0,
            },
        )
        .unwrap();
    world.set_linear_velocity(entity(1), [0.0, 0.0]).unwrap();
    world.drop_through(entity(1), 1.0).unwrap();
    world.drop_through(entity(1), 0.0).unwrap();
    for _ in 0..30 {
        world.step(STEP).unwrap();
    }
    assert!(world.pose(entity(1)).unwrap().position[1] > 0.24);
    assert!(world.drop_through(entity(0), 1.0).is_err());
    assert!(world.drop_through(entity(1), -1.0).is_err());
    assert!(world.drop_through(entity(1), f32::NAN).is_err());
    assert!(
        world
            .set_one_way(
                entity(0),
                Some(OneWay2d {
                    normal: [0.0, 0.0],
                    ..OneWay2d::default()
                })
            )
            .is_err()
    );
    assert!(serde_json::from_str::<OneWay2d>(r#"{"angle": 2.0}"#).is_err());
    world.remember_drop_through(entity(3), 1.0).unwrap();
    world.remove(entity(3));
    assert!(world.remove(entity(0)));
    world.step(STEP).unwrap();
    assert!(world.pose(entity(1)).unwrap().position[1] < 0.3);
}

#[test]
fn sensors_and_spawn_window_timers_survive_drop_through() {
    use sindri_physics::PhysicsEventKind;
    let mut world = setup(RigidBodyKind::Static, 0.0);
    let mut sensor = Collider2d::rectangle([2.0, 0.4]);
    sensor.sensor = true;
    world
        .insert_static_collider(entity(4), PhysicsPose2d::default(), &[sensor])
        .unwrap();
    world
        .set_one_way(entity(4), Some(OneWay2d::default()))
        .unwrap();
    world.remove(entity(1));
    world.remember_drop_through(entity(1), 0.7).unwrap();
    world
        .insert_body(
            entity(1),
            RigidBody2d {
                pose: PhysicsPose2d {
                    position: [0.0, 0.3],
                    rotation: 0.0,
                },
                continuous_collision: true,
                ..RigidBody2d::default()
            },
            &[Collider2d::circle(0.2)],
        )
        .unwrap();
    world.finish_synchronize().unwrap();
    let mut entered = false;
    let mut exited = false;
    for _ in 0..90 {
        for event in world.step(STEP).unwrap() {
            if event.first == entity(1) && event.second == entity(4) {
                entered |= event.kind == PhysicsEventKind::SensorEntered;
                exited |= event.kind == PhysicsEventKind::SensorExited;
            }
        }
    }
    assert!(entered && exited, "sensors stay discrete and active");
    assert!((world.pose(entity(1)).unwrap().position[1] + 1.7).abs() < 0.06);
}

#[test]
fn policy_edits_wake_resting_bodies_on_static_platforms() {
    let mut world = setup(RigidBodyKind::Static, 0.0);
    world
        .move_to(
            entity(1),
            PhysicsPose2d {
                position: [0.0, 0.3],
                rotation: 0.0,
            },
        )
        .unwrap();
    world.set_linear_velocity(entity(1), [0.0, 0.0]).unwrap();
    for _ in 0..600 {
        world.step(STEP).unwrap();
    }
    assert!((world.pose(entity(1)).unwrap().position[1] - 0.3).abs() < 0.06);
    world
        .set_one_way(
            entity(0),
            Some(OneWay2d {
                normal: [0.0, -1.0],
                ..OneWay2d::default()
            }),
        )
        .unwrap();
    for _ in 0..90 {
        world.step(STEP).unwrap();
    }
    assert!(
        (world.pose(entity(1)).unwrap().position[1] + 1.7).abs() < 0.06,
        "changing the support side wakes a sleeping rider"
    );
}
