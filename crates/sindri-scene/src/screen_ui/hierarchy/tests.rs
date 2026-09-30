//! Where elements end up once their parents have had their say, checked.

use super::{UiHierarchy, UiPlaced};
use crate::UiAnchor;
use crate::extract::SceneExtractor;
use glam::{Quat, Vec2};
use sindri_core::{SceneDocument, World};

/// A world from a scene fragment, with the built-in schemas behind it.
fn world(entities: &str) -> (World, SceneExtractor) {
    let document = format!(
        r#"{{ "format_version": 10, "metadata": {{ "name": "t" }},
             "entities": [{entities}] }}"#
    );
    let extractor = SceneExtractor::new().expect("built-in schemas register");
    let parsed = SceneDocument::from_json(&document).expect("the fragment parses");
    let world = World::from_scene(&parsed)
        .expect("the fragment loads")
        .world;
    (world, extractor)
}

fn placement(world: &World, extractor: &SceneExtractor, name: &str) -> UiPlaced {
    let hierarchy = UiHierarchy::of(world, extractor.components()).expect("the hierarchy resolves");
    let entity = world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some(name))
        .map_or_else(|| panic!("{name} is in the world"), |(entity, _)| entity);
    hierarchy
        .placement(entity)
        .unwrap_or_else(|| panic!("{name} is a UI element"))
}

const IMAGE: &str = r#""sindri.ui.image": { "texture": "sindri:white", "anchor": "center" }"#;

/// A child's offset is measured from where its parent ended up, not from the
/// screen.
///
/// The bug this is here for put six upgrade cards' labels on top of each
/// other however far apart the cards were, because each label was placed
/// against the viewport as though it had no parent at all.
#[test]
fn a_child_is_placed_from_its_parent_rather_than_from_the_screen() {
    let (world, extractor) = world(&format!(
        r#"{{ "id": "panel", "name": "panel",
              "transform_3d": {{ "position": [0.3, -0.2, 0.0] }},
              "components": {{ {IMAGE} }} }},
           {{ "id": "label", "name": "label", "parent": "panel",
              "transform_3d": {{ "position": [0.05, 0.1, 0.0] }},
              "components": {{ {IMAGE} }} }}"#
    ));
    let label = placement(&world, &extractor, "label");
    assert!(
        (label.offset - Vec2::new(0.35, -0.1)).length() < 1.0e-6,
        "{label:?}"
    );
}

/// A parent's turn carries its children round with it rather than sliding
/// them sideways.
#[test]
fn a_turned_parent_turns_where_its_children_sit() {
    // A quarter turn about Z sends +X to +Y.
    let quarter = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let [x, y, z, w] = quarter.to_array();
    let (world, extractor) = world(&format!(
        r#"{{ "id": "panel", "name": "panel",
              "transform_3d": {{ "rotation": [{x}, {y}, {z}, {w}] }},
              "components": {{ {IMAGE} }} }},
           {{ "id": "label", "name": "label", "parent": "panel",
              "transform_3d": {{ "position": [0.2, 0.0, 0.0] }},
              "components": {{ {IMAGE} }} }}"#
    ));
    let label = placement(&world, &extractor, "label");
    assert!(
        (label.offset - Vec2::new(0.0, 0.2)).length() < 1.0e-5,
        "{label:?}"
    );
}

/// The anchor comes from the outermost element that declares one, so a label
/// stays on its card when the window changes shape instead of running off to
/// its own corner of the screen.
#[test]
fn a_child_keeps_its_parents_anchor() {
    let (world, extractor) = world(
        r#"{ "id": "panel", "name": "panel",
             "components": { "sindri.ui.image": { "texture": "sindri:white",
                 "anchor": "bottom_right" } } },
           { "id": "label", "name": "label", "parent": "panel",
             "components": { "sindri.ui.image": { "texture": "sindri:white",
                 "anchor": "top_left" } } }"#,
    );
    assert_eq!(
        placement(&world, &extractor, "label").anchor,
        UiAnchor::BottomRight
    );
    assert_eq!(
        placement(&world, &extractor, "panel").anchor,
        UiAnchor::BottomRight
    );
}

/// A group with no UI component of its own still moves what is under it,
/// which is what makes "put the whole menu somewhere else" one edit.
#[test]
fn an_ancestor_that_draws_nothing_still_moves_its_children() {
    let (world, extractor) = world(&format!(
        r#"{{ "id": "menu", "name": "menu",
              "transform_3d": {{ "position": [0.0, 0.5, 0.0] }} }},
           {{ "id": "label", "name": "label", "parent": "menu",
              "transform_3d": {{ "position": [0.0, -0.1, 0.0] }},
              "components": {{ {IMAGE} }} }}"#
    ));
    let label = placement(&world, &extractor, "label");
    assert!((label.offset.y - 0.4).abs() < 1.0e-6, "{label:?}");
}

