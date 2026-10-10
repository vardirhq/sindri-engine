//! Slope policy, oriented probes and arbitrary world up for 3D movement.

use sindri_core::EntityId;
use sindri_physics::{
    CharacterOptions3d, Collider3d, PhysicsPose3d, PhysicsWorld3d, RaycastFilter3d,
};

fn id() -> EntityId {
    EntityId::from_bits(1_u64 << 32)
}

fn ramp_world(angle: f32) -> PhysicsWorld3d {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let (sin, cos) = (angle * 0.5).sin_cos();
    world
        .insert_static_collider(
            id(),
            PhysicsPose3d {
                position: [0.0; 3],
                rotation: [0.0, 0.0, sin, cos],
            },
            &[Collider3d::cuboid([10.0, 0.25, 10.0])],
        )
        .unwrap();
    world
}

#[test]
fn climb_limit_changes_ascent_on_the_same_rotated_ramp() {
    let angle = std::f32::consts::PI / 6.0;
    let world = ramp_world(angle);
    // The sphere sits above the ramp's rotated top face, with the normal skin.
    let normal = [-angle.sin(), angle.cos(), 0.0];
    let pose = PhysicsPose3d {
        position: normal.map(|axis| axis * 0.76),
        ..PhysicsPose3d::default()
    };
    let query = |limit| {
        world
            .move_character(
                Collider3d::sphere(0.5).shape,
                pose,
                [1.0, -0.1, 0.5],
                CharacterOptions3d {
                    max_slope_angle: limit,
                    ..CharacterOptions3d::default()
                },
                RaycastFilter3d::default(),
            )
            .unwrap()
    };
    let walkable = query(std::f32::consts::FRAC_PI_4);
    assert!(
        walkable.translation[0] > 0.5 && walkable.translation[1] > 0.25,
        "{walkable:?}"
    );
    assert!(walkable.grounded);
    let steep = query(0.2);
    assert!(
        steep.translation[1] < 0.02,
        "steep ramp must not lift the actor: {steep:?}"
    );
    assert!(steep.translation[0] < walkable.translation[0]);
    assert!((walkable.translation[2] - 0.5).abs() < 0.003);
    // Curved shape-cast normals are approximate; the steep ramp can
    // slightly reduce lateral travel while rejecting ascent.
    assert!((steep.translation[2] - 0.5).abs() < 0.02, "{steep:?}");
}

#[test]
fn alternate_up_and_rotated_capsule_land_against_a_vertical_floor() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_static_collider(
            id(),
            PhysicsPose3d {
                position: [-0.25, 0.0, 0.0],
                ..PhysicsPose3d::default()
            },
            &[Collider3d::cuboid([0.25, 10.0, 10.0])],
        )
        .unwrap();
    let half = std::f32::consts::FRAC_1_SQRT_2;
    let pose = PhysicsPose3d {
        position: [3.0, 0.0, 0.0],
        rotation: [0.0, 0.0, half, half],
    };
    let result = world
        .move_character(
            Collider3d::capsule(0.5, 0.25).shape,
            pose,
            [-4.0, 0.0, 1.0],
            CharacterOptions3d {
                up: [1.0, 0.0, 0.0],
                ..CharacterOptions3d::default()
            },
            RaycastFilter3d::default(),
        )
        .unwrap();
    // Repeated curved contact normals can nudge the capsule off the plane.
    assert!((result.translation[0] + 2.24).abs() < 0.01, "{result:?}");
    assert!((result.translation[2] - 1.0).abs() < 0.003);
    assert!(result.grounded);
    assert_eq!(result.collisions[0].hit.entity, id());
    assert!(result.collisions[0].hit.normal[0] > 0.99);
}

#[test]
fn invalid_slope_slide_and_step_parameters_fail_before_queries() {
    for options in [
        CharacterOptions3d {
            min_slide_angle: f32::NAN,
            ..CharacterOptions3d::default()
        },
        CharacterOptions3d {
            min_slide_angle: 2.0,
            ..CharacterOptions3d::default()
        },
        CharacterOptions3d {
            step_min_width: -0.1,
            ..CharacterOptions3d::default()
        },
        CharacterOptions3d {
            up: [0.0, 1.01, 0.0],
            ..CharacterOptions3d::default()
        },
    ] {
        assert!(options.validate().is_err());
    }
    let almost_unit = CharacterOptions3d {
        up: [0.0, 1.00001, 0.0],
        ..CharacterOptions3d::default()
    };
    assert!(almost_unit.validate().is_ok());
    let world = ramp_world(0.0);
    let result = world
        .move_character(
            Collider3d::sphere(0.5).shape,
            PhysicsPose3d {
                position: [0.0, 3.0, 0.0],
                ..PhysicsPose3d::default()
            },
            [0.0, -4.0, 0.0],
            almost_unit,
            RaycastFilter3d::default(),
        )
        .unwrap();
    assert!((result.translation[1] + 2.24).abs() < 0.003);
    assert!(result.grounded);
}
