//! What the frame draws: the hull plates, fittings and wreckage out of the
//! diver's view are culled, so a ship of a few hundred pieces draws only what
//! is in front of the eye.

use drift::Run;
use sindri_assets::{AssetBytes, AssetDecoder, ModelAssetDecoder};
use sindri_core::AssetId;
use sindri_render::{FrameCommand, Viewport};
use sindri_scene::{CameraView, TextureBindings, referenced_models};

/// Loads every model the scene names and binds it for drawing, as a host does
/// before its first frame; a headless run only loads what collides.
fn bind_models(game: &mut Run) {
    for name in referenced_models(&game.run.world) {
        let id = AssetId::new(name.clone()).expect("a model path");
        let bytes =
            std::fs::read(drift::project().join("assets").join(&name)).expect("the model reads");
        let asset = ModelAssetDecoder
            .decode(AssetBytes::new(id.clone(), bytes))
            .expect("the model decodes");
        let prepared = sindri::model::prepare(asset).expect("the model prepares");
        game.run.scene.bind_model(id, prepared.resource);
    }
}

/// How many cubes and models one frame draws, and how many the scene has.
fn drawn(game: &mut Run) -> (usize, usize) {
    bind_models(game);
    let authored = game
        .run
        .world
        .entities()
        .filter(|(_, data)| {
            data.components.contains_key("sindri.mesh")
                || data.components.contains_key("sindri.model")
        })
        .count();
    let passes = game
        .run
        .draw_with(|scene, world, _| {
            let frame = scene
                .extract(
                    world,
                    Viewport::new(1280, 720),
                    CameraView::default(),
                    &TextureBindings::new(),
                )
                .map_err(|error| error.to_string())?;
            let drawn = frame
                .passes()
                .iter()
                .filter(|pass| {
                    matches!(
                        pass.command,
                        FrameCommand::TexturedCube { .. } | FrameCommand::Model { .. }
                    )
                })
                .count();
            Ok((drawn, sindri_scene::UiTextSizes::new()))
        })
        .expect("the frame extracts");
    (passes, authored)
}

#[test]
fn facing_aft_in_the_cabin_draws_neither_the_hold_nor_the_wreckage() {
    let mut game = Run::open().expect("DRIFT opens");
    game.wait(0.3);
    let (forward, authored) = drawn(&mut game);
    assert!(
        authored > 150,
        "the ship and the wreckage: {authored} pieces"
    );
    // Culling is by the view's frustum, not by what walls hide: looking
    // forward down the cabin, the hold and the wreckage are straight ahead
    // behind the bulkheads, and are drawn.
    assert!(forward * 10 > authored * 8, "{forward} of {authored}");
    // Turned to face aft, everything forward of the eye drops out.
    let eye = game.run.entity("eye").expect("the eye");
    game.run
        .world
        .get_mut(eye)
        .and_then(|data| data.transform_3d.as_mut())
        .expect("the eye has a transform")
        .rotation = [
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
    ];
    let (aft, _) = drawn(&mut game);
    assert!(
        aft * 4 < authored,
        "facing aft draws the cabin's end: {aft} of {authored}"
    );
}

#[test]
fn looking_out_of_the_hatch_draws_the_wreckage_ahead_and_not_the_ship_behind() {
    let mut game = Run::open_at([5.5, -0.6, 0.0]).expect("DRIFT opens");
    game.wait(0.3);
    let (out, authored) = drawn(&mut game);
    assert!(
        out * 2 < authored,
        "the cabin and airlock behind are not drawn: {out} of {authored}"
    );
}
