//! Text properties, written to the element's `sindri.ui.text`.
//!
//! Every one of them inherits, as in CSS (`weave::INHERITED`), so a panel's
//! `color` or `font-size` reaches the words inside it.

use sindri_core::{EntityId, World};
use weave::Viewport;

use crate::{ApplyError, color, invalid, length, set_component_field};

const TEXT: &str = "sindri.ui.text";

/// Applies `property` if it is a text property, answering whether it was.
pub(crate) fn apply(
    world: &mut World,
    entity: EntityId,
    id: &str,
    property: &str,
    value: &str,
    viewport: Viewport,
) -> Result<bool, ApplyError> {
    let refuse = || invalid(id, property, value);
    let (field, stored) = match property {
        "color" => {
            let resolved = color(value).ok_or_else(refuse)?;
            ("color", serde_json::json!(resolved))
        }
        "font-size" | "line-height" | "letter-spacing" => {
            let resolved = length(value, viewport).ok_or_else(refuse)?;
            let field = match property {
                "font-size" => "font_size",
                "line-height" => "line_height",
                _ => "letter_spacing",
            };
            (field, resolved.into())
        }
        "text-align" => {
            let stored = match value.trim() {
                "start" | "left" => "left",
                "center" => "center",
                "end" | "right" => "right",
                "justify" => "justify",
                _ => return Err(refuse()),
            };
            ("line_align", stored.into())
        }
        "text-wrap" => {
            let stored = match value.trim() {
                "none" | "nowrap" => "none",
                "wrap" | "word" => "word",
                "glyph" => "glyph",
                _ => return Err(refuse()),
            };
            ("wrap", stored.into())
        }
        "text-transform" => {
            let stored = match value.trim() {
                "none" => "as_written",
                "uppercase" => "upper",
                "lowercase" => "lower",
                _ => return Err(refuse()),
            };
            ("case", stored.into())
        }
        "font-weight" => {
            let bold = match value.trim() {
                "normal" | "400" => false,
                "bold" | "700" => true,
                _ => return Err(refuse()),
            };
            ("bold", bold.into())
        }
        // The face's own italic, asked of the font as `font-weight` asks for
        // its bold. `oblique` is not offered: there is no slanted fallback.
        "font-style" => {
            let italic = match value.trim() {
                "normal" => false,
                "italic" => true,
                _ => return Err(refuse()),
            };
            ("italic", italic.into())
        }
        _ => return Ok(false),
    };
    set_component_field(world, entity, TEXT, field, stored);
    Ok(true)
}
