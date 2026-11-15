//! A collider's pieces by name: something that entered a sensor piece is told
//! apart from something that bumped the body, and a contact says which piece
//! of each collider touches (`docs/object-parts.md`, step 5).

use std::time::Duration;

use serde_json::{Value, json};
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{Physics2d, ScriptComponent, ScriptFrame, ScriptSources, ScriptValue, Scripts};
use sindri_physics::{
    Collider2d, PhysicsEvent2d, PhysicsEventKind, PhysicsPose2d, PhysicsWorld2d, RigidBody2d,
};
use sindri_platform::InputState;

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

/// An entity running `script`, with `collider` as its authored collider.
fn spawn(world: &mut World, script: Option<&str>, collider: Value) -> EntityId {
    let mut components = vec![("sindri.physics2d.collider".to_owned(), collider)];
    if let Some(script) = script {
        components.push((
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "pieces.decay", "script": script }),
        ));
    }
    world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: components.into_iter().collect(),
        ..EntityData::default()
    })
}

fn run(
    scripts: &mut Scripts,
    world: &mut World,
    source: &str,
    physics: &mut PhysicsWorld2d,
    events: &[PhysicsEvent2d],
) {
    let mut sources = ScriptSources::new();
    sources.insert("pieces.decay", source);
    let input = InputState::default();
    let report = scripts.advance(
        world,
        &registry(),
        ScriptFrame::new(&sources, &input, 1.0 / 60.0).with_physics(Physics2d {
            world: physics,
            events,
        }),
    );
    assert!(report.failures.is_empty(), "{:?}", report.failures);
}

fn number(scripts: &Scripts, entity: EntityId, field: &str) -> f64 {
    match scripts.field(entity, field) {
        Some(ScriptValue::Number(value)) => *value,
        other => panic!("{field}: {other:?}"),
    }
}

const GLOOM: &str = r#"
script Gloom {
    var sniffed: number = 0.0;
    var bumped: number = 0.0;
    var anything: number = 0.0;

    fn update(dt: number) {
        this.sniffed += Physics.sensor_entered_at("nose").len;
        this.bumped += Physics.collision_started_at("body").len;
        this.anything += Physics.sensor_entered().len + Physics.collision_started().len;
        if Physics.sensor_entered_at("body").len > 0.0 {
            this.anything += 100.0;
        }
    }
}
"#;

#[test]
fn an_event_on_one_piece_is_told_from_one_on_another() {
    let mut world = World::default();
    let gloom = spawn(
        &mut world,
        Some("Gloom"),
        json!({ "pieces": [
            { "name": "body", "shape": { "shape": "circle", "radius": 0.5 } },
            { "name": "nose", "shape": { "shape": "circle", "radius": 2.0 }, "sensor": true },
        ]}),
    );
    let wick = world.spawn(EntityData::default());
    let crate_ = world.spawn(EntityData::default());
    let mut physics = PhysicsWorld2d::new([0.0, 0.0]).expect("a world");
    // Wick walked into the nose; a crate bumped the body. The gloom is the
    // second half of one event and the first of the other.
    let events = [
        PhysicsEvent2d {
            first: wick.min(gloom),
            second: wick.max(gloom),
            kind: PhysicsEventKind::SensorEntered,
            first_piece: u16::from(gloom < wick),
            second_piece: u16::from(gloom >= wick),
        },
        PhysicsEvent2d {
            first: gloom.min(crate_),
            second: gloom.max(crate_),
            kind: PhysicsEventKind::CollisionStarted,
            first_piece: 0,
            second_piece: 0,
        },
    ];
    let mut scripts = Scripts::new();
    run(&mut scripts, &mut world, GLOOM, &mut physics, &events);
    assert!(
        (number(&scripts, gloom, "sniffed") - 1.0).abs() < 1e-9,
        "the nose"
    );
    assert!(
        (number(&scripts, gloom, "bumped") - 1.0).abs() < 1e-9,
        "the body"
    );
    assert!(
        (number(&scripts, gloom, "anything") - 2.0).abs() < 1e-9,
        "the unnamed calls answer every piece, and the body sensed nothing"
    );
}

const FEET: &str = r#"
script Feet {
    var piece: String = "";
    var under: String = "";

    fn update(dt: number) {
        for contact in Physics.contacts(this.entity) {
            this.piece = contact.piece;
            this.under = contact.other_piece;
        }
    }
}
"#;

#[test]
fn a_contact_names_the_piece_of_each_collider_that_touches() {
    let mut world = World::default();
    let walker = spawn(
        &mut world,
        Some("Feet"),
        json!({ "pieces": [
            { "name": "head", "shape": { "shape": "circle", "radius": 0.3 }, "offset": [0.0, 3.0] },
            { "name": "feet", "shape": { "shape": "circle", "radius": 0.5 } },
        ]}),
    );
    let ground = spawn(
        &mut world,
        None,
        json!({ "name": "floor", "shape": { "shape": "box", "half_extents": [5.0, 0.5] } }),
    );
    let mut physics = PhysicsWorld2d::new([0.0, -10.0]).expect("a world");
    let mut head = Collider2d::circle(0.3);
    head.offset = [0.0, 3.0];
    physics
        .insert_body(
            walker,
            RigidBody2d {
                pose: PhysicsPose2d {
                    position: [0.0, 1.0],
                    rotation: 0.0,
                },
                ..RigidBody2d::default()
            },
            &[head, Collider2d::circle(0.5)],
        )
        .expect("the walker");
    physics
        .insert_static_collider(
            ground,
            PhysicsPose2d::default(),
            &[Collider2d::rectangle([5.0, 0.5])],
        )
        .expect("the ground");
    for _ in 0..60 {
        physics
            .step(Duration::from_secs_f32(1.0 / 60.0))
            .expect("a step");
    }
    let mut scripts = Scripts::new();
    run(&mut scripts, &mut world, FEET, &mut physics, &[]);
    assert_eq!(
        scripts.field(walker, "piece"),
        Some(&ScriptValue::String("feet".to_owned())),
        "its feet touch"
    );
    assert_eq!(
        scripts.field(walker, "under"),
        Some(&ScriptValue::String("floor".to_owned())),
        "a single collider is one piece, named by its own `name`"
    );
}
