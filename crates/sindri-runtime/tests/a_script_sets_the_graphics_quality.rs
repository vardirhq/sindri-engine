//! A settings menu choosing how far the game draws: a script asks for a
//! graphics quality, the host's scene draws at it, and the script reads it
//! back. A level the host chose itself is reported too.

use serde_json::json;
use sindri_core::{EntityData, SceneComponent, World};
use sindri_decay::{ScriptComponent, ScriptSources};
use sindri_platform::InputState;
use sindri_runtime::Session;
use sindri_scene::{GraphicsQuality, SceneExtractor};

const MENU: &str = r#"
script Menu {
    var asked: bool = false;

    fn update(dt: f32) {
        print(Graphics.quality);
        if !this.asked {
            this.asked = true;
            Graphics.set_quality("low");
        }
    }
}
"#;

#[test]
fn a_script_chooses_the_quality_and_reads_it_back() {
    let mut scene = SceneExtractor::new().expect("the engine's components register");
    scene
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    let mut sources = ScriptSources::new();
    sources.insert("scripts/menu.decay", MENU);
    let mut session = Session::with_sources(scene.components().clone(), sources);
    let mut world = World::default();
    world.spawn(EntityData {
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "scripts/menu.decay", "script": "Menu" }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let input = InputState::default();
    let mut step = |session: &mut Session, scene: &mut SceneExtractor| {
        let report = session
            .step(&mut world, &input, (640.0, 360.0), 1.0 / 60.0)
            .expect("steps");
        session.sync_graphics_quality(scene);
        report
            .scripts
            .printed
            .into_iter()
            .map(|line| line.message)
            .collect::<Vec<_>>()
    };

    assert_eq!(
        step(&mut session, &mut scene),
        ["high"],
        "authored, to start"
    );
    assert_eq!(
        scene.graphics_quality(),
        GraphicsQuality::Low,
        "the menu's choice"
    );
    assert_eq!(
        step(&mut session, &mut scene),
        ["low"],
        "and the script sees it"
    );

    // The host lowering it itself, as a struggling device's does, is heard.
    scene.set_graphics_quality(GraphicsQuality::Medium);
    session.sync_graphics_quality(&mut scene);
    assert_eq!(step(&mut session, &mut scene), ["medium"]);
}

#[test]
fn an_unknown_quality_is_an_error_naming_the_levels() {
    let checked = sindri_decay::check_source(
        r"script Menu { fn update(dt: f32) { Graphics.set_quality(3.0); } }",
    );
    assert!(!checked.compiles(), "a level is named, not numbered");
}
