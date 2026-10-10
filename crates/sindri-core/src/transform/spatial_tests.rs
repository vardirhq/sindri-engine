use super::*;

fn near(actual: [f32; 3], expected: [f32; 3]) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 1.0e-5),
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn directions_ignore_scale_and_normalize_rotation() {
    let mut t = Transform3D {
        scale: [-2.0, 0.0, 5.0],
        ..Transform3D::default()
    };
    t.set_yaw_pitch_roll_radians([0.7, -0.4, 0.3]);
    let basis = [t.forward(), t.right(), t.up()];
    t.rotation = t.rotation.map(|v| v * f32::MAX);
    for (actual, expected) in [t.forward(), t.right(), t.up()].into_iter().zip(basis) {
        near(actual, expected);
        assert!((actual.into_iter().map(|v| v * v).sum::<f32>() - 1.0).abs() < 1.0e-5);
    }
    t.rotation = [0.0; 4];
    near(t.forward(), [0.0, 0.0, -1.0]);
}

#[test]
fn look_at_handles_poles_and_distant_finite_targets() {
    let mut t = Transform3D::default();
    for (target, expected) in [
        ([0.0, 1.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, -1.0, 0.0], [0.0, -1.0, 0.0]),
        ([-3.0, 4.0, 0.0], [-0.6, 0.8, 0.0]),
    ] {
        t.look_at(target).unwrap();
        near(t.forward(), expected);
        near(t.position, [0.0; 3]);
    }
    t.position = [f32::MAX, 0.0, 0.0];
    t.look_at([-f32::MAX, 0.0, 0.0]).unwrap();
    near(t.forward(), [-1.0, 0.0, 0.0]);
}

#[test]
fn orbit_rotates_position_and_orientation_with_an_unnormalized_axis() {
    let mut t = Transform3D {
        position: [3.0, 2.0, 0.0],
        scale: [2.0, 3.0, 4.0],
        ..Transform3D::default()
    };
    t.rotate_around(
        [1.0, 2.0, 0.0],
        [0.0, f32::MAX, 0.0],
        std::f32::consts::FRAC_PI_2,
    )
    .unwrap();
    near(t.position, [1.0, 2.0, -2.0]);
    near(t.forward(), [-1.0, 0.0, 0.0]);
    near(t.scale, [2.0, 3.0, 4.0]);
    t.rotate_around(
        [1.0, 2.0, 0.0],
        [0.0, 2.0, 0.0],
        -std::f32::consts::FRAC_PI_2,
    )
    .unwrap();
    near(t.position, [3.0, 2.0, 0.0]);
    near(t.forward(), [0.0, 0.0, -1.0]);
}

#[test]
fn rejected_operations_are_atomic() {
    let mut t = Transform3D {
        position: [f32::MAX, 0.0, 0.0],
        ..Transform3D::default()
    };
    let original = t;
    assert_eq!(t.look_at(t.position), Err(TransformError::CoincidentTarget));
    assert_eq!(
        t.look_at([f32::NAN, 0.0, 0.0]),
        Err(TransformError::NonFinite)
    );
    assert_eq!(
        t.rotate_around([0.0; 3], [0.0; 3], 1.0),
        Err(TransformError::ZeroAxis)
    );
    assert_eq!(
        t.rotate_around([-f32::MAX, 0.0, 0.0], [0.0, 1.0, 0.0], std::f32::consts::PI),
        Err(TransformError::NonFinite)
    );
    assert_eq!(t, original);
}