/// A layout spaces its children, and that reaches what is drawn.
///
/// It used to reach only what was clickable, so an element could be
/// clickable somewhere it was never drawn.
#[test]
fn a_layout_spaces_what_is_drawn_and_not_only_what_is_clicked() {
    let (world, extractor) = world(&format!(
        r#"{{ "id": "row", "name": "row",
              "components": {{ "sindri.ui.layout": {{ "direction": "column",
                  "spacing": 0.4 }} }} }},
           {{ "id": "first", "name": "first", "parent": "row",
              "components": {{ {IMAGE} }} }},
           {{ "id": "second", "name": "second", "parent": "row",
              "components": {{ {IMAGE} }} }}"#
    ));
    let first = placement(&world, &extractor, "first").offset;
    let second = placement(&world, &extractor, "second").offset;
    assert!(
        (first.y - second.y).abs() > 0.39,
        "a column of two at 0.4 spacing: {first:?} then {second:?}"
    );
}

/// Main- and cross-axis placement use the final authored box sizes.
#[test]
fn layout_alignment_uses_parent_and_child_boxes() {
    let (world, extractor) = world(&format!(
        r#"{{ "id": "row", "name": "row",
              "transform_3d": {{ "scale": [4.0, 2.0, 1.0] }},
              "components": {{ "sindri.ui.layout": {{ "direction": "row",
                  "spacing": 0.5, "justify": "start", "align": "end" }} }} }},
           {{ "id": "first", "name": "first", "parent": "row",
              "transform_3d": {{ "scale": [1.0, 0.5, 1.0] }},
              "components": {{ {IMAGE} }} }}"#
    ));
    let first = placement(&world, &extractor, "first").offset;
    assert!(
        (first - Vec2::new(-1.5, -0.75)).length() < 1.0e-6,
        "{first:?}"
    );
}

/// Edge spacing uses box extents, not centre distance.
#[test]
fn layout_spacing_keeps_tall_children_apart() {
    let (world, extractor) = world(&format!(
        r#"{{ "id": "column", "name": "column",
              "transform_3d": {{ "scale": [2.0, 3.0, 1.0] }},
              "components": {{ "sindri.ui.layout": {{ "direction": "column",
                  "spacing": 0.2, "justify": "center", "align": "center" }} }} }},
           {{ "id": "first", "name": "first", "parent": "column",
              "transform_3d": {{ "scale": [1.0, 0.5, 1.0] }},
              "components": {{ {IMAGE} }} }},
           {{ "id": "second", "name": "second", "parent": "column",
              "transform_3d": {{ "scale": [1.0, 1.0, 1.0] }},
              "components": {{ {IMAGE} }} }}"#
    ));
    let first = placement(&world, &extractor, "first").offset;
    let second = placement(&world, &extractor, "second").offset;
    let first_bottom = first.y - 0.25;
    let second_top = second.y + 0.5;
    assert!((first_bottom - second_top - 0.2).abs() < 1.0e-6);
}

/// A button with no art of its own is still an element, and is still laid
/// out.
///
/// Left out of the hierarchy it does not fall back to "no anchor" but to
/// "no placement", so a row of bare hit areas silently stops being spaced.
/// A test caught exactly that.
#[test]
fn a_button_with_no_art_is_still_placed() {
    let (world, extractor) = world(
        r#"{ "id": "row", "name": "row",
             "components": { "sindri.ui.layout": { "direction": "column",
                 "spacing": 0.5 } } },
           { "id": "first", "name": "first", "parent": "row",
             "components": { "sindri.ui.button": { "label": "one" } } },
           { "id": "second", "name": "second", "parent": "row",
             "components": { "sindri.ui.button": { "label": "two" } } }"#,
    );
    let first = placement(&world, &extractor, "first").offset;
    let second = placement(&world, &extractor, "second").offset;
    assert!(
        (first.y - second.y).abs() > 0.49,
        "{first:?} then {second:?}"
    );
}

/// A shape-only card is an element in its own right, not merely a parent
/// whose children happen to draw.
///
/// Omitting shapes from the element set made a proof card fall back to the
/// screen centre while its text children correctly followed the layout.
#[test]
fn a_shape_only_element_is_still_placed() {
    let (world, extractor) = world(
        r#"{ "id": "column", "name": "column",
             "transform_3d": { "scale": [2.0, 2.0, 1.0] },
             "components": { "sindri.ui.layout": { "direction": "column",
                 "justify": "start", "align": "center" } } },
           { "id": "card", "name": "card", "parent": "column",
             "transform_3d": { "scale": [1.0, 0.5, 1.0] },
             "components": { "sindri.ui.shape": { "kind": "rect",
                 "fill": [1.0, 1.0, 1.0, 1.0], "anchor": "center" } } }"#,
    );
    let card = placement(&world, &extractor, "card");
    assert!((card.offset - Vec2::new(0.0, 0.75)).length() < 1.0e-6);
}

