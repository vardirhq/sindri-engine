//! Sprite Masks, opened off disk and played: the keyhole wanders, the night
//! is masked outside it and the gem inside it.

use std::path::PathBuf;

use sindri_causeway::{Session, extractor};
use sindri_core::{LoadedScenes, SceneDocument, World};
use sindri_decay::{ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_render::{FrameCommand, Viewport};
use sindri_scene::{CameraView, TextureBindings};

const STEP: f32 = 1.0 / 60.0;
const SCENE: &str = "sprite-masks.scene";
const SCRIPT: &str = "scripts/keyhole.decay";

fn read(id: &str) -> String {
    let root = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../examples/sprite-masks/assets"
    ));
    std::fs::read_to_string(root.join(id)).unwrap_or_else(|error| panic!("{id}: {error}"))
}

/// Each masked batch's mask: whether it is outside, and where the mask is.
fn masks(world: &World) -> Vec<(bool, f32)> {
    let registry = extractor().expect("the schemas register");
    let frame = registry
        .extract(
            world,
            Viewport::new(960, 540),
            CameraView::default(),
            &TextureBindings::new(),
        )
        .expect("the scene extracts");
    frame
        .passes()
        .iter()
        .filter_map(|pass| match &pass.command {
            FrameCommand::SpriteBatch {
                mask: Some(mask), ..
            } => Some((mask.outside, mask.from_world.inverse().w_axis.x)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_keyhole_wanders_and_masks_both_ways() {
    let document = SceneDocument::from_json(&read(SCENE)).expect("the scene parses");
    let mut world = World::default();
    LoadedScenes::new()
        .enter_keeping_identities(&mut world, SCENE, &document)
        .expect("the scene loads");
    let mut sources = ScriptSources::new();
    sources.insert(SCRIPT, read(SCRIPT));
    let registry = extractor().expect("the schemas register");
    let failures = Scripts::new().compile(&world, registry.components(), &sources);
    assert!(failures.is_empty(), "{failures:?}");

    let before = masks(&world);
    assert_eq!(
        before.len(),
        2,
        "the night and the gem are masked: {before:?}"
    );
    assert!(before.iter().any(|(outside, _)| *outside), "the night");
    assert!(before.iter().any(|(outside, _)| !*outside), "the gem");

    let mut session = Session::with_sources(registry.components().clone(), sources)
        .with_scenes(vec![(SCENE.to_owned(), document)], LoadedScenes::new());
    let input = InputState::default();
    for _ in 0..60 {
        session
            .step(&mut world, &input, (960.0, 540.0), STEP)
            .expect("the step runs")
            .log();
    }
    let after = masks(&world);
    assert!(
        (after[0].1 - before[0].1).abs() > 0.5,
        "the keyhole moved: {before:?} then {after:?}"
    );
}
