//! The same typed motion controls before and after body materialization.

use std::time::Duration;

use serde_json::json;
use sindri_core::{EntityData, SceneComponent, Transform3D, World};
use sindri_decay::{Physics2d, ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_physics::{Collider2d, PhysicsWorld2d, RigidBody2d};
use sindri_scene::{SceneExtractor, ScenePhysics2d};

fn authored() -> (SceneExtractor, World, sindri_core::EntityId) {
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.register::<ScriptComponent>("Script").unwrap();
    let registry = extractor.components();
    let mut world = World::default();
    let entity = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [
            (
                "sindri.physics2d.rigid_body".to_owned(),
                registry
                    .default_payload("sindri.physics2d.rigid_body")
                    .unwrap()
                    .clone(),
            ),
            (
                "sindri.physics2d.collider".to_owned(),
                registry
                    .default_payload("sindri.physics2d.collider")
                    .unwrap()
                    .clone(),
            ),
            (
                ScriptComponent::TYPE_NAME.to_owned(),
                json!({"source": "motion.decay", "script": "Motion"}),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    (extractor, world, entity)
}

#[test]
fn a_host_with_its_own_driver_needs_no_authored_body_component() {
    let (extractor, mut world, entity) = authored();
    world.get_mut(entity).unwrap().components.remove("sindri.physics2d.rigid_body");
    world.get_mut(entity).unwrap().components.remove("sindri.physics2d.collider");
    let mut physics = PhysicsWorld2d::new([0.0; 2]).unwrap();
    physics.insert_body(entity, RigidBody2d::default(), &[Collider2d::rectangle([0.5; 2])]).unwrap();
    let mut sources = ScriptSources::new();
    sources.insert("motion.decay", r"
    script Motion {
        fn start() {
            Physics.set_angular_velocity(this.entity, 2.0);
            if Physics.angular_velocity(this.entity) != 2.0 { this.transform.position.y = 99.0; }
            Physics.apply_force(this.entity, Vec2(2.0, 0.0));
            Physics.apply_torque(this.entity, 1.0);
            Physics.apply_angular_impulse(this.entity, 0.5);
            Physics.apply_impulse_at_point(this.entity, Vec2(1.0, 0.0), Vec2(0.0, 0.5));
        }
    }");
    let input = InputState::default();
    let report = Scripts::new().advance(&mut world, extractor.components(),
        ScriptFrame::new(&sources, &input, 0.1).with_physics(Physics2d { world: &mut physics, events: &[] }));
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert!(world.get(entity).unwrap().transform_3d.unwrap().position[1].abs() < f32::EPSILON);
    physics.step(Duration::from_millis(100)).unwrap();
    assert!((physics.linear_velocity(entity).unwrap()[0] - 1.2).abs() < 1.0e-4);
    assert!((physics.angular_velocity(entity).unwrap() - 2.6).abs() < 1.0e-4);
}

#[test]
fn typed_motion_controls_work_in_the_spawn_window_and_on_live_bodies() {
    for materialized in [false, true] {
        let (extractor, mut world, entity) = authored();
        let mut physics = ScenePhysics2d::top_down().unwrap();
        if materialized {
            physics
                .step(
                    &mut world,
                    extractor.components(),
                    Duration::from_millis(100),
                )
                .unwrap();
        }
        let mut scripts = Scripts::new();
        let mut sources = ScriptSources::new();
        sources.insert("motion.decay", r"
        script Motion {
            fn start() {
                Physics.set_angular_velocity(this.entity, 2.0);
                if Physics.angular_velocity(this.entity) != 2.0 { this.transform.position.y = 99.0; }
                Physics.apply_force(this.entity, Vec2(2.0, 0.0));
                Physics.apply_torque(this.entity, 1.0);
                Physics.apply_angular_impulse(this.entity, 0.5);
                Physics.apply_impulse_at_point(this.entity, Vec2(1.0, 0.0), Vec2(0.0, 0.5));
            }
        }");
        let input = InputState::default();
        let (backend, events) = physics.for_scripts();
        let report = scripts.advance(
            &mut world,
            extractor.components(),
            ScriptFrame::new(&sources, &input, 0.1).with_physics(Physics2d {
                world: backend,
                events,
            }),
        );
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        assert!(world.get(entity).unwrap().transform_3d.unwrap().position[1].abs() < f32::EPSILON);
        physics
            .step(
                &mut world,
                extractor.components(),
                Duration::from_millis(100),
            )
            .unwrap();
        assert!((physics.world().linear_velocity(entity).unwrap()[0] - 1.2).abs() < 1.0e-4);
        assert!((physics.world().angular_velocity(entity).unwrap() - 2.6).abs() < 1.0e-4);
        physics
            .step(
                &mut world,
                extractor.components(),
                Duration::from_millis(100),
            )
            .unwrap();
        assert!((physics.world().angular_velocity(entity).unwrap() - 2.6).abs() < 1.0e-4);
        let rotation = world.get(entity).unwrap().transform_3d.unwrap().rotation;
        assert!(rotation[2].abs() > 0.01, "scene writes rotation back");
    }
}

#[test]
fn motion_controls_report_wrong_kinds_missing_bodies_and_missing_physics() {
    for (kind, physics_enabled, body_present) in [
        ("static", true, true),
        ("dynamic", false, true),
        ("dynamic", true, false),
    ] {
        let (extractor, mut world, entity) = authored();
        if body_present {
            world
                .get_mut(entity)
                .unwrap()
                .components
                .get_mut("sindri.physics2d.rigid_body")
                .unwrap()["kind"] = json!(kind);
        } else {
            world
                .get_mut(entity)
                .unwrap()
                .components
                .remove("sindri.physics2d.rigid_body");
        }
        let mut physics = ScenePhysics2d::top_down().unwrap();
        let mut scripts = Scripts::new();
        let mut sources = ScriptSources::new();
        sources.insert(
            "motion.decay",
            "script Motion { fn start() { Physics.apply_force(this.entity, Vec2(1.0, 0.0)); } }",
        );
        let input = InputState::default();
        let mut frame = ScriptFrame::new(&sources, &input, 0.1);
        if physics_enabled {
            let (backend, events) = physics.for_scripts();
            frame = frame.with_physics(Physics2d {
                world: backend,
                events,
            });
        }
        let report = scripts.advance(&mut world, extractor.components(), frame);
        assert!(!report.failures.is_empty());
    }
}
