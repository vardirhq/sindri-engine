//! The HUD's frame and the signboard by the gap are one 24-pixel panel,
//! sliced: each keeps a six-pixel wooden border at its own size.

use lamplighter::Run;
use sindri_core::SpriteSheetDocument;
use sindri_render::{FrameCommand, TextureId, Viewport};
use sindri_scene::{CameraView, TextureBindings};

#[test]
fn the_hud_frame_and_the_signboard_are_sliced() {
    let sheet = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/textures/panel.sheet"
    ))
    .expect("the panel has a sheet");
    let sheet = SpriteSheetDocument::from_json(&sheet).expect("the sheet parses");
    let panel = TextureId::new(77);
    let mut bindings = TextureBindings::new();
    bindings.bind("textures/panel.png", panel);
    bindings
        .bind_sheet("textures/panel.png", &sheet)
        .expect("the border fits");

    let mut run = Run::open().expect("the project opens");
    assert!(run.step(1.0 / 60.0).is_empty());
    let frame = run
        .scene
        .extract(
            &run.world,
            Viewport::new(960, 540),
            CameraView::default(),
            &bindings,
        )
        .expect("the level extracts");
    let pieces: usize = frame
        .passes()
        .iter()
        .filter_map(|pass| match &pass.command {
            FrameCommand::SpriteBatch {
                texture, instances, ..
            } if *texture == panel => Some(instances.len()),
            _ => None,
        })
        .sum();
    assert_eq!(
        pieces, 18,
        "nine pieces for the HUD frame and nine for the sign"
    );
}
