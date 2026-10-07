use super::*;
use sindri_physics::MotorMode2d;

#[test]
fn position_drive_holds_signed_anchor_distance_on_a_rotated_rail_and_retargets() {
    let rotation = std::f32::consts::FRAC_PI_4;
    let (mut world, mut joint, owner) = setup(rotation);
    joint.settings.first_anchor = [0.2, 0.0];
    joint.settings.second_anchor = [-0.3, 0.0];
    joint.settings.motor_mode = MotorMode2d::Position;
    joint.settings.motor_stiffness = 20.0;
    joint.settings.motor_damping = 4.0;
    for target in [0.6, -0.6] {
        joint.settings.motor_target_distance = target;
        let speed = world.linear_velocity(joint.second).unwrap();
        world.set_slider_joint(owner, joint).unwrap();
        let after = world.linear_velocity(joint.second).unwrap();
        assert!((after[0] - speed[0]).hypot(after[1] - speed[1]) < 1e-5);
        for _ in 0..600 {
            world.step(STEP).unwrap();
        }
        let pose = world.pose(joint.second).unwrap();
        let along = pose.position[0] * rotation.cos() + pose.position[1] * rotation.sin() - 0.5;
        let across = -pose.position[0] * rotation.sin() + pose.position[1] * rotation.cos();
        assert!((along - target).abs() < 0.03, "{pose:?}");
        assert!(across.abs() < 0.03);
        assert!((pose.rotation - rotation).abs() < 0.03);
        assert!(world.linear_velocity(joint.second).unwrap()[0].abs() < 0.03);
        assert_eq!(world.joint_count(), 1);
    }
    joint.settings.motor_mode = MotorMode2d::Velocity;
    joint.settings.motor_velocity = -2.0;
    world.set_slider_joint(owner, joint).unwrap();
    for _ in 0..240 {
        world.step(STEP).unwrap();
    }
    assert!(world.linear_velocity(joint.second).unwrap()[0] < -1.0);
}

#[test]
fn position_drive_obeys_force_caps_and_enabled_travel_limits() {
    let (mut bounded, mut joint, owner) = setup(0.0);
    joint.settings.motor_mode = MotorMode2d::Position;
    joint.settings.motor_target_distance = 2.0;
    joint.settings.motor_stiffness = 50.0;
    joint.settings.motor_damping = 4.0;
    joint.settings.limits_enabled = true;
    joint.settings.lower_distance = -0.3;
    joint.settings.upper_distance = 0.3;
    bounded.set_slider_joint(owner, joint).unwrap();
    for _ in 0..600 {
        bounded.step(STEP).unwrap();
    }
    assert!((bounded.pose(joint.second).unwrap().position[0] - 0.3).abs() < 0.03);
    let (mut weak, mut joint, owner) = setup(0.0);
    joint.settings.motor_mode = MotorMode2d::Position;
    joint.settings.motor_target_distance = 2.0;
    joint.settings.motor_stiffness = 50.0;
    joint.settings.motor_damping = 4.0;
    joint.settings.motor_max_force = 0.1;
    weak.set_slider_joint(owner, joint).unwrap();
    for _ in 0..12 {
        weak.step(STEP).unwrap();
    }
    let speed = weak.linear_velocity(joint.second).unwrap()[0];
    assert!((speed - 0.1 * 0.2 / std::f32::consts::FRAC_PI_4).abs() < 0.002);
}

#[test]
fn old_payloads_select_velocity_and_bad_position_settings_are_atomic() {
    let old: SliderSettings2d = serde_json::from_str(r"{}").unwrap();
    assert_eq!(old.motor_mode, MotorMode2d::Velocity);
    let (mut world, joint, owner) = setup(0.0);
    world.set_slider_joint(owner, joint).unwrap();
    for (target, stiffness, damping) in [
        (f32::INFINITY, 1.0, 1.0),
        (f32::NAN, 1.0, 1.0),
        (0.6, -1.0, 1.0),
        (0.6, 1.0, -1.0),
        (0.6, f32::INFINITY, 1.0),
    ] {
        let settings = SliderSettings2d {
            motor_mode: MotorMode2d::Position,
            motor_target_distance: target,
            motor_stiffness: stiffness,
            motor_damping: damping,
            ..joint.settings
        };
        assert!(
            world
                .set_slider_joint(owner, SliderJoint2d { settings, ..joint })
                .is_err()
        );
        assert_eq!(world.joint_count(), 1);
    }
    for _ in 0..180 {
        world.step(STEP).unwrap();
    }
    assert!(world.linear_velocity(joint.second).unwrap()[0] > 1.5);
}
