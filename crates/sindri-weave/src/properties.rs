//! Every property Weave applies to Sindri UI, written down once.
//!
//! The table is what the bridge applies: a declaration whose property is not
//! here is ignored, as an unknown one always was, so a property handled in
//! code but missing from the table stops working and its tests say so. A test
//! also applies every entry's example and requires it to change the world, so
//! an entry cannot describe something that does nothing.
//!
//! `sindri-capabilities` writes this table to `docs/generated/` as the Weave
//! property reference, the way it writes the Decay host surface.

/// One property, or several that differ only in axis or side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Property {
    /// Every spelling that means this property, the usual one first.
    pub names: &'static [&'static str],
    /// The heading it is listed under.
    pub group: &'static str,
    /// What a value may be.
    pub values: &'static str,
    /// The Sindri UI data it writes.
    pub writes: &'static str,
    /// What it is for, in a sentence or two.
    pub summary: &'static str,
    /// A value that applies cleanly, for the first name.
    pub example: &'static str,
}

const SIZE: &str = "Size and position";
const BOX: &str = "Box";
const FLEX: &str = "Flex layout";
const GRID: &str = "Grid layout";
const TEXT: &str = "Text";
const SHAPE: &str = "Shape";
const MOTION: &str = "Motion";

/// Every property, in the order the reference lists them.
pub const PROPERTIES: &[Property] = &[
    Property {
        names: &["width", "height"],
        group: SIZE,
        values: "a length, `auto` or `fit-content`",
        writes: "the transform's scale; a text element's `sindri.ui.text` `bounds`",
        summary: "The element's size. `auto` and `fit-content` size a layout to its children, \
                  padding and gaps, and a text element to its measured words.",
        example: "120px",
    },
    Property {
        names: &["min-width", "max-width", "min-height", "max-height"],
        group: SIZE,
        values: "a length",
        writes: "the transform's scale, and `sindri.ui.box` `min_size` or `max_size`",
        summary: "Limits on the size, which a layout also honors when it grows or shrinks the \
                  element. When the two conflict the minimum wins, as in CSS.",
        example: "40px",
    },
    Property {
        names: &["x", "y"],
        group: SIZE,
        values: "a length",
        writes: "the transform's position",
        summary: "Where the element is, replacing the position the scene gave it.",
        example: "10px",
    },
    Property {
        names: &["anchor"],
        group: SIZE,
        values: "`center`, `top`, `bottom`, `left`, `right`, `top-left`, `top-right`, \
                 `bottom-left` or `bottom-right`",
        writes: "`anchor` on `sindri.ui.image`, `sindri.ui.text` and `sindri.ui.shape`",
        summary: "Which point of the element its position names.",
        example: "top-left",
    },
    Property {
        names: &[
            "padding",
            "padding-top",
            "padding-right",
            "padding-bottom",
            "padding-left",
        ],
        group: BOX,
        values: "a length of zero or more; `padding` takes one to four, as in CSS",
        writes: "`sindri.ui.box` `padding`",
        summary: "Room inside the element's edge: a layout's children start inside it. A \
                  percentage on any side is of the containing element's content width.",
        example: "8px",
    },
    Property {
        names: &[
            "margin",
            "margin-top",
            "margin-right",
            "margin-bottom",
            "margin-left",
        ],
        group: BOX,
        values: "a length, negative allowed; `margin` takes one to four, as in CSS",
        writes: "`sindri.ui.box` `margin`",
        summary: "Room a layout keeps free around the element. A percentage on any side is of \
                  the containing element's content width.",
        example: "8px",
    },
    Property {
        names: &["display"],
        group: FLEX,
        values: "`flex` or `grid`",
        writes: "adds `sindri.ui.layout` (flex) or `sindri.ui.grid` (grid), removing the other",
        summary: "Makes the element a flex line or a grid that lays out its children.",
        example: "grid",
    },
    Property {
        names: &["flex-direction", "direction"],
        group: FLEX,
        values: "`row` or `column`",
        writes: "`sindri.ui.layout` `direction`",
        summary: "The axis a flex line runs along.",
        example: "column",
    },
    Property {
        names: &["justify-content"],
        group: FLEX,
        values: "`start`, `center`, `end`, `space-between`, `space-around` or `space-evenly` \
                 (`flex-start`/`left` and `flex-end`/`right` mean `start` and `end`)",
        writes: "`sindri.ui.layout` `justify`",
        summary: "How a flex line spreads its children along its direction.",
        example: "center",
    },
    Property {
        names: &["align-items"],
        group: FLEX,
        values: "`start`, `center`, `end` or `stretch` (`flex-start` and `flex-end` too)",
        writes: "`sindri.ui.layout` `align`, and `sindri.ui.grid` `align_items`",
        summary: "Where children sit across a flex line, or across a grid's rows.",
        example: "center",
    },
    Property {
        names: &["flex-wrap"],
        group: FLEX,
        values: "`wrap` or `nowrap`",
        writes: "`sindri.ui.layout` `wrap`",
        summary: "Whether children that do not fit start a new line.",
        example: "wrap",
    },
    Property {
        names: &["flex"],
        group: FLEX,
        values: "`none`, `auto`, `initial`, or `<grow> [<shrink>] [<basis>]`",
        writes: "`flex-grow`, `flex-shrink` and `flex-basis`, which it stands for",
        summary: "Shorthand for the three below, read as CSS reads it: `flex: 1` makes \
                  children share a line equally whatever their sizes.",
        example: "1",
    },
    Property {
        names: &["flex-grow", "flex-shrink"],
        group: FLEX,
        values: "a number, zero or more",
        writes: "`sindri.ui.box` `grow` or `shrink`",
        summary: "The element's share of a line's spare room, or of its shortfall.",
        example: "2",
    },
    Property {
        names: &["flex-basis"],
        group: FLEX,
        values: "a length of zero or more, `auto` or `content`",
        writes: "`sindri.ui.box` `basis`",
        summary: "The element's size along the line before growing or shrinking.",
        example: "50%",
    },
    Property {
        names: &["order"],
        group: FLEX,
        values: "a whole number",
        writes: "`sindri.ui.box` `order`",
        summary: "Where the element comes in its layout, lowest first.",
        example: "2",
    },
    Property {
        names: &["position"],
        group: FLEX,
        values: "`absolute`, or `static` (`relative` reads the same)",
        writes: "`sindri.ui.box` `absolute`",
        summary: "`absolute` takes the element out of its parent's line or grid, which \
                  closes up as if it were not there; `x` and `y` then place it from the \
                  parent's middle, over or under its siblings.",
        example: "absolute",
    },
    Property {
        names: &["align-self"],
        group: FLEX,
        values: "`auto`, `start`, `center`, `end` or `stretch`",
        writes: "`sindri.ui.box` `align_self`",
        summary: "One child's own `align-items`.",
        example: "center",
    },
    Property {
        names: &["grid-template-columns", "grid-template-rows"],
        group: GRID,
        values: "tracks: `1fr`, `auto`, lengths and `repeat(n, …)`, or `none`",
        writes: "`sindri.ui.grid` `columns` or `rows`",
        summary: "A grid's columns or rows.",
        example: "1fr 2fr",
    },
    Property {
        names: &["gap", "row-gap", "column-gap"],
        group: GRID,
        values: "a length of zero or more; `gap` takes a row gap and a column gap",
        writes: "`sindri.ui.grid` `gap`, and `sindri.ui.layout` `spacing`",
        summary: "Space between a grid's tracks. A flex line takes the gap along it.",
        example: "8px",
    },
    Property {
        names: &["justify-items"],
        group: GRID,
        values: "`start`, `center`, `end` or `stretch`",
        writes: "`sindri.ui.grid` `justify_items`",
        summary: "Where grid items sit across their cells.",
        example: "center",
    },
    Property {
        names: &["grid-column", "grid-row"],
        group: GRID,
        values: "`2`, `span 2`, `2 / 4`, `2 / span 3` or `auto`",
        writes: "`sindri.ui.box` `grid_column` or `grid_row`",
        summary: "Where an item starts in a grid, and how many tracks it spans.",
        example: "2 / span 2",
    },
    Property {
        names: &["color"],
        group: TEXT,
        values: "a color",
        writes: "`sindri.ui.text` `color`",
        summary: "The text's color.",
        example: "#ff8800",
    },
    Property {
        names: &["font-size", "line-height", "letter-spacing"],
        group: TEXT,
        values: "a length",
        writes: "`sindri.ui.text` `font_size`, `line_height` or `letter_spacing`",
        summary: "The glyphs' size, the distance between baselines, and extra space after \
                  every glyph.",
        example: "24px",
    },
    Property {
        names: &["font-weight"],
        group: TEXT,
        values: "`normal`, `400`, `bold` or `700`",
        writes: "`sindri.ui.text` `bold`",
        summary: "The font's own bold face, or its regular one.",
        example: "bold",
    },
    Property {
        names: &["font-style"],
        group: TEXT,
        values: "`normal` or `italic`",
        writes: "`sindri.ui.text` `italic`",
        summary: "The font's own italic face, or its upright one.",
        example: "italic",
    },
    Property {
        names: &["text-align"],
        group: TEXT,
        values: "`start` (or `left`), `center`, `end` (or `right`) or `justify`",
        writes: "`sindri.ui.text` `line_align`",
        summary: "Where each line sits across the text's box.",
        example: "center",
    },
    Property {
        names: &["text-wrap"],
        group: TEXT,
        values: "`none` (or `nowrap`), `word` (or `wrap`) or `glyph`",
        writes: "`sindri.ui.text` `wrap`",
        summary: "Whether lines break, and where: between words, or between any two glyphs.",
        example: "word",
    },
    Property {
        names: &["text-transform"],
        group: TEXT,
        values: "`none`, `uppercase` or `lowercase`",
        writes: "`sindri.ui.text` `case`",
        summary: "The case the text is shown in, leaving the text itself as written.",
        example: "uppercase",
    },
    Property {
        names: &["background"],
        group: SHAPE,
        values: "a color",
        writes: "`sindri.ui.shape` `fill`",
        summary: "The element's fill.",
        example: "#1f2937",
    },
    Property {
        names: &["border-color"],
        group: SHAPE,
        values: "a color",
        writes: "`sindri.ui.shape` `stroke`",
        summary: "The color of the element's outline.",
        example: "#ffffff",
    },
    Property {
        names: &["border-width", "border-radius"],
        group: SHAPE,
        values: "a length in `px`, `vw` or `vh`, a percentage, or a plain number as a fraction \
                 of the element's shorter side",
        writes: "`sindri.ui.shape` `stroke_width` or `corner_radius`",
        summary: "The outline's thickness, and how round the corners are.",
        example: "4px",
    },
    Property {
        names: &["box-shadow"],
        group: SHAPE,
        values: "`<x> <y> [<blur> [<spread>]] [<color>]`, or `none`; one shadow, no `inset`",
        writes: "`sindri.ui.shape` `shadow`",
        summary: "A shadow behind the element's shape.",
        example: "0 4px 8px #00000080",
    },
    Property {
        names: &["transition"],
        group: MOTION,
        values: "comma-separated `<property> <duration> [<delay>] [<easing>]`; a property may \
                 be `all`, a duration is `ms` or `s`, and an easing is `linear`, `ease`, \
                 `ease-in`, `ease-out` or `ease-in-out`",
        writes: "nothing itself: the properties it names ease to each new value",
        summary: "Eases a property to a new value instead of jumping, for example on `:hover`. \
                  Colors, lengths and plain numbers ease; anything else changes at once.",
        example: "background 150ms ease-out",
    },
];