/// An element with no parents is placed exactly where it always was.
#[test]
fn an_element_with_no_parents_is_unchanged() {
    let (world, extractor) = world(
        r#"{ "id": "hud", "name": "hud",
             "transform_3d": { "position": [0.1, -0.3, 0.0] },
             "components": { "sindri.ui.image": { "texture": "sindri:white",
                 "anchor": "top_left" } } }"#,
    );
    let hud = placement(&world, &extractor, "hud");
    assert!(
        (hud.offset - Vec2::new(0.1, -0.3)).length() < 1.0e-6,
        "{hud:?}"
    );
    assert_eq!(hud.anchor, UiAnchor::TopLeft);
    assert!(hud.rotation.abs_diff_eq(Quat::IDENTITY, 1.0e-6));
}

/// A layout's children start inside its padding and keep their margins,
/// as they would in a browser.
#[test]
fn a_layout_places_children_inside_its_padding_and_around_their_margins() {
    let (world, extractor) = world(&format!(
        r#"{{ "id": "panel", "name": "panel",
              "transform_3d": {{ "scale": [4.0, 2.0, 1.0] }},
              "components": {{ {IMAGE},
                  "sindri.ui.layout": {{ "direction": "row", "spacing": 0.0,
                                         "justify": "start", "align": "start" }},
                  "sindri.ui.box": {{ "padding": [0.2, 0.0, 0.0, 0.5] }} }} }},
           {{ "id": "first", "name": "first", "parent": "panel",
              "transform_3d": {{ "scale": [1.0, 0.5, 1.0] }},
              "components": {{ {IMAGE} }} }},
           {{ "id": "second", "name": "second", "parent": "panel",
              "transform_3d": {{ "scale": [1.0, 0.5, 1.0] }},
              "components": {{ {IMAGE},
                  "sindri.ui.box": {{ "margin": [0.1, 0.0, 0.0, 0.25] }} }} }}"#
    ));
    let first = placement(&world, &extractor, "first");
    // Left edge at the padding: -2 + 0.5; top edge at 1 - 0.2.
    assert!(
        (first.offset - Vec2::new(-1.5 + 0.5, 0.8 - 0.25)).length() < 1.0e-5,
        "{first:?}"
    );
    let second = placement(&world, &extractor, "second");
    // The first's right edge is at -0.5; then a quarter of margin and
    // half its width. A tenth lower than the first, for its top margin.
    assert!(
        (second.offset - Vec2::new(-0.5 + 0.25 + 0.5, 0.8 - 0.1 - 0.25)).length() < 1.0e-5,
        "{second:?}"
    );
}

/// A child that grows is drawn at the size it grew to, inside a parent
/// sized to its content on the other axis.
#[test]
fn a_grown_child_is_placed_at_its_grown_size_in_a_fitted_parent() {
    let (world, extractor) = world(&format!(
        r#"{{ "id": "bar", "name": "bar",
              "transform_3d": {{ "scale": [4.0, 9.0, 1.0] }},
              "components": {{ {IMAGE},
                  "sindri.ui.layout": {{ "direction": "row", "spacing": 0.0,
                                         "fit_content": [false, true] }},
                  "sindri.ui.box": {{ "padding": [0.1, 0.0, 0.1, 0.0] }} }} }},
           {{ "id": "fill", "name": "fill", "parent": "bar",
              "transform_3d": {{ "scale": [1.0, 0.5, 1.0] }},
              "components": {{ {IMAGE}, "sindri.ui.box": {{ "grow": 1.0 }} }} }},
           {{ "id": "end", "name": "end", "parent": "bar",
              "transform_3d": {{ "scale": [1.0, 0.5, 1.0] }},
              "components": {{ {IMAGE} }} }}"#
    ));
    let bar = placement(&world, &extractor, "bar");
    // Fitted down to its content: half a unit and its padding.
    let fitted = bar.size_or([0.0; 2]);
    assert!(
        (fitted[0] - 4.0).abs() < 1.0e-5 && (fitted[1] - 0.7).abs() < 1.0e-5,
        "{bar:?}"
    );
    let fill = placement(&world, &extractor, "fill");
    let grown = fill.size_or([0.0; 2]);
    assert!(
        (grown[0] - 3.0).abs() < 1.0e-5 && (grown[1] - 0.5).abs() < 1.0e-5,
        "{fill:?}"
    );
    assert!(
        (fill.offset - Vec2::new(-0.5, 0.0)).length() < 1.0e-5,
        "{fill:?}"
    );
}
