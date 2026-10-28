//! A script asks the 2D lighting whether one place sees another and how bright
//! a place is, with a lamp and a wall in the scene.

use serde_json::json;
use sindri_core::{ComponentSchemaRegistry, EntityData, SceneComponent, Transform3D, World};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{
    EnvironmentComponent, Light2dComponent, Light2dOccluderComponent, LightField2d,
};

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
        .register::<EnvironmentComponent>("Environment")
        .expect("sindri.environment registers");
    registry
        .register::<Light2dComponent>("Light 2D")
        .expect("sindri.light2d registers");
    registry
        .register::<Light2dOccluderComponent>("Light 2D Occluder")
        .expect("sindri.light2d.occluder registers");
    registry
}

fn at(x: f32) -> Transform3D {
    Transform3D {
        position: [x, 0.0, 0.0],
        ..Transform3D::default()
    }
}

fn entity(name: &str, x: f32, component: &str, value: serde_json::Value) -> EntityData {
    EntityData {
        name: Some(name.to_owned()),
        transform_3d: Some(at(x)),
        components: [(component.to_owned(), value)].into_iter().collect(),
        ..EntityData::default()
    }
}

fn scene(lighting: bool) -> World {
    let mut world = World::default();
    world.spawn(entity(
        "Sky",
        0.0,
        EnvironmentComponent::TYPE_NAME,
        json!({ "lighting_2d": { "enabled": lighting, "ambient": [0.0, 0.0, 0.0] } }),
    ));
    world.spawn(entity(
        "Lamp",
        0.0,
        Light2dComponent::TYPE_NAME,
        json!({ "color": [1.0, 1.0, 1.0, 1.0], "intensity": 1.0, "radius": 6.0,
                "falloff": 1.0, "shadows": true }),
    ));
    world.spawn(entity(
        "Wall",
        2.0,
        Light2dOccluderComponent::TYPE_NAME,
        json!({ "size": [0.2, 2.0] }),
    ));
    world.spawn(entity(
        "Asker",
        0.0,
        ScriptComponent::TYPE_NAME,
        json!({ "source": "asker.decay", "script": "Asker" }),
    ));
    world
}

fn ask(source: &str, lighting: bool, gathered: bool) -> (World, sindri_decay::ScriptReport) {
    let mut world = scene(lighting);
    let components = registry();
    let field = LightField2d::gather(&world, &components).expect("the field gathers");
    assert_eq!(field.is_some(), lighting);
    let mut sources = ScriptSources::new();
    sources.insert("asker.decay", source);
    let input = InputState::default();
    let mut frame = ScriptFrame::new(&sources, &input, 1.0 / 60.0);
    if let (Some(field), true) = (&field, gathered) {
        frame = frame.with_lighting(field);
    }
    let report = Scripts::new().advance(&mut world, &components, frame);
    (world, report)
}

fn asker_x(world: &World) -> f32 {
    world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("Asker"))
        .and_then(|(_, data)| data.transform_3d)
        .expect("the asker is still there")
        .position[0]
}

#[test]
fn a_wall_blocks_the_line_of_sight_and_the_light() {
    let (world, report) = ask(
        r"
        script Asker {
            fn update(dt: f32) {
                var seen = 0.0;
                if Lighting.visible(Vec2(0.0, 0.0), Vec2(1.0, 0.0)) { seen = seen + 1.0; }
                if Lighting.visible(Vec2(0.0, 0.0), Vec2(4.0, 0.0)) { seen = seen + 10.0; }
                if Lighting.level(Vec2(1.0, 0.0)) > 0.5 { seen = seen + 100.0; }
                if Lighting.level(Vec2(4.0, 0.0)) < 0.01 { seen = seen + 1000.0; }
                this.transform.position.x = seen;
            }
        }
        ",
        true,
        true,
    );
    assert!(report.failures.is_empty(), "{report:#?}");
    // In front of the wall it sees and is lit; behind it neither, and it is dark.
    assert!(
        (asker_x(&world) - 1101.0).abs() < 1.0e-3,
        "{}",
        asker_x(&world)
    );
}

#[test]
fn asking_without_lighting_says_so() {
    let source = r"
        script Asker {
            fn update(dt: f32) {
                this.transform.position.x = Lighting.level(Vec2(1.0, 0.0));
            }
        }
        ";
    let (_, report) = ask(source, false, true);
    assert!(
        report
            .failures
            .iter()
            .any(|f| format!("{f:?}").contains("2D lighting")),
        "{report:#?}"
    );
}

#[test]
fn only_sources_that_name_the_lighting_ask_for_it() {
    let mut sources = ScriptSources::new();
    sources.insert("quiet.decay", "script Quiet { fn update(dt: f32) {} }");
    assert!(!sources.asks_lighting());
    sources.insert(
        "asker.decay",
        "script Asker { fn update(dt: f32) { Lighting.level(Vec2(0.0, 0.0)); } }",
    );
    assert!(sources.asks_lighting());
    sources.remove("asker.decay");
    assert!(!sources.asks_lighting(), "and it forgets when they go");
}
