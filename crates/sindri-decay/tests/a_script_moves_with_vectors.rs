//! Vectors against a real world: a script reads another entity's position as
//! one value, does arithmetic on it, and writes its own back whole.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFailure, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

/// A named target at `(3, 4, 0)` and a scripted mover at the origin.
fn world(script: &str) -> (World, EntityId, ScriptSources) {
    let mut world = World::default();
    world.spawn(EntityData {
        name: Some("Target".to_owned()),
        transform_3d: Some(Transform3D {
            position: [3.0, 4.0, 0.0],
            ..Transform3D::default()
        }),
        ..EntityData::default()
    });
    let mover = world.spawn(EntityData {
        name: Some("Mover".to_owned()),
        transform_3d: Some(Transform3D::default()),
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "m.decay", "script": "M" }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert("m.decay", script);
    (world, mover, sources)
}

fn run(world: &mut World, sources: &ScriptSources) -> Vec<ScriptFailure> {
    let mut scripts = Scripts::new();
    scripts
        .advance(
            world,
            &registry(),
            ScriptFrame::new(sources, &InputState::default(), 1.0 / 60.0),
        )
        .failures
}

fn transform(world: &World, entity: EntityId) -> Transform3D {
    world
        .get(entity)
        .and_then(|data| data.transform_3d)
        .expect("the entity kept its transform")
}

fn assert_close(actual: [f32; 3], expected: [f32; 3]) {
    for (a, b) in actual.iter().zip(expected) {
        assert!((a - b).abs() < 1e-5, "{actual:?} is not {expected:?}");
    }
}

#[test]
fn a_script_steps_towards_another_entity_by_vector() {
    let (mut world, mover, sources) = world(
        "script M {
            fn update(dt: f32) {
                let target = World.find(\"Target\");
                let to = target.transform.position - this.transform.position;
                this.transform.position += to.normalized * 2.5;
                this.transform.scale = Vec3(2.0, 2.0, 2.0);
            }
        }",
    );
    let failures = run(&mut world, &sources);
    assert!(failures.is_empty(), "{failures:?}");
    let moved = transform(&world, mover);
    assert_close(moved.position, [1.5, 2.0, 0.0]);
    assert_close(moved.scale, [2.0, 2.0, 2.0]);
}

#[test]
fn the_maths_a_script_can_do() {
    let (mut world, mover, sources) = world(
        "script M {
            fn update(dt: f32) {
                this.transform.position = Vec3(
                    floor(2.7) + ceil(0.2),
                    clamp(9.0, 3.0, 0.0),
                    lerp(0.0, 10.0, 0.25) + sign(-4.0) + round(PI) - TAU / TAU
                );
            }
        }",
    );
    let failures = run(&mut world, &sources);
    assert!(failures.is_empty(), "{failures:?}");
    assert_close(transform(&world, mover).position, [3.0, 3.0, 3.5]);
}
