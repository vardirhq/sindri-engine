//! Typed authored-state controls validate atomically and share the fixed-step lifecycle.
use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent, SceneEntityId, Transform3D, World};
use sindri_decay::{Physics2d, ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{
    DistanceJoint2dComponent, HingeJoint2dComponent, SceneExtractor, ScenePhysics2d,
    SliderJoint2dComponent, SpringJoint2dComponent,
};
use std::time::Duration;

const STEP: Duration = Duration::from_nanos(16_666_667);
const KINDS: [&str; 4] = [
    DistanceJoint2dComponent::TYPE_NAME,
    HingeJoint2dComponent::TYPE_NAME,
    SliderJoint2dComponent::TYPE_NAME,
    SpringJoint2dComponent::TYPE_NAME,
];

fn fixture(kind: &str) -> (SceneExtractor, World, EntityId, EntityId) {
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.register::<ScriptComponent>("Script").unwrap();
    let registry = extractor.components();
    let mut world = World::default();
    let mut moving = None;
    for (name, dynamic) in [("anchor", false), ("body", true)] {
        let mut components = std::collections::BTreeMap::from([(
            "sindri.physics2d.collider".into(),
            registry
                .default_payload("sindri.physics2d.collider")
                .unwrap()
                .clone(),
        )]);
        if dynamic {
            components.insert(
                "sindri.physics2d.rigid_body".into(),
                registry
                    .default_payload("sindri.physics2d.rigid_body")
                    .unwrap()
                    .clone(),
            );
        }
        components.get_mut("sindri.physics2d.collider").unwrap()["pieces"][0]["layers"] =
            json!({"memberships": 0, "filter": 0});
        let entity = world.spawn(EntityData {
            source_id: SceneEntityId::new(name).ok(),
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
    let mut payload = registry.default_payload(kind).unwrap().clone();
    payload["first"] = json!("anchor");
    payload["second"] = json!("body");
    payload["future_field"] = json!(17);
    // Old authored payloads have no enabled field and must remain enabled.
    payload.as_object_mut().unwrap().remove("enabled");
    let owner = world.spawn(EntityData {
        source_id: SceneEntityId::new("joint").ok(),
        components: [
            (kind.into(), payload),
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

#[test]
fn suspension_getter_and_reconnection_work_before_and_after_initial_sync() {
    for kind in KINDS {
        for built in [false, true] {
            let (extractor, mut world, owner, moving) = fixture(kind);
            let mut physics = ScenePhysics2d::top_down().unwrap();
            if built {
                physics
                    .step(&mut world, extractor.components(), STEP)
                    .unwrap();
            }
            let mut sources = ScriptSources::new();
            sources.insert("joint.decay", r#"script Joint {
                var elapsed: f32 = 0.0;
                fn start() {
                    if !Physics.joint_enabled(this.entity) { Physics.set_joint_enabled(World.find("missing"), true); }
                    Physics.set_joint_enabled(this.entity, false);
                    if Physics.joint_enabled(this.entity) { Physics.set_joint_enabled(World.find("missing"), true); }
                }
                fn update(dt: f32) {
                    this.elapsed += dt;
                    if this.elapsed > 0.05 && !Physics.joint_enabled(this.entity) { Physics.set_joint_enabled(this.entity, true); }
                }
            }"#);
            let mut scripts = Scripts::new();
            let input = InputState::default();
            advance(
                &mut scripts,
                &mut world,
                &extractor,
                &sources,
                &input,
                &mut physics,
                true,
            );
            assert_eq!(
                world.get(owner).unwrap().components[kind]["enabled"],
                json!(false)
            );
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
            assert_eq!(physics.world().joint_count(), 0);
            physics
                .world_mut()
                .set_linear_velocity(moving, [3.0, 0.0])
                .unwrap();
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
            assert!(physics.world().linear_velocity(moving).unwrap()[0] > 2.9);
            for _ in 0..10 {
                advance(
                    &mut scripts,
                    &mut world,
                    &extractor,
                    &sources,
                    &input,
                    &mut physics,
                    true,
                );
                physics
                    .step(&mut world, extractor.components(), STEP)
                    .unwrap();
            }
            assert_eq!(physics.world().joint_count(), 1);
            assert_eq!(
                world.get(owner).unwrap().components[kind]["future_field"],
                json!(17)
            );
        }
    }
}

#[test]
fn distance_tuning_and_bad_joint_state_calls_preserve_unknown_fields_and_reject_atomically() {
    for (kind, call, succeeds, with_physics) in [
        (
            KINDS[0],
            "Physics.set_distance(this.entity, 2.0);",
            true,
            true,
        ),
        (
            KINDS[0],
            "Physics.set_distance(this.entity, 0.0);",
            false,
            true,
        ),
        (
            KINDS[0],
            "Physics.set_distance(this.entity, -1.0);",
            false,
            true,
        ),
        (
            KINDS[0],
            "Physics.set_distance(this.entity, 400000000000000000000000000000000000000.0);",
            false,
            true,
        ),
        (
            KINDS[1],
            "Physics.set_distance(this.entity, 2.0);",
            false,
            true,
        ),
        (
            KINDS[0],
            "Physics.set_joint_enabled(this.entity, 1.0);",
            false,
            true,
        ),
        (
            KINDS[0],
            "Physics.joint_enabled(this.entity);",
            false,
            false,
        ),
        (
            KINDS[0],
            "Physics.set_joint_enabled(this.entity, false);",
            false,
            false,
        ),
        (
            KINDS[0],
            "Physics.set_distance(this.entity, 2.0);",
            false,
            false,
        ),
    ] {
        exercise_distance_call(kind, call, succeeds, with_physics);
    }
}

fn exercise_distance_call(kind: &str, call: &str, succeeds: bool, with_physics: bool) {
    let (extractor, mut world, owner, _) = fixture(kind);
    let original = world.get(owner).unwrap().components[kind].clone();
    let mut sources = ScriptSources::new();
    sources.insert(
        "joint.decay",
        format!("script Joint {{ fn start() {{ {call} }} }}"),
    );
    let mut physics = ScenePhysics2d::top_down().unwrap();
    let input = InputState::default();
    let mut scripts = Scripts::new();
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
    let report = scripts.advance(&mut world, extractor.components(), frame);
    assert_eq!(
        report.failures.is_empty(),
        succeeds,
        "{call}: {:?}",
        report.failures
    );
    if call.contains("400000000000000000000000000000000000000.0") {
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.to_string().contains("finite"))
        );
    }
    let payload = &world.get(owner).unwrap().components[kind];
    if succeeds {
        assert_eq!(payload["future_field"], json!(17));
        assert_eq!(payload["max_distance"], json!(2.0));
        physics
            .step(&mut world, extractor.components(), STEP)
            .unwrap();
        assert_eq!(physics.world().joint_count(), 1);
    } else {
        assert_eq!(payload, &original);
    }
}

#[test]
fn missing_conflicting_or_invalid_joint_components_are_rejected_before_mutation() {
    for problem in ["missing", "conflict", "invalid"] {
        for call in [
            "Physics.joint_enabled(this.entity);",
            "Physics.set_joint_enabled(this.entity, false);",
            "Physics.set_distance(this.entity, 2.0);",
            "Physics.set_joint_endpoints(this.entity, null, null);",
            "Physics.remove_joint(this.entity);",
        ] {
            let (extractor, mut world, owner, _) = fixture(KINDS[0]);
            let components = &mut world.get_mut(owner).unwrap().components;
            match problem {
                "missing" => {
                    components.remove(KINDS[0]);
                }
                "conflict" => {
                    components.insert(KINDS[1].into(), json!({}));
                }
                _ => {
                    components.get_mut(KINDS[0]).unwrap()["enabled"] = json!("yes");
                }
            }
            let original = components.clone();
            let mut sources = ScriptSources::new();
            sources.insert(
                "joint.decay",
                format!("script Joint {{ fn start() {{ {call} }} }}"),
            );
            let mut physics = ScenePhysics2d::top_down().unwrap();
            let input = InputState::default();
            let mut scripts = Scripts::new();
            advance(
                &mut scripts,
                &mut world,
                &extractor,
                &sources,
                &input,
                &mut physics,
                false,
            );
            assert_eq!(world.get(owner).unwrap().components, original);
        }
    }
}

fn advance(
    scripts: &mut Scripts,
    world: &mut World,
    extractor: &SceneExtractor,
    sources: &ScriptSources,
    input: &InputState,
    physics: &mut ScenePhysics2d,
    succeeds: bool,
) {
    let (backend, events) = physics.for_scripts();
    let report = scripts.advance(
        world,
        extractor.components(),
        ScriptFrame::new(sources, input, 1.0 / 60.0).with_physics(Physics2d {
            world: backend,
            events,
        }),
    );
    assert_eq!(
        report.failures.is_empty(),
        succeeds,
        "{:?}",
        report.failures
    );
}

#[path = "joint_state_controls/endpoints.rs"]
mod endpoints;

#[path = "joint_state_controls/removal.rs"]
mod removal;

#[path = "joint_state_controls/creation.rs"]
mod creation;

#[path = "joint_state_controls/hinge_creation.rs"]
mod hinge_creation;

#[path = "joint_state_controls/spring_creation.rs"]
mod spring_creation;

#[path = "joint_state_controls/slider_creation.rs"]
mod slider_creation;

#[path = "joint_state_controls/hinge_position.rs"]
mod hinge_position;
