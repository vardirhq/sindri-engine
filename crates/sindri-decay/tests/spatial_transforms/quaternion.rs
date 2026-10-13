//! Whole quaternion writes and copied values at the real script boundary.

use sindri_core::{EntityData, Transform3D};
use sindri_decay::check_source;

use super::{fixture, near, run};

#[test]
fn quaternion_values_are_copied_and_preserve_parent_relative_placement() {
    let original = Transform3D {
        position: [0.3, 0.7, 0.9],
        scale: [0.8, 0.9, 1.1],
        z_locked: true,
        ..Transform3D::default()
    };
    let (mut world, actor, registry) = fixture(original);
    let mut parent = Transform3D {
        scale: [2.0, 3.0, 4.0],
        ..Transform3D::default()
    };
    parent.set_yaw_pitch_roll_radians([0.5, 0.0, 0.0]);
    let carrier = world.spawn(EntityData {
        transform_3d: Some(parent),
        ..EntityData::default()
    });
    world.set_parent(actor, Some(carrier)).unwrap();
    let result = run(
        &mut world,
        &registry,
        r#"
        let actor = World.find("Actor");
        let original: Quat = this.transform.rotation;
        var copy = original;
        copy.y = 2.0;
        copy.w = 2.0;
        actor.transform.rotation = copy;
        if original.y != 0.0 || original.w != 1.0 { this.transform.position.x = 99.0; }
        if abs(actor.transform.rotation.y - actor.transform.rotation.w) > 0.00001 { this.transform.scale.x = 99.0; }
        let reread = this.entity.transform.rotation;
        if abs(reread.y - reread.w) > 0.00001 { this.transform.scale.x = 99.0; }
    "#,
    );
    assert!(result.is_empty(), "{result}");
    let actual = world.get(actor).unwrap().transform_3d.unwrap();
    assert_eq!(
        actual.position.map(f32::to_bits),
        original.position.map(f32::to_bits)
    );
    assert_eq!(
        actual.scale.map(f32::to_bits),
        original.scale.map(f32::to_bits)
    );
    assert!(actual.z_locked);
    near(actual.forward(), [-1.0, 0.0, 0.0]);
    near(
        world.world_transform(actor).unwrap().forward(),
        [-0.5_f32.cos(), 0.0, 0.5_f32.sin()],
    );
}

#[test]
fn normalization_handles_huge_and_tiny_finite_components() {
    for magnitude in [
        format!("1{}.0", "0".repeat(300)),
        format!("0.{}1", "0".repeat(299)),
    ] {
        let (mut world, actor, registry) = fixture(Transform3D::default());
        let source = format!(
            "this.transform.rotation = Quat(x: 0.0, y: {magnitude}, z: 0.0, w: {magnitude});"
        );
        let result = run(&mut world, &registry, &source);
        assert!(result.is_empty(), "{result}");
        let actual = world.get(actor).unwrap().transform_3d.unwrap();
        near(actual.forward(), [-1.0, 0.0, 0.0]);
        assert!((actual.rotation.into_iter().map(|v| v * v).sum::<f32>() - 1.0).abs() < 1.0e-6);
    }
}

#[test]
fn invalid_whole_writes_leave_the_transform_unchanged() {
    for body in [
        "this.transform.rotation = Quat(x: 0.0, y: 0.0, z: 0.0, w: 0.0);",
        "this.entity.transform.rotation = Quat(x: 0.0 / 0.0, y: 1.0, z: 0.0, w: 1.0);",
        "this.transform.rotation = Quat(x: 0.0, y: 1.0 / 0.0, z: 0.0, w: 1.0);",
    ] {
        let original = Transform3D {
            position: [2.0, 3.0, 4.0],
            ..Transform3D::default()
        };
        let (mut world, actor, registry) = fixture(original);
        let result = run(&mut world, &registry, body);
        assert!(result.contains("finite and nonzero"), "{body}: {result}");
        assert_eq!(world.get(actor).unwrap().transform_3d, Some(original));
    }
}

#[test]
fn quaternion_surface_rejects_wrong_types_and_unknown_components() {
    for body in [
        "this.transform.rotation = Vec3(0.0, 0.0, 1.0);",
        "this.transform.rotation = Color(1.0, 1.0, 1.0, 1.0);",
        "this.transform.rotation.q = 1.0;",
        "this.transform.rotation.y = 1.0;",
        "this.entity.transform.rotation.w = 0.0;",
        "var q: Quat = Quat(x: 0.0, y: 0.0, z: 0.0);",
    ] {
        let source = format!("script Mover {{ fn update(dt: f32) {{ {body} }} }}");
        assert!(!check_source(&source).compiles(), "{source}");
    }
}

#[test]
fn reads_preserve_authored_components_until_a_checked_write() {
    let original = Transform3D {
        rotation: [0.0, 2.0, 0.0, 2.0],
        ..Transform3D::default()
    };
    let (mut world, actor, registry) = fixture(original);
    let result = run(
        &mut world,
        &registry,
        "let q = this.transform.rotation; if q.y != 2.0 || q.w != 2.0 { this.transform.position.x = 99.0; }",
    );
    assert!(result.is_empty(), "{result}");
    assert_eq!(world.get(actor).unwrap().transform_3d, Some(original));
    let result = run(
        &mut world,
        &registry,
        "this.transform.rotation = this.transform.rotation;",
    );
    assert!(result.is_empty(), "{result}");
    let actual = world.get(actor).unwrap().transform_3d.unwrap();
    near(actual.forward(), [-1.0, 0.0, 0.0]);
    near(
        actual.yaw_pitch_roll_radians(),
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
    );
}

#[test]
fn local_rotation_writes_work_under_a_singular_parent() {
    let original = Transform3D {
        position: [0.3, 0.7, 0.9],
        z_locked: true,
        ..Transform3D::default()
    };
    let (mut world, actor, registry) = fixture(original);
    let carrier = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            scale: [0.0, 3.0, 4.0],
            ..Transform3D::default()
        }),
        ..EntityData::default()
    });
    world.set_parent(actor, Some(carrier)).unwrap();
    let result = run(
        &mut world,
        &registry,
        "this.transform.rotation = Quat(x: 0.0, y: 1.0, z: 0.0, w: 1.0);",
    );
    assert!(result.is_empty(), "{result}");
    let actual = world.get(actor).unwrap().transform_3d.unwrap();
    assert_eq!(
        actual.position.map(f32::to_bits),
        original.position.map(f32::to_bits)
    );
    assert!(actual.z_locked);
    near(actual.forward(), [-1.0, 0.0, 0.0]);
}
