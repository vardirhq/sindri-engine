//! An impulse through a point: through the centre it only pushes; through an
//! end it pushes and turns, as a thruster at the end of a ship does.

use sindri_core::EntityId;
use sindri_physics::{
    BodyControl3d, Collider3d, PhysicsError, PhysicsWorld3d, RigidBody3d, RigidBodyKind,
};

fn id(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}

/// A free 4 x 1 x 1 bar along X, centred on the origin.
fn bar() -> PhysicsWorld3d {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_body(
            id(1),
            RigidBody3d::default(),
            &[Collider3d::cuboid([2.0, 0.5, 0.5])],
        )
        .unwrap();
    world
}

#[test]
fn through_the_centre_it_only_pushes() {
    let mut world = bar();
    world
        .apply_impulse_at(id(1), [0.0, 0.0, 4.0], [0.0, 0.0, 0.0])
        .unwrap();
    let spin = world.angular_velocity(id(1)).unwrap();
    assert!(spin.iter().all(|axis| axis.abs() < 1e-5), "{spin:?}");
    assert!(world.linear_velocity(id(1)).unwrap()[2] > 0.0);
}

#[test]
fn through_an_end_it_pushes_and_turns() {
    let mut world = bar();
    // Pushing +Z at the +X end turns the bar about -Y (right-hand rule).
    world
        .apply_impulse_at(id(1), [0.0, 0.0, 4.0], [2.0, 0.0, 0.0])
        .unwrap();
    let spin = world.angular_velocity(id(1)).unwrap();
    assert!(spin[1] < -0.1, "{spin:?}");
    let push = world.linear_velocity(id(1)).unwrap();
    assert!(
        push[2] > 0.0,
        "it is pushed as much as by a central impulse"
    );
    // The same impulse through the centre gives the same push.
    let mut central = bar();
    central.apply_impulse(id(1), [0.0, 0.0, 4.0]).unwrap();
    let central_push = central.linear_velocity(id(1)).unwrap();
    assert!((central_push[2] - push[2]).abs() < 1e-5);
}

#[test]
fn it_queues_before_a_body_exists_and_refuses_what_it_cannot_push() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .remember_control(
            id(1),
            RigidBodyKind::Dynamic,
            BodyControl3d::ImpulseAt {
                impulse: [0.0, 0.0, 4.0],
                point: [2.0, 0.0, 0.0],
            },
        )
        .unwrap();
    world
        .insert_body(
            id(1),
            RigidBody3d::default(),
            &[Collider3d::cuboid([2.0, 0.5, 0.5])],
        )
        .unwrap();
    assert!(world.angular_velocity(id(1)).unwrap()[1] < -0.1);

    let mut world = bar();
    assert!(matches!(
        world.apply_impulse_at(id(1), [0.0, 0.0, f32::NAN], [0.0; 3]),
        Err(PhysicsError::NonFinite(_))
    ));
    world
        .insert_body(
            id(2),
            RigidBody3d {
                kind: RigidBodyKind::Static,
                ..RigidBody3d::default()
            },
            &[Collider3d::sphere(0.5)],
        )
        .unwrap();
    assert!(matches!(
        world.apply_impulse_at(id(2), [1.0, 0.0, 0.0], [0.0; 3]),
        Err(PhysicsError::WrongBodyKind(..))
    ));
}
