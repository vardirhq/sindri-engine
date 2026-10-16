//! Which batches a sprite mask reaches, and what a missing one means.

use sindri_render::{FrameCommand, SpriteMask};
use sindri_scene::{CameraView, SceneExtractor, TextureBindings};

use crate::support::{VIEWPORT, close, scene, world_from};

fn masks(entities: &str) -> Vec<Option<SpriteMask>> {
    let frame = SceneExtractor::new()
        .unwrap()
        .extract(
            &world_from(&scene(entities)),
            VIEWPORT,
            CameraView::default(),
            &TextureBindings::new(),
        )
        .expect("the scene extracts");
    frame
        .passes()
        .iter()
        .filter_map(|pass| match &pass.command {
            FrameCommand::SpriteBatch { mask, .. } => Some(*mask),
            _ => None,
        })
        .collect()
}

/// A sprite meets the mask its `mask_name` names, placed where the mask's
/// entity stands, and the way round it asked for.
#[test]
fn a_sprite_meets_the_mask_it_names() {
    let drawn = masks(
        r#",
        { "id": "hole", "transform_3d": { "position": [2.0, 1.0, 0.0], "scale": [4.0, 4.0, 1.0] },
          "components": { "sindri.sprite_mask": { "texture": "hole.png", "name": "hole" } } },
        { "id": "night", "components": { "sindri.sprite": {
            "texture": "night.png", "mask": "outside", "mask_name": "hole" } } }"#,
    );
    assert_eq!(drawn.len(), 1, "the mask itself draws nothing");
    let mask = drawn[0].expect("the night is masked");
    assert!(mask.outside);
    let centre = mask
        .from_world
        .transform_point3(glam::Vec3::new(2.0, 1.0, 0.0));
    assert!(close(centre.x, 0.0) && close(centre.y, 0.0), "{centre:?}");
}

/// Inside a mask that is not there is nowhere; outside one is everywhere.
#[test]
fn a_missing_mask_hides_inside_and_spares_outside() {
    let inside = masks(
        r#",
        { "id": "gem", "components": { "sindri.sprite": {
            "texture": "gem.png", "mask": "inside", "mask_name": "nowhere" } } }"#,
    );
    assert!(inside.is_empty());
    let outside = masks(
        r#",
        { "id": "night", "components": { "sindri.sprite": {
            "texture": "night.png", "mask": "outside", "mask_name": "nowhere" } } }"#,
    );
    assert_eq!(outside, vec![None]);
}
