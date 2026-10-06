//! Hinge construction uses finite local anchors and keeps structural ownership.
use super::creation::{empty_fixture, run_call};
use super::*;

pub(super) const CREATE: &str = r#"Physics.create_hinge_joint(this.entity, World.find("Anchor"), World.find("Body"), Vec2(1.0, 0.0), Vec2(0.0, 0.0));"#;

#[test]
fn local_anchors_and_motor_control_work_before_and_after_body_sync() {
    for built in [false, true] {
        let (extractor, mut world, owner, body) = empty_fixture();
        let anchor = world
            .entity_for_source_id(&SceneEntityId::new("anchor").unwrap())
            .unwrap();
        let other = world.get(owner).unwrap().components.clone();
        let mut physics = ScenePhysics2d::top_down().unwrap();
        physics
            .world_mut()
            .remember_distance_joint(anchor, body, 10.0)
            .unwrap();
        if built {
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
        }
        for _ in 0..2 {
            let before = physics.world().joint_count();
            run_call(&mut world, &extractor, &mut physics, CREATE, true);
            assert_eq!(physics.world().joint_count(), before);
            let joint = &world.get(owner).unwrap().components[KINDS[1]];
            assert_eq!(joint["first_anchor"], json!([1.0, 0.0]));
            assert_eq!(joint["second_anchor"], json!([0.0, 0.0]));
            assert_eq!(joint["motor_enabled"], false);
            assert_eq!(joint["limits_enabled"], false);
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
            assert_eq!(physics.world().joint_count(), 2);
            assert!(physics.world().angular_velocity(body).unwrap() > 1.0);
            let position = world.world_transform(body).unwrap().position_2d();
            assert!((position[0] - 1.0).hypot(position[1]) < 0.02);
            let speed = physics.world().angular_velocity(body).unwrap();
            run_call(
                &mut world,
                &extractor,
                &mut physics,
                "Physics.remove_joint(this.entity);",
                true,
            );
            assert_eq!(world.get(owner).unwrap().components, other);
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
            assert_eq!(physics.world().joint_count(), 1);
            assert!((physics.world().angular_velocity(body).unwrap() - speed).abs() < 0.01);
        }
    }
}

#[test]
fn conflicting_owners_and_invalid_anchors_fail_before_mutation() {
    for kind in KINDS {
        let (extractor, mut world, owner, _) = fixture(kind);
        let before = world.get(owner).unwrap().components.clone();
        run_call(
            &mut world,
            &extractor,
            &mut ScenePhysics2d::top_down().unwrap(),
            CREATE,
            false,
        );
        assert_eq!(world.get(owner).unwrap().components, before);
    }
    for anchors in [
        "Vec2(1e40, 0.0), Vec2(0.0, 0.0)",
        "Vec2(0.0, 0.0), Vec2(0.0, 1e40)",
    ] {
        let (extractor, mut world, owner, _) = empty_fixture();
        let before = world.get(owner).unwrap().components.clone();
        let call = format!(
            r#"Physics.create_hinge_joint(this.entity, World.find("Anchor"), World.find("Body"), {anchors});"#
        );
        run_call(
            &mut world,
            &extractor,
            &mut ScenePhysics2d::top_down().unwrap(),
            &call,
            false,
        );
        assert_eq!(world.get(owner).unwrap().components, before);
    }
}

#[test]
fn unbound_endpoints_and_missing_physics_have_explicit_contracts() {
    let (extractor, mut world, owner, body) = empty_fixture();
    let before = world.get(owner).unwrap().components.clone();
    let mut sources = ScriptSources::new();
    sources.insert(
        "joint.decay",
        format!("script Joint {{ fn start() {{ {CREATE} }} }}"),
    );
    let report = Scripts::new().advance(
        &mut world,
        extractor.components(),
        ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
    );
    assert!(!report.failures.is_empty());
    assert_eq!(world.get(owner).unwrap().components, before);
    let mut physics = ScenePhysics2d::top_down().unwrap();
    world.get_mut(body).unwrap().disabled = true;
    run_call(&mut world, &extractor, &mut physics, CREATE, true);
    physics
        .step(&mut world, extractor.components(), STEP)
        .unwrap();
    assert_eq!(physics.world().joint_count(), 0);
    world.get_mut(body).unwrap().disabled = false;
    physics
        .step(&mut world, extractor.components(), STEP)
        .unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    run_call(
        &mut world,
        &extractor,
        &mut physics,
        "Physics.remove_joint(this.entity); Physics.create_hinge_joint(this.entity, null, null, Vec2(0.0, 0.0), Vec2(0.0, 0.0));",
        true,
    );
    physics
        .step(&mut world, extractor.components(), STEP)
        .unwrap();
    assert_eq!(physics.world().joint_count(), 0);
    assert_eq!(world.get(owner).unwrap().components[KINDS[1]]["first"], "");
}
