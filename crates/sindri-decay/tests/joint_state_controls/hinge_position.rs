use super::creation::run_call;
use super::*;

const HOLD: &str = "Physics.set_hinge_position_motor(this.entity, 0.6, 5.0, 1.0, 1.0);";

#[test]
fn position_settings_apply_before_sync_and_survive_rebuild_and_resume() {
    for built in [false, true] {
        let (extractor, mut world, owner, moving) = fixture(KINDS[1]);
        let mut physics = ScenePhysics2d::top_down().unwrap();
        if built {
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
        }
        run_call(&mut world, &extractor, &mut physics, HOLD, true);
        let payload = &world.get(owner).unwrap().components[KINDS[1]];
        assert_eq!(payload["motor_mode"], "position");
        assert_eq!(payload["future_field"], 17);
        for _ in 0..600 {
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
        }
        assert!((physics.world().pose(moving).unwrap().rotation - 0.6).abs() < 0.03);
        run_call(
            &mut world,
            &extractor,
            &mut physics,
            "Physics.set_joint_enabled(this.entity, false);",
            true,
        );
        physics
            .step(&mut world, extractor.components(), STEP)
            .unwrap();
        assert_eq!(physics.world().joint_count(), 0);
        run_call(
            &mut world,
            &extractor,
            &mut physics,
            "Physics.set_joint_enabled(this.entity, true);",
            true,
        );
        world
            .get_mut(moving)
            .unwrap()
            .components
            .get_mut("sindri.physics2d.collider")
            .unwrap()["pieces"][0]["friction"] = json!(0.8);
        for _ in 0..120 {
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
        }
        assert_eq!(physics.world().joint_count(), 1);
        assert!((physics.world().pose(moving).unwrap().rotation - 0.6).abs() < 0.03);
        run_call(
            &mut world,
            &extractor,
            &mut physics,
            "Physics.set_hinge_motor(this.entity, -2.0, 1.0);",
            true,
        );
        assert_eq!(
            world.get(owner).unwrap().components[KINDS[1]]["motor_mode"],
            "velocity"
        );
        for _ in 0..180 {
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
        }
        assert!(physics.world().angular_velocity(moving).unwrap() < -1.5);
    }
}

#[test]
fn invalid_position_calls_leave_authored_state_and_live_motor_untouched() {
    for call in [
        "Physics.set_hinge_position_motor(this.entity, 4.0, 5.0, 1.0, 1.0);",
        "Physics.set_hinge_position_motor(this.entity, 0.6, -5.0, 1.0, 1.0);",
        "Physics.set_hinge_position_motor(this.entity, 0.6, 5.0, -1.0, 1.0);",
        "Physics.set_hinge_position_motor(this.entity, 0.6, 5.0, 1.0, -1.0);",
        "Physics.set_hinge_position_motor(this.entity, 0.6, 400000000000000000000000000000000000000.0, 1.0, 1.0);",
        "Physics.set_hinge_position_motor(World.find(\"missing\"), 0.6, 5.0, 1.0, 1.0);",
    ] {
        let (extractor, mut world, owner, moving) = fixture(KINDS[1]);
        let mut physics = ScenePhysics2d::top_down().unwrap();
        run_call(&mut world, &extractor, &mut physics, HOLD, true);
        physics
            .step(&mut world, extractor.components(), STEP)
            .unwrap();
        let before = world.get(owner).unwrap().components.clone();
        let speed = physics.world().angular_velocity(moving).unwrap();
        run_call(&mut world, &extractor, &mut physics, call, false);
        assert_eq!(world.get(owner).unwrap().components, before);
        assert_eq!(physics.world().joint_count(), 1);
        assert!((physics.world().angular_velocity(moving).unwrap() - speed).abs() < 1e-5);
    }
}

#[test]
fn zero_torque_disables_position_drive_and_coasts() {
    let (extractor, mut world, owner, moving) = fixture(KINDS[1]);
    let mut physics = ScenePhysics2d::top_down().unwrap();
    run_call(
        &mut world,
        &extractor,
        &mut physics,
        "Physics.set_hinge_motor(this.entity, 2.0, 1.0);",
        true,
    );
    for _ in 0..120 {
        physics
            .step(&mut world, extractor.components(), STEP)
            .unwrap();
    }
    run_call(
        &mut world,
        &extractor,
        &mut physics,
        "Physics.set_hinge_position_motor(this.entity, 0.6, 5.0, 1.0, 0.0);",
        true,
    );
    assert_eq!(
        world.get(owner).unwrap().components[KINDS[1]]["motor_enabled"],
        false
    );
    for _ in 0..120 {
        physics
            .step(&mut world, extractor.components(), STEP)
            .unwrap();
    }
    assert!(physics.world().angular_velocity(moving).unwrap() > 1.5);
}

#[test]
fn position_drive_rejects_missing_physics_wrong_kind_and_malformed_hinges() {
    for problem in ["no_physics", "wrong_kind", "malformed"] {
        let kind = if problem == "wrong_kind" {
            KINDS[0]
        } else {
            KINDS[1]
        };
        let (extractor, mut world, owner, _) = fixture(kind);
        if problem == "malformed" {
            world
                .get_mut(owner)
                .unwrap()
                .components
                .get_mut(kind)
                .unwrap()["motor_mode"] = json!("unknown");
        }
        let original = world.get(owner).unwrap().components.clone();
        let mut sources = ScriptSources::new();
        sources.insert(
            "joint.decay",
            format!("script Joint {{ fn start() {{ {HOLD} }} }}"),
        );
        let input = InputState::default();
        let frame = ScriptFrame::new(&sources, &input, 1.0 / 60.0);
        let mut physics = ScenePhysics2d::top_down().unwrap();
        let frame = if problem == "no_physics" {
            frame
        } else {
            let (backend, events) = physics.for_scripts();
            frame.with_physics(Physics2d {
                world: backend,
                events,
            })
        };
        let report = Scripts::new().advance(&mut world, extractor.components(), frame);
        assert!(!report.failures.is_empty(), "{problem}");
        assert_eq!(world.get(owner).unwrap().components, original);
    }
}
