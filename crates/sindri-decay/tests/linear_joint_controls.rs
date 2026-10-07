//! Joint controls validate before changing runtime components or solver state.
use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent, SceneEntityId, Transform3D, World};
use sindri_decay::{Physics2d, ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{
    SceneExtractor, ScenePhysics2d, SliderJoint2dComponent, SpringJoint2dComponent,
};
use std::time::Duration;
const STEP: Duration = Duration::from_nanos(16_666_667);

#[test]
fn typed_slider_controls_support_initial_sync_and_reject_bad_calls_atomically() {
    for (call, succeeds) in [
        ("Physics.set_slider_motor(this.entity, 2.0, 10.0);", true),
        (
            "Physics.set_slider_motor(this.entity, 2.0, 10.0); Physics.set_slider_motor(this.entity, 0.0, 0.0);",
            true,
        ),
        ("Physics.set_slider_motor(this.entity, 2.0, -1.0);", false),
        (
            "Physics.set_slider_motor(this.entity, Vec2(1.0, 0.0), 10.0);",
            false,
        ),
        ("Physics.set_spring(this.entity, 1.0, 10.0, 1.0);", false),
    ] {
        exercise(SliderJoint2dComponent::TYPE_NAME, call, succeeds, true);
    }
    exercise(
        SliderJoint2dComponent::TYPE_NAME,
        "Physics.set_slider_motor(this.entity, 2.0, 10.0);",
        false,
        false,
    );
}
#[test]
fn typed_spring_tuning_supports_initial_sync_and_preserves_unknown_fields() {
    for (call, succeeds) in [
        ("Physics.set_spring(this.entity, 2.0, 10.0, 4.0);", true),
        ("Physics.set_spring(this.entity, 0.0, 10.0, 4.0);", false),
        ("Physics.set_spring(this.entity, 2.0, -1.0, 4.0);", false),
        ("Physics.set_spring(this.entity, 2.0, 10.0, -1.0);", false),
        ("Physics.set_slider_motor(this.entity, 2.0, 10.0);", false),
    ] {
        exercise(SpringJoint2dComponent::TYPE_NAME, call, succeeds, true);
    }
    exercise(
        SpringJoint2dComponent::TYPE_NAME,
        "Physics.set_spring(this.entity, 2.0, 10.0, 4.0);",
        false,
        false,
    );
}

fn exercise(kind: &str, call: &str, succeeds: bool, with_physics: bool) {
    let (extractor, mut world, owner, moving) = fixture(kind);
    let registry = extractor.components();
    let original = world.get(owner).unwrap().components[kind].clone();
    let mut sources = ScriptSources::new();
    sources.insert(
        "joint.decay",
        format!("script Joint {{ fn start() {{ {call} }} }}"),
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
    let payload = &world.get(owner).unwrap().components[kind];
    if succeeds {
        assert_eq!(payload["future_field"], json!(17));
        let driven = payload
            .get("motor_enabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        for _ in 0..300 {
            physics.step(&mut world, registry, STEP).unwrap();
        }
        assert_eq!(physics.world().joint_count(), 1);
        if kind == SpringJoint2dComponent::TYPE_NAME {
            assert!((physics.world().pose(moving).unwrap().position[0] - 2.0).abs() < 0.05);
        } else if driven {
            assert!(physics.world().linear_velocity(moving).unwrap()[0] > 1.8);
        } else {
            assert!(physics.world().linear_velocity(moving).unwrap()[0].abs() < 1e-6);
        }
    } else {
        assert_eq!(payload, &original);
    }
}

fn fixture(kind: &str) -> (SceneExtractor, World, EntityId, EntityId) {
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.register::<ScriptComponent>("Script").unwrap();
    let registry = extractor.components();
    let mut world = World::default();
    let mut moving = None;
    for (id, dynamic) in [("anchor", false), ("body", true)] {
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
        let entity = world.spawn(EntityData {
            source_id: SceneEntityId::new(id).ok(),
            transform_3d: Some(Transform3D {
                position: [if dynamic { 1.0 } else { 0.0 }, 0.0, 0.0],
                ..Transform3D::default()
            }),
            components,
            ..EntityData::default()
        });
        if dynamic {
            moving = Some(entity);
        }
    }
    let owner = world.spawn(EntityData {
        source_id: SceneEntityId::new("joint").ok(),
        components: [
            (
                kind.into(),
                json!({"first": "anchor", "second": "body", "future_field": 17}),
            ),
            (
                ScriptComponent::TYPE_NAME.into(),
                json!({"source": "joint.decay", "script": "Joint"}),
            ),
        ]
        .into(),
        ..EntityData::default()
    });
    (extractor, world, owner, moving.unwrap())
}
