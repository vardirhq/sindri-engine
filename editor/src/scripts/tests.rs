//! The editor's script loading, against real files.

use std::{collections::BTreeMap, fs, thread::sleep, time::Duration};

use serde_json::json;
use sindri_core::{ComponentSchemaRegistry, EntityData, SceneComponent, World};
use sindri_decay::ScriptComponent;
use tempfile::TempDir;

use super::{SceneScripts, ScriptFailure};

/// A world holding one entity that runs `source`, and a registry that knows
/// what `sindri.script` is.
fn scripted(source: &str) -> (World, ComponentSchemaRegistry) {
    let mut components = ComponentSchemaRegistry::default();
    components
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers once");
    let mut world = World::default();
    world.spawn(EntityData {
        name: Some("Thing".to_owned()),
        components: BTreeMap::from([(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": source, "script": "Thing" }),
        )]),
        ..EntityData::default()
    });
    (world, components)
}

/// A scene directory holding one script, and the scene path inside it.
fn project(script: &str, text: &str) -> TempDir {
    let directory = TempDir::new().expect("a temporary directory");
    let scripts = directory.path().join("scripts");
    fs::create_dir_all(&scripts).expect("the scripts directory is creatable");
    fs::write(scripts.join(script), text).expect("the script is writable");
    directory
}

/// The bug this guards: a cold open reported one error per scripted entity
/// for the moment between the scene landing and its scripts arriving, and
/// the console keeps what it is told, so twelve phantom errors sat in the
/// status bar of a game that was working.
#[test]
fn a_script_still_loading_is_not_an_error() {
    let directory = project("thing.decay", "script Thing {\n    fn update() {}\n}\n");
    let scene = directory.path().join("thing.scene");
    let (world, components) = scripted("scripts/thing.decay");

    let mut scripts = SceneScripts::for_scene(Some(&scene));
    scripts.request(&world, &components);

    // Before anything is polled the source cannot have arrived, which is
    // exactly the window the editor used to report.
    assert!(
        scripts.compile(&world, &components).is_empty(),
        "a source still in flight is not a compile failure"
    );

    // And once it lands it compiles, so the silence above was not the
    // failure being swallowed for good.
    for _ in 0..200 {
        scripts.poll();
        if scripts.compile(&world, &components).is_empty()
            && scripts.exports("scripts/thing.decay", "Thing").is_some()
        {
            return;
        }
        sleep(Duration::from_millis(10));
    }
    panic!("the script never arrived");
}

/// The other half: a source that will never arrive is still reported, so
/// suppressing the in-flight case did not suppress a real typo.
#[test]
fn a_script_that_will_never_arrive_is_an_error() {
    let directory = project("thing.decay", "script Thing {\n    fn update() {}\n}\n");
    let scene = directory.path().join("thing.scene");
    let (world, components) = scripted("scripts/absent.decay");

    let mut scripts = SceneScripts::for_scene(Some(&scene));
    scripts.request(&world, &components);

    for _ in 0..200 {
        scripts.poll();
        let failures = scripts.compile(&world, &components);
        if failures
            .iter()
            .any(|failure| matches!(failure, ScriptFailure::MissingSource { .. }))
        {
            return;
        }
        sleep(Duration::from_millis(10));
    }
    panic!("a script that does not exist was never reported");
}

/// The bug this guards: a project with more scripts than the loader would
/// queue had the rest refused, and what did arrive compiled against half a
/// project — `Game` without its `state`, a script without the shared function
/// it calls — so a working game opened with a console full of errors.
#[test]
fn a_large_project_loads_whole_before_anything_compiles() {
    let directory = project(
        "thing.decay",
        "script Thing {\n    fn update() {\n        Game.score = twice(Game.score);\n    }\n}\n",
    );
    let scripts_dir = directory.path().join("scripts");
    // Written after the script that needs them, and far more of them than the
    // sixteen the loader used to hold.
    for index in 0..60 {
        fs::write(
            scripts_dir.join(format!("filler-{index:02}.decay")),
            format!("script Filler{index} {{\n    fn update() {{}}\n}}\n"),
        )
        .expect("a filler script is writable");
    }
    fs::write(
        scripts_dir.join("zz-shared.decay"),
        "shared fn twice(x: f32) -> f32 {\n    return x * 2.0;\n}\n",
    )
    .expect("the shared script is writable");
    fs::write(
        scripts_dir.join("zz-state.decay"),
        "state Game {\n    var score: f32 = 1.0;\n}\n",
    )
    .expect("the state script is writable");
    // A prefab nothing in the scene names, as an enemy's drop would be.
    let prefabs = directory.path().join("prefabs");
    fs::create_dir_all(&prefabs).expect("the prefabs directory is creatable");
    fs::write(
        prefabs.join("drop.prefab"),
        r#"{"format_version":1,"entities":[{"id":"drop","name":"Drop"}]}"#,
    )
    .expect("the prefab is writable");

    let scene = directory.path().join("thing.scene");
    let (world, components) = scripted("scripts/thing.decay");
    let mut scripts = SceneScripts::for_scene(Some(&scene));
    let notes = scripts.request(&world, &components);
    assert!(
        !notes
            .iter()
            .any(|note| matches!(note, super::ScriptNote::Failed(_))),
        "nothing in a project is refused: {notes:?}"
    );

    for _ in 0..500 {
        scripts.poll();
        let failures = scripts.compile(&world, &components);
        assert!(
            failures.is_empty(),
            "compiled against part of the project: {failures:?}"
        );
        if !scripts.loading() && scripts.exports("scripts/thing.decay", "Thing").is_some() {
            assert!(
                scripts.has_prefab("prefabs/drop.prefab"),
                "a prefab only another prefab could name is loaded too"
            );
            return;
        }
        sleep(Duration::from_millis(10));
    }
    panic!("the project never finished loading");
}
