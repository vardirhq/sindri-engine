use super::*;
use sindri_physics::MotorMode2d;

#[test]
fn position_drive_holds_retargets_and_switches_back_without_resetting_motion() {
    let (mut world, mut joint, owner) = world();
    joint.settings.motor_mode = MotorMode2d::Position;
    joint.settings.motor_target_angle = 0.6;
    joint.settings.motor_stiffness = 10.0;
    joint.settings.motor_damping = 2.0;
    for target in [0.6, -0.6] {
        joint.settings.motor_target_angle = target;
        let speed = world.angular_velocity(joint.second).unwrap();
        world.set_hinge_joint(owner, joint).unwrap();
        assert!((world.angular_velocity(joint.second).unwrap() - speed).abs() < 1e-5);
        for _ in 0..600 {
            world.step(STEP).unwrap();
        }
        let pose = world.pose(joint.second).unwrap();
        assert!((pose.rotation - target).abs() < 0.03, "{pose:?}");
        assert!(world.angular_velocity(joint.second).unwrap().abs() < 0.03);
        assert!(
            (pose.position[0] - pose.rotation.cos()).hypot(pose.position[1] - pose.rotation.sin())
                < 0.035
        );
        assert_eq!(world.joint_count(), 1);
    }
    joint.settings.motor_mode = MotorMode2d::Velocity;
    joint.settings.motor_velocity = -2.0;
    world.set_hinge_joint(owner, joint).unwrap();
    for _ in 0..240 {
        world.step(STEP).unwrap();
    }
    assert!(world.angular_velocity(joint.second).unwrap() < -1.5);
}

#[test]
fn position_drive_obeys_torque_caps_and_enabled_limits() {
    let (mut bounded, mut joint, owner) = world();
    joint.settings.motor_mode = MotorMode2d::Position;
    joint.settings.motor_target_angle = 0.6;
    joint.settings.motor_stiffness = 50.0;
    joint.settings.motor_damping = 2.0;
    joint.settings.limits_enabled = true;
    joint.settings.lower_angle = -0.3;
    joint.settings.upper_angle = 0.3;
    bounded.set_hinge_joint(owner, joint).unwrap();
    for _ in 0..600 {
        bounded.step(STEP).unwrap();
    }
    assert!((bounded.pose(joint.second).unwrap().rotation - 0.3).abs() < 0.03);
    let (mut weak, mut joint, owner) = world();
    joint.settings.motor_mode = MotorMode2d::Position;
    joint.settings.motor_target_angle = 0.6;
    joint.settings.motor_stiffness = 50.0;
    joint.settings.motor_damping = 2.0;
    joint.settings.motor_max_torque = 0.001;
    weak.set_hinge_joint(owner, joint).unwrap();
    for _ in 0..12 {
        weak.step(STEP).unwrap();
    }
    let speed = weak.angular_velocity(joint.second).unwrap();
    assert!(speed > 0.0 && speed < 0.1, "{speed}");
}

#[test]
fn old_payloads_select_velocity_and_bad_position_settings_are_atomic() {
    let old: HingeSettings2d = serde_json::from_str(r"{}").unwrap();
    assert_eq!(old.motor_mode, MotorMode2d::Velocity);
    let (mut world, joint, owner) = world();
    world.set_hinge_joint(owner, joint).unwrap();
    for (target, stiffness, damping) in [
        (4.0, 1.0, 1.0),
        (f32::NAN, 1.0, 1.0),
        (0.6, -1.0, 1.0),
        (0.6, 1.0, -1.0),
        (0.6, f32::INFINITY, 1.0),
    ] {
        let settings = HingeSettings2d {
            motor_mode: MotorMode2d::Position,
            motor_target_angle: target,
            motor_stiffness: stiffness,
            motor_damping: damping,
            ..joint.settings
        };
        assert!(
            world
                .set_hinge_joint(owner, HingeJoint2d { settings, ..joint })
                .is_err()
        );
        assert_eq!(world.joint_count(), 1);
    }
    for _ in 0..180 {
        world.step(STEP).unwrap();
    }
    assert!(world.angular_velocity(joint.second).unwrap() > 1.5);
}
