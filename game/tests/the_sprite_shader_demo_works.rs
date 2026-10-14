//! Sprite Shaders, opened off disk and played the way the export host plays
//! it: every shader the scene names compiles against the engine's surface, and
//! the scripts drive the parameters those shaders read.

use std::{path::PathBuf, time::Duration};

use sindri_causeway::{Session, extractor};
use sindri_core::{EntityId, LoadedScenes, SceneDocument, SceneEntityId, World};
use sindri_decay::{ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{SpriteShaderBindings, referenced_sprite_shaders, unresolved_sprite_shaders};

const STEP: f32 = 1.0 / 60.0;
const SCENE: &str = "sprite-shaders.scene";
const SCRIPT: &str = "scripts/shader-demo.decay";

fn root() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../examples/sprite-shaders/assets"
    ))
}

fn read(id: &str) -> String {
    std::fs::read_to_string(root().join(id)).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn open() -> (World, SceneDocument) {
    let document = SceneDocument::from_json(&read(SCENE)).expect("the scene parses");
    let mut world = World::default();
    LoadedScenes::new()
        .enter_keeping_identities(&mut world, SCENE, &document)
        .expect("the scene loads");
    (world, document)
}

fn params(world: &World, name: &str) -> Vec<f64> {
    let id: EntityId = world
        .entity_for_source_id(&SceneEntityId::new(name).expect("a valid id"))
        .unwrap_or_else(|| panic!("no {name}"));
    world.get(id).expect("there").components["sindri.sprite"]["shader_params"]
        .as_array()
        .expect("four numbers")
        .iter()
        .map(|value| value.as_f64().expect("a number"))
        .collect()
}

#[test]
fn every_shader_the_scene_names_compiles() {
    let (world, _) = open();
    let named = referenced_sprite_shaders(&world);
    assert_eq!(named.len(), 4, "{named:?}");
    let mut bindings = SpriteShaderBindings::default();
    for path in &named {
        bindings
            .bind(path, &read(path))
            .unwrap_or_else(|error| panic!("{error}"));
    }
    assert!(unresolved_sprite_shaders(&world, &bindings).is_empty());
}

#[test]
fn the_scripts_drive_the_shader_parameters() {
    let (mut world, document) = open();
    let mut sources = ScriptSources::new();
    sources.insert(SCRIPT, read(SCRIPT));
    let registry = extractor().expect("the schemas register");
    let failures = Scripts::new().compile(&world, registry.components(), &sources);
    assert!(failures.is_empty(), "{failures:?}");
    let mut session = Session::with_sources(registry.components().clone(), sources)
        .with_scenes(vec![(SCENE.to_owned(), document)], LoadedScenes::new());
    let mut input = InputState::default();
    let mut play = |world: &mut World, steps: usize| {
        for _ in 0..steps {
            session
                .step(world, &input, (1280.0, 720.0), STEP)
                .expect("the step runs")
                .log();
            input.begin_frame(Duration::from_secs_f32(STEP));
        }
    };

    play(&mut world, 1);
    assert!(params(&world, "flash")[0] > 0.9, "the first hit flashes");
    assert!(
        params(&world, "dissolve")[0] < 0.01,
        "the burn starts untouched"
    );

    // Part way: a fraction, not rounded to a whole number. A payload authored
    // as `0` rather than `0.0` is written back as an integer, which would
    // turn every burn into on or off.
    play(&mut world, 44);
    let burn = params(&world, "dissolve")[0];
    assert!(
        burn > 0.2 && burn < 0.8,
        "half way through the burn: {burn}"
    );

    // Half the burn's three seconds: as far gone as it gets.
    play(&mut world, 46);
    assert!(
        params(&world, "dissolve")[0] > 0.99,
        "{:?}",
        params(&world, "dissolve")
    );
    let flash = params(&world, "flash")[0];
    assert!(flash < 1.0, "the flash fades between hits: {flash}");
    assert_eq!(
        params(&world, "wave"),
        [0.06, 2.0, 0.0, 0.0],
        "authored, not scripted"
    );
}
