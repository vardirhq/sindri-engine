//! A scene authors a list or a struct `@export` as JSON, read by the type
//! the script declared, and the inspector is told what is inside each.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFailure, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

const SOURCE: &str = r"enum Kind { Coin, Gem }
struct Drop { name: String, weight: f32, kind: Kind }
struct Tuning { speed: f32, jump: f32 }
script Chest {
    @export var waypoints: List<Vec2> = [];
    @export var loot: List<Drop> = [];
    @export var tuning = Tuning(speed: 2.0, jump: 3.0);
    fn update(dt: f32) {
        var total = 0.0;
        for drop in loot {
            if drop.kind == Kind.Gem { total += drop.weight * 10.0; } else { total += drop.weight; }
        }
        this.transform.position.x = total;
        this.transform.position.y = waypoints.length * 100.0 + waypoints[1].y;
        this.transform.position.z = tuning.speed * 10.0 + tuning.jump;
    }
}";

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

fn chest(world: &mut World, properties: &serde_json::Value) -> EntityId {
    world.spawn(EntityData {
        name: Some("Chest".to_owned()),
        transform_3d: Some(Transform3D::default()),
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "chest.decay", "script": "Chest", "properties": properties }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    })
}

fn frame(world: &mut World, scripts: &mut Scripts) -> Vec<ScriptFailure> {
    let mut sources = ScriptSources::new();
    sources.insert("chest.decay", SOURCE);
    scripts
        .advance(
            world,
            &registry(),
            ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
        )
        .failures
}

#[test]
fn a_scene_authors_lists_and_structs() {
    let mut world = World::default();
    let entity = chest(
        &mut world,
        &json!({
            "waypoints": [[1.0, 2.0], [3.0, 4.0]],
            "loot": [
                { "name": "coin", "weight": 2.0, "kind": "Coin" },
                { "name": "gem", "weight": 1.5, "kind": "Gem" }
            ],
            // `jump` left out keeps the script's 3.
            "tuning": { "speed": 5.0 }
        }),
    );
    let mut scripts = Scripts::new();
    let failures = frame(&mut world, &mut scripts);
    assert!(failures.is_empty(), "{failures:?}");
    let position = world
        .get(entity)
        .and_then(|data| data.transform_3d)
        .expect("a transform")
        .position;
    assert!((position[0] - 17.0).abs() < 1e-5, "{position:?}");
    assert!((position[1] - 204.0).abs() < 1e-5, "{position:?}");
    assert!((position[2] - 53.0).abs() < 1e-5, "{position:?}");
}

#[test]
fn a_wrong_authored_list_or_struct_is_refused_with_why() {
    for (properties, expected) in [
        (json!({ "loot": { "name": "coin" } }), "is not a list"),
        (
            json!({ "loot": [{ "name": "coin", "colour": 1 }] }),
            "item 0: `Drop` has no field `colour`; it has name, weight, kind",
        ),
        (
            json!({ "loot": [{ "kind": "Ruby" }] }),
            "item 0: `kind`: \"Ruby\" is not a `Kind`; it is one of Coin, Gem",
        ),
        (json!({ "tuning": 3.0 }), "3.0 is not a `Tuning`"),
    ] {
        let mut world = World::default();
        chest(&mut world, &properties);
        let mut scripts = Scripts::new();
        let failures = frame(&mut world, &mut scripts);
        assert!(
            failures.iter().any(|failure| matches!(
                failure,
                ScriptFailure::Property { reason, .. } if reason.contains(expected)
            )),
            "{properties} should be refused with {expected:?}: {failures:?}"
        );
    }
}

#[test]
fn the_inspector_is_told_what_a_list_and_a_struct_hold() {
    let mut world = World::default();
    chest(
        &mut world,
        &json!({ "waypoints": [[0.0, 0.0], [0.0, 0.0]] }),
    );
    let mut scripts = Scripts::new();
    let failures = frame(&mut world, &mut scripts);
    assert!(failures.is_empty(), "{failures:?}");
    let exports = scripts.exports("chest.decay", "Chest").expect("compiled");

    let loot = &exports[1];
    assert_eq!(loot.type_name.as_deref(), Some("List<Drop>"));
    let element = loot.element.as_ref().expect("a list says what it holds");
    assert_eq!(element.type_name.as_deref(), Some("Drop"));
    let names: Vec<_> = element
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    assert_eq!(names, ["name", "weight", "kind"]);
    assert_eq!(element.fields[2].choices, ["Coin", "Gem"]);

    let tuning = &exports[2];
    assert_eq!(tuning.type_name.as_deref(), Some("Tuning"));
    assert_eq!(tuning.fields.len(), 2);
    assert_eq!(
        tuning.fields[1].default,
        sindri_decay::ScriptValue::Number(3.0)
    );
}
