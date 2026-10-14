//! The 2D drawables whose blanks are long enough to be their own file.

use sindri_core::ComponentSchemaRegistry;

use crate::components::{ShapeComponent, UiShapeBlend, UiShapeComponent, UiShapeKind};

use super::super::SceneExtractError;

/// The two shape components, whose blanks are long enough to be their own
/// function.
///
/// Long because a shape is described entirely by numbers — there is no texture
/// standing in for the look — and because a blank that drew nothing would read
/// as a broken add rather than an empty one. Both default to a mint outline.
pub(super) fn register_shapes(
    components: &mut ComponentSchemaRegistry,
) -> Result<(), SceneExtractError> {
    // The same blank as the overlay shape, minus the anchor: a mint outline,
    // because a shape whose default drew nothing would read as a broken add.
    components.register_with_default::<ShapeComponent>(
        "Shape",
        serde_json::json!({
            "kind": UiShapeKind::default().as_str(),
            "count": 6.0,
            "fill": [0.0, 0.0, 0.0, 0.0],
            "stroke": [0.49, 1.0, 0.77, 1.0],
            "stroke_width": 0.06,
            "corner_radius": 0.0,
            "dashes": 0.0,
            "dash_duty": 0.5,
            "sweep_start": 0.0,
            "sweep_turns": 1.0,
            "blend": UiShapeBlend::default().as_str(),
            "layer": 0
        }),
    )?;
    components.register_with_default::<UiShapeComponent>(
        "UI Shape",
        serde_json::json!({
            "kind": UiShapeKind::default().as_str(),
            "count": 6.0,
            "fill": [0.0, 0.0, 0.0, 0.0],
            "stroke": [0.49, 1.0, 0.77, 1.0],
            "stroke_width": 0.04,
            "corner_radius": 0.0,
            "dashes": 0.0,
            "dash_duty": 0.5,
            "sweep_start": 0.0,
            "sweep_turns": 1.0,
            "blend": UiShapeBlend::default().as_str(),
            "anchor": "center",
            "layer": 0,
            "shadow": {
                "color": [0.0, 0.0, 0.0, 0.0],
                "offset": [0.0, 0.0],
                "blur": 0.0,
                "spread": 0.0
            }
        }),
    )?;
    Ok(())
}
