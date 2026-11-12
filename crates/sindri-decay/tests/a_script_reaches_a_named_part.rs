//! `this.entity.light2d("halo")`: a script reaches a named part of its object,
//! holds it in a field across frames, and finds no part where there is none.

use serde_json::{Value, json};
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts};
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

fn lantern(world: &mut World) -> EntityId {
    world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [
            (
                ScriptComponent::TYPE_NAME.to_owned(),
                json!({ "source": "lantern.decay", "script": "Lantern" }),
            ),
            (
                "sindri.light2d".to_owned(),
                json!({ "radius": 1.0, "color": [1.0, 1.0, 1.0, 1.0] }),
            ),
            (
                "sindri.light2d#halo".to_owned(),
                json!({ "radius": 1.0, "color": [1.0, 1.0, 1.0, 1.0] }),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    })
}

const LANTERN: &str = r#"
script Lantern {
    var halo: Light2d = null;
    var missing: number = 0.0;
    var same: number = 0.0;
    var frames: number = 0.0;

    fn start() {
        this.halo = this.entity.light2d("halo");
    }

    fn update(dt: number) {
        this.frames += 1.0;
        this.light2d.radius = 10.0;
        if this.halo != null {
            this.halo.radius = 4.0 + this.frames;
            this.halo.color.a = 0.5;
        }
        if this.entity.light2d("glow") == null {
            this.missing += 1.0;
        }
        if this.light2d("halo") == this.halo {
            this.same += 1.0;
        }
    }
}
"#;

fn light(world: &World, entity: EntityId, key: &str) -> Value {
    world.get(entity).expect("the lantern").components[key].clone()
}

#[test]
fn a_script_reaches_its_objects_named_light_across_frames() {
    let mut world = World::default();
    let lantern = lantern(&mut world);
    let mut sources = ScriptSources::new();
    sources.insert("lantern.decay", LANTERN);
    let input = InputState::default();
    let registry = registry();
    let mut scripts = Scripts::new();
    for _ in 0..2 {
        let report = scripts.advance(
            &mut world,
            &registry,
            ScriptFrame::new(&sources, &input, 1.0 / 60.0),
        );
        assert!(report.failures.is_empty(), "{:?}", report.failures);
    }

    let beam = light(&world, lantern, "sindri.light2d");
    let halo = light(&world, lantern, "sindri.light2d#halo");
    assert_eq!(
        beam["radius"],
        json!(10.0),
        "the unnamed light is the entity's own"
    );
    assert_eq!(
        beam["color"][3],
        json!(1.0),
        "and the halo's writes stay off it"
    );
    assert_eq!(
        halo["radius"],
        json!(6.0),
        "the held part was written each frame"
    );
    assert_eq!(halo["color"][3], json!(0.5));
    assert_eq!(
        scripts.field(lantern, "missing"),
        Some(&sindri_decay::ScriptValue::Number(2.0)),
        "no glow, so null"
    );
    assert_eq!(
        scripts.field(lantern, "same"),
        Some(&sindri_decay::ScriptValue::Number(2.0)),
        "the same part is the same reference"
    );
}

const GLOOM: &str = r#"
script Gloom {
    var surge: Particles = null;
    var thrown: number = 0.0;

    fn start() {
        this.surge = this.entity.particles("surge");
    }

    fn update(dt: number) {
        this.particles.stop();
        if this.surge != null {
            this.surge.play();
            this.surge.rate = 40.0;
            this.thrown = this.surge.emit(3.0);
        }
    }
}
"#;

#[test]
fn a_script_plays_and_throws_from_a_named_emitter() {
    let mut world = World::default();
    let gloom = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [
            (
                ScriptComponent::TYPE_NAME.to_owned(),
                json!({ "source": "gloom.decay", "script": "Gloom" }),
            ),
            (
                "sindri.particles".to_owned(),
                json!({ "texture": "smoke.png", "playing": true, "rate": 7.0 }),
            ),
            (
                "sindri.particles#surge".to_owned(),
                json!({ "texture": "spark.png", "playing": false, "rate": 1.0, "speed": 0.0 }),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert("gloom.decay", GLOOM);
    let input = InputState::default();
    let registry = registry();
    let mut effects = sindri_scene::Effects2d::default();
    let mut scripts = Scripts::new();
    let report = scripts.advance(
        &mut world,
        &registry,
        ScriptFrame::new(&sources, &input, 1.0 / 60.0).with_effects(&mut effects),
    );
    assert!(report.failures.is_empty(), "{:?}", report.failures);

    let smoke = light(&world, gloom, "sindri.particles");
    let surge = light(&world, gloom, "sindri.particles#surge");
    assert_eq!(smoke["playing"], json!(false), "its own emitter stopped");
    assert_eq!(surge["playing"], json!(true), "the surge plays");
    assert_eq!(surge["rate"], json!(40.0));
    assert_eq!(
        scripts.field(gloom, "thrown"),
        Some(&sindri_decay::ScriptValue::Number(3.0))
    );
    assert_eq!(
        effects.part_live(gloom, "surge"),
        3,
        "thrown from the surge"
    );
    assert_eq!(effects.emitter_live(gloom), 0, "and not from its own");
}
