//! Rotation changes actual box/capsule query geometry in all three dimensions.

#[path = "physics3d_controls/support.rs"]
mod support;

use decay_runtime::Value;
use support::{Fixture, near, reference};

fn args(capsule: bool, cast: bool, axis: [f64; 3], angle: f64) -> Vec<Value> {
    let mut args = vec![Value::Vec3([0.0; 3])];
    if capsule {
        args.extend([Value::Number(3.0), Value::Number(0.1)]);
    } else {
        args.push(Value::Vec3([0.1, 3.0, 0.1]));
    }
    args.extend([Value::Vec3(axis), Value::Number(angle)]);
    if cast {
        args.extend([Value::Vec3([0.0, 0.0, 2.0]), Value::Number(10.0)]);
    }
    args.extend([Value::Number(1.0), Value::Bool(false), Value::Null]);
    args
}

#[test]
fn rotated_overlaps_reach_sideways_targets_and_filter_whole_entities() {
    for (axis, position) in [
        ([0.0, 0.0, 2.0], [2.0, 0.0, 0.0]),
        ([2.0, 0.0, 0.0], [0.0, 0.0, 2.0]),
    ] {
        let mut fixture = Fixture::new();
        let target = fixture.actor("static", true);
        fixture
            .world
            .get_mut(target)
            .unwrap()
            .transform_3d
            .as_mut()
            .unwrap()
            .position = position;
        fixture.step();
        for (call, capsule) in [("overlap_box", false), ("overlap_capsule", true)] {
            let mut probe = args(capsule, false, axis, std::f64::consts::FRAC_PI_2);
            assert_eq!(
                fixture.call(target, call, &probe, true).unwrap(),
                Value::array(vec![])
            );
            let count = probe.len();
            probe[count - 2] = Value::Bool(true);
            assert_eq!(
                fixture.call(target, call, &probe, true).unwrap(),
                Value::array(vec![reference(target)])
            );
            probe[count - 1] = reference(target);
            assert_eq!(
                fixture.call(target, call, &probe, true).unwrap(),
                Value::array(vec![])
            );
            probe[count - 1] = Value::Null;
            probe[count - 4] = Value::Number(0.0);
            assert_eq!(
                fixture.call(target, call, &probe, true).unwrap(),
                Value::array(vec![])
            );
        }
        near(
            fixture.physics.world().linear_velocity(target).unwrap(),
            [0.0; 3],
        );
    }
}

#[test]
fn rotated_sweeps_hit_with_normalized_travel_and_return_initial_overlap() {
    let mut fixture = Fixture::new();
    let target = fixture.actor("static", false);
    fixture
        .world
        .get_mut(target)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [2.0, 0.0, 5.0];
    fixture.step();
    for (call, capsule) in [("cast_box", false), ("cast_capsule", true)] {
        let mut probe = args(capsule, true, [0.0, 0.0, 1.0], 0.0);
        assert_eq!(
            fixture.call(target, call, &probe, true).unwrap(),
            Value::Null
        );
        let angle_at = if capsule { 4 } else { 3 };
        probe[angle_at] = Value::Number(std::f64::consts::FRAC_PI_2);
        let Value::Struct { fields, .. } = fixture.call(target, call, &probe, true).unwrap() else {
            panic!("rotated sweep must hit");
        };
        assert_eq!(fields[0], reference(target));
        let Value::Number(distance) = fields[3] else {
            panic!("numeric travel");
        };
        assert!((distance - 4.4).abs() < 0.02);
        let Value::Vec3(normal) = fields[2] else {
            panic!("XYZ normal");
        };
        assert!(normal[2] < -0.99);
        probe[0] = Value::Vec3([0.0, 0.0, 5.0]);
        let Value::Struct { fields, .. } = fixture.call(target, call, &probe, true).unwrap() else {
            panic!("initial overlap must hit");
        };
        assert_eq!(fields[1], probe[0]);
        assert_eq!(fields[2], Value::Vec3([0.0; 3]));
        assert_eq!(fields[3], Value::Number(0.0));
        let last = probe.len() - 1;
        probe[last] = reference(target);
        assert_eq!(
            fixture.call(target, call, &probe, true).unwrap(),
            Value::Null
        );
    }
}

#[test]
fn invalid_probe_values_arity_and_context_are_rejected() {
    let mut fixture = Fixture::new();
    let target = fixture.actor("static", false);
    fixture.step();
    for (call, capsule, cast) in [
        ("overlap_box", false, false),
        ("cast_box", false, true),
        ("overlap_capsule", true, false),
        ("cast_capsule", true, true),
    ] {
        let probe = args(capsule, cast, [0.0, 1.0, 0.0], 0.0);
        let axis_at = if capsule { 3 } else { 2 };
        for (at, value) in [
            (axis_at, Value::Vec3([0.0; 3])),
            (axis_at, Value::Vec3([f64::NAN; 3])),
            (axis_at, Value::Vec2([1.0; 2])),
            (axis_at + 1, Value::Number(f64::INFINITY)),
            (axis_at + 1, Value::Number(f64::MAX)),
            (
                1,
                if capsule {
                    Value::Number(-1.0)
                } else {
                    Value::Vec3([1.0, 0.0, 1.0])
                },
            ),
        ] {
            let mut invalid = probe.clone();
            invalid[at] = value;
            assert!(
                fixture.call(target, call, &invalid, true).is_err(),
                "{call}"
            );
        }
        assert!(fixture.call(target, call, &[], true).is_err());
        assert!(fixture.call(target, call, &probe, false).is_err());
        if capsule {
            let mut invalid = probe.clone();
            invalid[2] = Value::Number(0.0);
            assert!(fixture.call(target, call, &invalid, true).is_err());
        }
        if cast {
            for (at, value) in [
                (axis_at + 2, Value::Vec3([0.0; 3])),
                (axis_at + 3, Value::Number(-1.0)),
            ] {
                let mut invalid = probe.clone();
                invalid[at] = value;
                assert!(fixture.call(target, call, &invalid, true).is_err());
            }
        }
    }
}

#[test]
fn zero_height_capsule_and_extreme_nonzero_axes_remain_valid() {
    let mut fixture = Fixture::new();
    let target = fixture.actor("static", false);
    fixture.step();
    for axis in [
        [f64::from(f32::from_bits(1)), 0.0, 0.0],
        [f64::from(f32::MAX); 3],
    ] {
        let mut probe = args(true, false, axis, f64::from(f32::MAX));
        probe[1] = Value::Number(0.0);
        assert_eq!(
            fixture
                .call(target, "overlap_capsule", &probe, true)
                .unwrap(),
            Value::array(vec![reference(target)])
        );
    }
}

#[test]
fn positive_angles_follow_the_right_hand_rule() {
    let mut fixture = Fixture::new();
    let target = fixture.actor("static", false);
    fixture
        .world
        .get_mut(target)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [-1.5, 1.5, 0.0];
    fixture.step();
    for (call, capsule) in [("overlap_box", false), ("overlap_capsule", true)] {
        let probe = args(capsule, false, [0.0, 0.0, 1.0], std::f64::consts::FRAC_PI_4);
        assert_eq!(
            fixture.call(target, call, &probe, true).unwrap(),
            Value::array(vec![reference(target)])
        );
        let probe = args(
            capsule,
            false,
            [0.0, 0.0, 1.0],
            -std::f64::consts::FRAC_PI_4,
        );
        assert_eq!(
            fixture.call(target, call, &probe, true).unwrap(),
            Value::array(vec![])
        );
    }
}
