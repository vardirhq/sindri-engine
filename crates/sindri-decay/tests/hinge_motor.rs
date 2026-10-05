//! Typed motor control survives the authored-to-solver lifecycle window.

use serde_json::json;
use sindri_core::{EntityData, SceneComponent, SceneEntityId, Transform3D, World};
use sindri_decay::{Physics2d, ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{HingeJoint2dComponent, SceneExtractor, ScenePhysics2d};
use std::time::Duration;

const STEP: Duration = Duration::from_nanos(16_666_667);

#[test]
fn motor_calls_validate_atomically_and_support_unsynchronized_endpoints() {
    for (call, succeeds, driven, with_physics) in [
        (
            "Physics.set_hinge_motor(this.entity, 2.0, 1.0);",
            true,
            true,
            true,
        ),
        (
            "Physics.set_hinge_motor(this.entity, 2.0, 1.0); Physics.set_hinge_motor(this.entity, 0.0, 0.0);",
            true,
            false,
            true,
        ),
        (
            "Physics.set_hinge_motor(this.entity, 2.0, -1.0);",
            false,
            false,
            true,
        ),
        (
            "Physics.set_hinge_motor(this.entity, Vec2(1.0, 0.0), 1.0);",
            false,
            false,
            true,
        ),
        (
            "Physics.set_hinge_motor(World.find(\"Anchor\"), 2.0, 1.0);",
            false,
            false,
            true,
        ),
        (
            "Physics.set_hinge_motor(this.entity, 2.0, 1.0);",
            false,
            false,
            false,
        ),
    ] {
        run_case(call, succeeds, driven, with_physics);
    }
}

fn run_case(call: &str, succeeds: bool, driven: bool, with_physics: bool) {
    let (extractor, mut world) = authored_bodies();
    let registry = extractor.components();
    let owner = world.spawn(EntityData {
        source_id: SceneEntityId::new("hinge").ok(),
        components: [
            (
                HingeJoint2dComponent::TYPE_NAME.into(),
                json!({"first": "anchor",
            "second": "rotor", "future_field": 17}),
            ),
            (
                ScriptComponent::TYPE_NAME.into(),
                json!({"source": "motor.decay",
            "script": "Motor"}),
            ),
        ]
        .into(),
        ..EntityData::default()
    });
    let original = world.get(owner).unwrap().components[HingeJoint2dComponent::TYPE_NAME].clone();
    let mut sources = ScriptSources::new();
    sources.insert(
        "motor.decay",
        format!("script Motor {{ fn start() {{ {call} }} }}"),
    );
    let mut scripts = Scripts::new();
    let mut physics = ScenePhysics2d::top_down().unwrap();
    let input = InputState::default();
    let frame = ScriptFrame::new(&sources, &input, 1.0 / 60.0);
    let frame = if with_physics {
        let (backend, events) = physics.for_scripts();
        frame.with_physics(Physics2d {
            world: backend,
            events,
        })
    } else {
        frame
    };
    let report = scripts.advance(&mut world, registry, frame);
    assert_eq!(
        report.failures.is_empty(),
        succeeds,
        "{:?}",
        report.failures
    );
    let payload = &world.get(owner).unwrap().components[HingeJoint2dComponent::TYPE_NAME];
    if succeeds {
        assert_eq!(payload["future_field"], json!(17));
        assert_eq!(payload["motor_enabled"], json!(driven));
        let rotor = world
            .entity_for_source_id(&SceneEntityId::new("rotor").unwrap())
            .unwrap();
        for _ in 0..120 {
            physics.step(&mut world, registry, STEP).unwrap();
        }
        let speed = physics.world().angular_velocity(rotor).unwrap();
        if driven {
            assert!(speed > 1.5);
        } else {
            assert!(speed.abs() < 1e-5);
        }
        assert_eq!(physics.world().joint_count(), 1);
    } else {
        assert_eq!(
            payload, &original,
            "rejected controls must not mutate the component"
        );
    }
}

fn authored_bodies() -> (SceneExtractor, World) {
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.register::<ScriptComponent>("Script").unwrap();
    let registry = extractor.components();
    let mut world = World::default();
    for (id, dynamic) in [("anchor", false), ("rotor", true)] {
        let mut components = [(
            "sindri.physics2d.collider".into(),
            registry
                .default_payload("sindri.physics2d.collider")
                .unwrap()
                .clone(),
        )]
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
        if dynamic {
            components.insert(
                "sindri.physics2d.rigid_body".into(),
                registry
                    .default_payload("sindri.physics2d.rigid_body")
                    .unwrap()
                    .clone(),
            );
        }
        world.spawn(EntityData {
            source_id: SceneEntityId::new(id).ok(),
            name: Some(if dynamic { "Rotor" } else { "Anchor" }.into()),
            transform_3d: Some(Transform3D::default()),
            components,
            ..EntityData::default()
        });
    }
    (extractor, world)
}
