//! `sindri.text`: words in the world, drawn with the sprites of their layer.

use sindri_render::{FrameCommand, RenderStage};
use sindri_scene::{CameraView, SceneExtractor, TextureBindings};

use crate::support::{VIEWPORT, close, scene, world_from};

/// The text lands where its entity is, ancestors and all, in the world
/// camera's stage and over the sprite that shares its layer.
#[test]
fn world_text_is_placed_at_its_entity_over_its_layer() {
    let frame = SceneExtractor::new()
        .unwrap()
        .extract(
            &world_from(&scene(
                r#",
        { "id": "enemy", "transform_3d": { "position": [3.0, 1.0, 0.0] },
          "components": { "sindri.sprite": { "texture": "enemy.png", "layer": 2 } } },
        { "id": "hit", "parent": "enemy", "transform_3d": { "position": [0.0, 0.5, 0.0] },
          "components": { "sindri.text": {
            "text": "[b]12[/b]", "font": "fonts/a.ttf", "font_size": 0.4, "layer": 2,
            "rich": true } } }"#,
            )),
            VIEWPORT,
            CameraView::default(),
            &TextureBindings::new(),
        )
        .expect("the scene extracts");
    let passes = frame.passes();
    let sprite = passes
        .iter()
        .position(|pass| matches!(pass.command, FrameCommand::SpriteBatch { .. }))
        .expect("the sprite draws");
    let text = passes
        .iter()
        .position(|pass| matches!(pass.command, FrameCommand::Text { .. }))
        .expect("the text draws");
    assert!(text > sprite, "the number is over the enemy");
    let pass = &passes[text];
    assert_eq!(pass.stage, RenderStage::Transparent2d);
    let FrameCommand::Text { instances } = &pass.command else {
        unreachable!()
    };
    let [x, y] = instances[0].position();
    assert!(close(x, 3.0) && close(y, 1.5), "{x}, {y}");
    assert!(close(instances[0].font_size(), 0.4));
}

/// A world label may be far taller than the overlay's two units: a size is in
/// world units here, and a sign over a building is several of them.
#[test]
fn world_text_sizes_are_world_units() {
    SceneExtractor::new()
        .unwrap()
        .extract(
            &world_from(&scene(
                r#",
        { "id": "sign", "components": { "sindri.text": {
            "text": "INN", "font": "fonts/a.ttf", "font_size": 4.0, "line_height": 4.5 } } }"#,
            )),
            VIEWPORT,
            CameraView::default(),
            &TextureBindings::new(),
        )
        .expect("a big world label extracts");
}
