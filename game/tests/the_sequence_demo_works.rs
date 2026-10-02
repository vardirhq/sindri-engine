//! Sequence Stage, played the way a browser plays it: read off disk and run
//! through the session every export runs. The intro is authored in the
//! scene, not in a script, so this checks the choreography itself arrives —
//! the ship lands, the title fades up — and that the script hears the end
//! and moves the stage on to its idle loop, and replays on a press.

use std::{path::PathBuf, time::Duration};

use sindri_causeway::{Session, extractor};
use sindri_core::{EntityId, LoadedScenes, SceneDocument, SceneEntityId, World};
use sindri_decay::{ScriptSources, Scripts};
use sindri_platform::{InputEvent, InputState, Key};

const STEP: f32 = 1.0 / 60.0;

struct Stage {
    session: Session,
    world: World,
    input: InputState,
}

impl Stage {
    fn open() -> Self {
        let root = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sequence/assets"
        ));
        let read = |id: &str| std::fs::read_to_string(root.join(id)).expect("the file reads");
        let scene = "sequence.scene";
        let script = "scripts/director.decay";
        let document = SceneDocument::from_json(&read(scene)).expect("the scene parses");
        let mut world = World::default();
        LoadedScenes::new()
            .enter_keeping_identities(&mut world, scene, &document)
            .expect("the scene loads");
        let mut sources = ScriptSources::new();
        sources.insert(script, read(script));
        let registry = extractor().expect("the schemas register");
        let failures = Scripts::new().compile(&world, registry.components(), &sources);
        assert!(failures.is_empty(), "{failures:?}");
        let session = Session::with_sources(registry.components().clone(), sources)
            .with_scenes(vec![(scene.to_owned(), document)], LoadedScenes::new());
        Self {
            session,
            world,
            input: InputState::default(),
        }
    }

    fn play_for(&mut self, seconds: f32) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = (seconds / STEP).round() as usize;
        for _ in 0..steps {
            self.session
                .step(&mut self.world, &self.input, (1280.0, 720.0), STEP)
                .expect("the step runs");
            self.input.begin_frame(Duration::from_secs_f32(STEP));
        }
    }

    fn id(&self, name: &str) -> EntityId {
        self.world
            .entity_for_source_id(&SceneEntityId::new(name).expect("a valid id"))
            .unwrap_or_else(|| panic!("no {name}"))
    }

    fn x(&self, name: &str) -> f32 {
        self.world
            .get(self.id(name))
            .and_then(|data| data.transform_3d)
            .expect("placed")
            .position[0]
    }

    fn alpha(&self, name: &str) -> f64 {
        self.world.get(self.id(name)).expect("there").components["sindri.ui.text"]["color"][3]
            .as_f64()
            .expect("a number")
    }

    fn zoom(&self) -> f64 {
        self.world.get(self.id("camera")).expect("there").components["sindri.camera"]
            ["vertical_size"]
            .as_f64()
            .expect("a number")
    }

    fn playing(&self) -> String {
        self.world
            .get(self.id("director"))
            .expect("there")
            .components["sindri.sequence"]["playing"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    }
}

#[test]
fn the_intro_plays_then_the_stage_idles_and_replays_on_a_press() {
    let mut stage = Stage::open();
    // Authored at its finished look, so the editor shows the stage; the first
    // step poses the intro's start.
    assert!(stage.x("ship").abs() < 1.0e-6);
    stage.play_for(STEP);
    assert!(stage.x("ship") < -6.0, "the ship starts off stage");
    assert!(
        stage.zoom() < 4.1,
        "the camera, outside the director, starts close: {}",
        stage.zoom()
    );
    assert!(stage.alpha("title") < 1.0e-6, "the title starts hidden");

    stage.play_for(0.6);
    let flying = stage.x("ship");
    assert!(flying > -7.0 && flying < 0.0, "half way in: {flying}");

    stage.play_for(1.0);
    assert!(stage.x("ship").abs() < 1.0e-4, "landed at the centre");
    assert!(
        (stage.alpha("title") - 1.0).abs() < 1.0e-6,
        "the title is up"
    );

    // The intro ends at 2.4 s; the script hears it and moves on.
    stage.play_for(1.2);
    assert_eq!(stage.playing(), "idle");
    let director = stage.id("director");
    assert!(stage.session.sequences().time(director).is_some());
    assert!(
        !stage.session.sequences().is_finished(director),
        "idle loops"
    );

    stage.input.apply(InputEvent::KeyPressed(Key::Space));
    stage.play_for(STEP);
    stage.input.apply(InputEvent::KeyReleased(Key::Space));
    stage.play_for(0.1);
    assert_eq!(stage.playing(), "intro");
    assert!(
        stage.x("ship") < -5.0,
        "replayed from the start: {}",
        stage.x("ship")
    );
}
