//! Sliced images: nine quads that keep their border, in the world and on the
//! screen.

use sindri_core::SpriteSheetDocument;
use sindri_render::{FrameCommand, SpriteInstance, TextureId};
use sindri_scene::{CameraView, SceneExtractor, TextureBindings};

use crate::support::{VIEWPORT, close, scene, world_from};

/// A 24-pixel panel with a six-pixel frame, bound the way a project binds it.
fn panel() -> TextureBindings {
    let mut sheet = SpriteSheetDocument::from_grid(1, 1);
    sheet.size = Some([24, 24]);
    sheet.borders.insert(String::new(), [6, 6, 6, 6]);
    let mut bindings = TextureBindings::new();
    bindings.bind("panel.png", TextureId::new(1));
    bindings.bind_sheet("panel.png", &sheet).expect("it slices");
    bindings
}

fn instances(entities: &str, bindings: &TextureBindings) -> Vec<SpriteInstance> {
    let world = world_from(&scene(entities));
    let frame = SceneExtractor::new()
        .unwrap()
        .extract(&world, VIEWPORT, CameraView::default(), bindings)
        .expect("the scene extracts");
    frame
        .passes()
        .iter()
        .filter_map(|pass| match &pass.command {
            FrameCommand::SpriteBatch { instances, .. } => Some(instances.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// A sliced board four units wide keeps a frame six pixels at sixteen a unit
/// on every side, and shares one draw with itself.
#[test]
fn a_sliced_sprite_keeps_its_frame() {
    let drawn = instances(
        r#",
        { "id": "board", "transform_3d": { "scale": [4.0, 2.0, 1.0] },
          "components": { "sindri.sprite": { "texture": "panel.png", "draw": "sliced" } } }"#,
        &panel(),
    );
    assert_eq!(drawn.len(), 9);
    let corner = drawn[0].model();
    assert!(close(corner.x_axis.x, 0.375) && close(corner.y_axis.y, 0.375));
    let middle = drawn[4].model();
    assert!(close(middle.x_axis.x, 3.25) && close(middle.y_axis.y, 1.25));
}

/// Drawn simply, or with a sheet that gives no border, it is one quad as it
/// always was.
#[test]
fn an_unsliced_sprite_is_one_quad() {
    let simple = r#",
        { "id": "board", "transform_3d": { "scale": [4.0, 2.0, 1.0] },
          "components": { "sindri.sprite": { "texture": "panel.png" } } }"#;
    assert_eq!(instances(simple, &panel()).len(), 1);
    let unbordered = r#",
        { "id": "board", "transform_3d": { "scale": [4.0, 2.0, 1.0] },
          "components": { "sindri.sprite": { "texture": "panel.png", "draw": "sliced" } } }"#;
    assert_eq!(instances(unbordered, &TextureBindings::new()).len(), 1);
}

/// A sliced UI bar keeps its end caps as it empties: at half full its corners
/// are the same size as when full.
#[test]
fn a_sliced_bar_keeps_its_caps_as_it_empties() {
    let bar = |amount: f32| {
        instances(
            &format!(
                r#",
        {{ "id": "bar", "transform_3d": {{ "scale": [1.0, 0.2, 1.0] }},
          "components": {{ "sindri.ui.image": {{ "texture": "panel.png", "draw": "sliced",
            "border_scale": 0.01, "fill": {{ "amount": {amount} }} }} }} }}"#
            ),
            &panel(),
        )
    };
    let full = bar(1.0);
    let half = bar(0.5);
    assert_eq!(full.len(), 9);
    assert_eq!(half.len(), 9);
    assert!(close(full[0].model().x_axis.x, half[0].model().x_axis.x));
    assert!(close(half[0].model().x_axis.x, 0.06));
    assert!(bar(0.0).is_empty(), "an empty bar draws nothing");
}
