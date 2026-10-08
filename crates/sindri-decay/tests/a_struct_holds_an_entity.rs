//! An entity held in a struct's field is reached the same as one held in a
//! name: `pair.target.transform.position.x` reads and writes the target.
//!
//! The struct field is read as a value first and the path walks on from the
//! reference it holds. Before, the whole chain went to the host as one path
//! rooted at a local holding a struct, which the checker accepted and the
//! runtime refused: a RayHit2d's `entity` could be passed on, but not asked
//! where it was.

use serde_json::json;
use sindri_core::{ComponentSchemaRegistry, EntityData, SceneComponent, Transform3D, World};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

#[test]
fn a_path_walks_on_from_an_entity_in_a_struct_field() {
    let mut registry = ComponentSchemaRegistry::default();
    registry.register::<ScriptComponent>("Script").unwrap();
    let mut world = World::default();
    let target = world.spawn(EntityData {
        name: Some("Target".to_owned()),
        transform_3d: Some(Transform3D {
            position: [2.0, 0.0, 0.0],
            ..Transform3D::default()
        }),
        ..EntityData::default()
    });
    world.spawn(EntityData {
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({"source": "pair.decay", "script": "Pairing"}),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert(
        "pair.decay",
        r#"
        struct Pair { label: String, target: Entity }
        struct Nest { inner: Pair }

        script Pairing {
            fn start() {
                let pair = Pair(label: "it", target: World.find("Target") ?? this.entity);
                Game.set("read", pair.target.transform.position.x);
                pair.target.transform.position.y = 3.0;
                let nest = Nest(inner: pair);
                nest.inner.target.transform.position.z = pair.target.transform.position.x + 2.0;
            }
        }
        "#,
    );
    let mut scripts = Scripts::new();
    let report = scripts.advance(
        &mut world,
        &registry,
        ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
    );
    assert!(report.failures.is_empty(), "{report:#?}");
    assert!((scripts.blackboard().get("read", 0.0) - 2.0).abs() < 1.0e-9);
    let position = world.get(target).unwrap().transform_3d.unwrap().position;
    assert_eq!(position, [2.0, 3.0, 4.0]);
}
