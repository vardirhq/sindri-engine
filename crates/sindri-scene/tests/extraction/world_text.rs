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

/// A label stays level under a turned entity unless it asks to follow the
/// turn, and then it takes the whole world turn, its parent's included.
#[test]
fn world_text_turns_with_its_entity_only_when_asked() {
    let quarter = std::f32::consts::FRAC_1_SQRT_2;
    let frame = SceneExtractor::new()
        .unwrap()
        .extract(
            &world_from(&scene(&format!(
                r#",
        {{ "id": "crate", "transform_3d": {{ "position": [1.0, 0.0, 0.0],
            "rotation": [0.0, 0.0, {quarter}, {quarter}] }} }},
        {{ "id": "level", "parent": "crate", "components": {{ "sindri.text": {{
            "text": "UP", "font": "fonts/a.ttf", "font_size": 0.4 }} }} }},
        {{ "id": "painted", "parent": "crate", "components": {{ "sindri.text": {{
            "text": "FRAGILE", "font": "fonts/a.ttf", "font_size": 0.4,
            "follow_rotation": true }} }} }}"#
            ))),
            VIEWPORT,
            CameraView::default(),
            &TextureBindings::new(),
        )
        .expect("the scene extracts");
    let turns: Vec<(String, f32)> = frame
        .passes()
        .iter()
        .filter_map(|pass| match &pass.command {
            FrameCommand::Text { instances } => Some(instances),
            _ => None,
        })
        .flatten()
        .map(|instance| (instance.text().to_owned(), instance.rotation()))
        .collect();
    let turn = |words: &str| {
        turns
            .iter()
            .find(|(text, _)| text == words)
            .map(|(_, turn)| *turn)
            .expect("drawn")
    };
    assert!(close(turn("UP"), 0.0), "{turns:?}");
    assert!(
        close(turn("FRAGILE"), std::f32::consts::FRAC_PI_2),
        "{turns:?}"
    );
}

/// Seen through a perspective camera, a label stands at its entity's full
/// position in the world, depth and all, facing the camera unless it follows
/// its entity's turn: a readout on a wrist, a display on a wall.
#[test]
fn world_text_stands_in_3d_under_a_perspective_camera() {
    let camera = r#"{ "id": "eye", "transform_3d": { "position": [0, 0, 0] },
        "components": { "sindri.camera": {
          "projection": "perspective", "vertical_fov_degrees": 60.0, "near": 0.05, "far": 100.0 } } }"#;
    let label = r#", { "id": "label", "transform_3d": { "position": [0.5, 0.2, -4.0] },
        "components": { "sindri.text": { "text": "O2 87%", "font": "fonts/a.ttf", "font_size": 0.1 } } }"#;
    let world = world_from(&crate::support::document(&format!("{camera}{label}")));
    let frame = SceneExtractor::new()
        .unwrap()
        .extract(
            &world,
            VIEWPORT,
            CameraView::default(),
            &TextureBindings::new(),
        )
        .expect("the scene extracts");
    let pass = frame
        .passes()
        .iter()
        .find(|pass| matches!(pass.command, FrameCommand::Text { .. }))
        .expect("the label draws");
    // The label's own origin, through its pass's camera, lands where the
    // entity does through the world camera: depth is kept, not flattened.
    let drawn = pass.camera.view_projection * glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
    let ndc = drawn.truncate() / drawn.w;
    assert!(
        drawn.w > 3.9 && drawn.w < 4.1,
        "four units in front of the camera: {drawn}"
    );
    assert!(
        ndc.x > 0.0 && ndc.y > 0.0,
        "up and to the right of the middle: {ndc}"
    );
}