/// The entry `name` belongs to, if Weave applies it at all.
#[must_use]
pub fn property(name: &str) -> Option<&'static Property> {
    PROPERTIES
        .iter()
        .find(|property| property.names.contains(&name))
}

#[cfg(test)]
mod tests {
    use sindri_core::{SceneDocument, World};
    use weave::{INHERITED, Viewport, parse};

    use super::{PROPERTIES, property};
    use crate::PresentationWorld;

    const VIEWPORT: Viewport = Viewport {
        width: 1280.0,
        height: 720.0,
    };

    /// A text, a shape, a box and both layouts on one element inside a
    /// frame, so every property has the component it writes to.
    fn element() -> World {
        let document = SceneDocument::from_json(
            r#"{
                "format_version": 10,
                "metadata": { "name": "properties" },
                "entities": [
                    { "id": "frame", "transform_3d": { "scale": [1.6, 1.0, 1.0] },
                      "components": { "sindri.ui.grid": {} } },
                    { "id": "el", "parent": "frame",
                      "transform_3d": { "scale": [0.5, 0.25, 1.0] },
                      "components": {
                        "sindri.ui.text": { "text": "Hi" },
                        "sindri.ui.shape": {},
                        "sindri.ui.box": {},
                        "sindri.ui.layout": {},
                        "sindri.ui.grid": {}
                      } }
                ]
            }"#,
        )
        .expect("scene parses");
        World::from_scene(&document).expect("scene loads").world
    }

    fn styled(sheet: &str) -> World {
        let sheet = parse(sheet).expect("Weave parses");
        PresentationWorld::resolve(&element(), &sheet, VIEWPORT)
            .unwrap_or_else(|error| panic!("{error}"))
            .world()
            .clone()
    }

    fn differs(world: &World) -> bool {
        let before = element();
        world
            .entities()
            .zip(before.entities())
            .any(|((_, after), (_, before))| after != before)
    }

    #[test]
    fn every_entry_changes_the_world_it_is_applied_to() {
        for entry in PROPERTIES {
            for name in entry.names {
                let world = styled(&format!("#el {{ {name}: {}; }}", entry.example));
                // A transition changes nothing until a value it names does;
                // its own tests in `transition.rs` cover that.
                if *name == "transition" {
                    continue;
                }
                assert!(
                    differs(&world),
                    "`{name}: {}` applied without changing anything",
                    entry.example
                );
            }
        }
    }

    #[test]
    fn a_property_outside_the_table_is_ignored() {
        assert!(property("font-family").is_none());
        assert!(!differs(&styled("#el { font-family: serif; }")));
    }

    #[test]
    fn names_are_unique_and_every_inherited_property_is_listed() {
        let mut seen = std::collections::BTreeSet::new();
        for name in PROPERTIES.iter().flat_map(|entry| entry.names) {
            assert!(seen.insert(*name), "`{name}` is listed twice");
        }
        for name in INHERITED {
            assert!(
                property(name).is_some(),
                "`{name}` inherits but is not applied"
            );
        }
    }

    #[test]
    fn font_style_asks_for_the_italic_face() {
        let world = styled("#el { font-style: italic; }");
        let italic = world
            .entities()
            .find_map(|(_, data)| data.components.get("sindri.ui.text"))
            .and_then(|text| text.get("italic"))
            .and_then(serde_json::Value::as_bool);
        assert_eq!(italic, Some(true));
        let refused = PresentationWorld::resolve(
            &element(),
            &parse("#el { font-style: oblique; }").expect("Weave parses"),
            VIEWPORT,
        );
        assert!(refused.is_err());
    }
}
