//! `sindri.script#halo`: one object runs several scripts, each attached as a
//! named part, each with its own fields, and each seeing its own name's parts
//! first.

use serde_json::{Value, json};
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSlot, ScriptSources, ScriptValue, Scripts};
use sindri_platform::InputState;
use sindri_scene::SceneExtractor;

fn registry() -> ComponentSchemaRegistry {
    let mut registry = SceneExtractor::new()
        .expect("the builtin components register")
        .components()
        .clone();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

fn light() -> Value {
    json!({ "radius": 1.0, "intensity": 1.0, "color": [1.0, 1.0, 1.0, 1.0] })
}

fn flame(radius: f64) -> Value {
    json!({
        "source": "flame.decay",
        "script": "Flame",
        "properties": { "base_radius": radius },
    })
}

/// A lantern with a beam and a halo, each flickered by its own `Flame`, and a
/// keeper on another object that reaches them.
fn lantern(world: &mut World) -> (EntityId, EntityId) {
    let lantern = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [
            (ScriptComponent::TYPE_NAME.to_owned(), flame(8.0)),
            ("sindri.script#halo".to_owned(), flame(3.0)),
            ("sindri.light2d".to_owned(), light()),
            ("sindri.light2d#halo".to_owned(), light()),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let keeper = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "keeper.decay", "script": "Keeper" }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    (lantern, keeper)
}

const FLAME: &str = r"
script Flame {
    @export let base_radius: number = 1.0;
    var frames: number = 0.0;
    var dimmed: number = 0.0;

    fn update(dt: number) {
        this.frames += 1.0;
        this.light2d.radius = this.base_radius + this.frames;
    }

    fn dim(by: number) {
        this.dimmed += by;
        this.light2d.intensity = 1.0 - by;
    }
}
";

const KEEPER: &str = r#"
script Keeper {
    var lantern: Entity = null;
    var radius: number = 0.0;
    var x: number = 0.0;

    fn start() {
        this.lantern = World.find("lantern");
    }

    fn update(dt: number) {
        let beam = Flame.on(this.lantern);
        if beam != null {
            this.radius = beam.base_radius;
            this.x = beam.transform.position.x;
            beam.dim(0.25);
        }
    }
}
"#;

fn advance(scripts: &mut Scripts, world: &mut World, sources: &ScriptSources) {
    let input = InputState::default();
    let report = scripts.advance(
        world,
        &registry(),
        ScriptFrame::new(sources, &input, 1.0 / 60.0),
    );
    assert!(report.failures.is_empty(), "{:?}", report.failures);
}

#[test]
fn each_script_on_a_lantern_flickers_its_own_light() {
    let mut world = World::default();
    let (lantern, keeper) = lantern(&mut world);
    world.get_mut(lantern).unwrap().name = Some("lantern".to_owned());
    let mut sources = ScriptSources::new();
    sources.insert("flame.decay", FLAME);
    sources.insert("keeper.decay", KEEPER);
    let mut scripts = Scripts::new();
    for _ in 0..2 {
        advance(&mut scripts, &mut world, &sources);
    }

    let light = |key: &str| world.get(lantern).unwrap().components[key].clone();
    assert_eq!(
        light("sindri.light2d")["radius"],
        json!(10.0),
        "8 + 2 frames"
    );
    assert_eq!(
        light("sindri.light2d#halo")["radius"],
        json!(5.0),
        "the halo's Flame reads light2d as its own part: 3 + 2 frames"
    );
    let halo = ScriptSlot::new(lantern, Some("halo".to_owned()));
    assert_eq!(
        scripts.slot_field(&halo, "frames"),
        Some(&ScriptValue::Number(2.0)),
        "each script keeps its own fields"
    );

    // `Flame.on` answers the lantern's own Flame first, and a message to it
    // reaches that one alone.
    assert_eq!(
        scripts.field(keeper, "radius"),
        Some(&ScriptValue::Number(8.0))
    );
    assert_eq!(
        scripts.field(lantern, "dimmed"),
        Some(&ScriptValue::Number(0.5))
    );
    assert_eq!(
        scripts.slot_field(&halo, "dimmed"),
        Some(&ScriptValue::Number(0.0))
    );
    assert_eq!(light("sindri.light2d")["intensity"], json!(0.75));
    assert_eq!(light("sindri.light2d#halo")["intensity"], json!(1.0));
}

#[test]
fn a_script_part_is_found_by_its_script_and_reached_by_reference() {
    let mut world = World::default();
    let (lantern, keeper) = lantern(&mut world);
    world.get_mut(lantern).unwrap().name = Some("lantern".to_owned());
    // The lantern's own script is something else, so `Flame.on` finds the
    // halo's.
    world
        .get_mut(lantern)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position[0] = 4.0;
    world.get_mut(lantern).unwrap().components.insert(
        ScriptComponent::TYPE_NAME.to_owned(),
        json!({ "source": "keeper.decay", "script": "Idle" }),
    );
    let mut sources = ScriptSources::new();
    sources.insert("flame.decay", FLAME);
    sources.insert(
        "keeper.decay",
        format!("{KEEPER}\nscript Idle {{ var quiet: number = 0.0; }}"),
    );
    let mut scripts = Scripts::new();
    for _ in 0..2 {
        advance(&mut scripts, &mut world, &sources);
    }
    let halo = ScriptSlot::new(lantern, Some("halo".to_owned()));
    assert_eq!(
        scripts.field(keeper, "radius"),
        Some(&ScriptValue::Number(3.0)),
        "read through the part's reference"
    );
    assert_eq!(
        scripts.field(keeper, "x"),
        Some(&ScriptValue::Number(4.0)),
        "which also stands for the lantern"
    );
    assert_eq!(
        scripts.slot_field(&halo, "dimmed"),
        Some(&ScriptValue::Number(0.5)),
        "and messaged through it"
    );
    assert!(scripts.is_running(lantern));
}
