//! Typed queries execute through Decay and return ordinary entity handles.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, SceneComponent, TagsComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts, check_source};
use sindri_platform::InputState;

#[test]
fn a_script_uses_nearest_and_walks_a_radius_snapshot() {
    let mut world = World::default();
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("register");
    for y in [3.0, 1.0, 2.0] {
        world.spawn(EntityData {
            transform_3d: Some(Transform3D {
                position: [0.0, y, 0.0],
                ..Transform3D::default()
            }),
            components: [(
                TagsComponent::TYPE_NAME.to_owned(),
                json!({ "tags": ["enemy"] }),
            )]
            .into_iter()
            .collect(),
            ..EntityData::default()
        });
    }
    world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "observer.decay", "script": "Observer" }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert(
        "observer.decay",
        r#"
        script Observer {
            fn start() {
                let origin = this.transform.world_position;
                let closest: Entity = World.nearest("enemy", origin);
                print(closest.transform.world_position.y);
                print(World.nearest("missing", origin) == null);
                let nearby: List<Entity> = World.within_radius("enemy", origin, 2.0);
                var total: f32 = 0.0;
                var weight: f32 = 1.0;
                for enemy in nearby {
                    total += enemy.transform.world_position.y * weight;
                    weight *= 10.0;
                    World.despawn(enemy);
                }
                print(total);
                print(nearby.length);
                for enemy in nearby { print(World.exists(enemy)); }
            }
        }
    "#,
    );
    let report = Scripts::new().advance(
        &mut world,
        &registry,
        ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
    );
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    let printed: Vec<_> = report
        .printed
        .iter()
        .map(|message| message.message.as_str())
        .collect();
    assert_eq!(printed, ["1", "true", "21", "2", "false", "false"]);
}

#[test]
fn spatial_signatures_are_checked_before_runtime() {
    for call in [
        r#"World.nearest(1.0, Vec3(0.0, 0.0, 0.0))"#,
        r#"World.nearest("enemy", Vec2(0.0, 0.0))"#,
        r#"World.within_radius("enemy", Vec3(0.0, 0.0, 0.0), true)"#,
        r#"World.within_radius("enemy", Vec3(0.0, 0.0, 0.0))"#,
    ] {
        let source = format!("script Observer {{ fn start() {{ {call}; }} }}");
        let checked = check_source(&source);
        assert!(!checked.diagnostics.is_empty(), "accepted {call}");
    }
}
