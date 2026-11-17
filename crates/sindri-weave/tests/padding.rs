use sindri_core::{SceneDocument, World};
use sindri_weave::PresentationWorld;
use weave::{Viewport, parse};

#[test]
fn percentage_children_use_the_parent_content_box() {
    let document = SceneDocument::from_json(
        r#"{
            "format_version": 10,
            "metadata": { "name": "weave-padding" },
            "entities": [
                {
                    "id": "child",
                    "name": "child",
                    "parent": "panel",
                    "transform_3d": { "scale": [0.1, 0.1, 1.0] },
                    "components": {
                        "sindri.ui.shape": { "kind": "rect", "anchor": "center" }
                    }
                },
                {
                    "id": "panel",
                    "name": "panel",
                    "transform_3d": { "scale": [0.2, 0.2, 1.0] },
                    "components": {
                        "sindri.ui.shape": { "kind": "rect", "anchor": "center" }
                    }
                }
            ]
        }"#,
    )
    .expect("scene parses");
    let source = World::from_scene(&document).expect("scene loads").world;
    let sheet = parse(
        r#"
            #panel {
                width: 600px;
                height: 400px;
                padding: 50px;
            }
            #child {
                width: 100%;
                height: 100%;
            }
        "#,
    )
    .expect("Weave parses");

    let styled = PresentationWorld::resolve(
        &source,
        &sheet,
        Viewport {
            width: 1_200.0,
            height: 800.0,
        },
    )
    .expect("styles resolve");

    let scale = |name: &str| {
        styled
            .world()
            .entities()
            .find(|(_, data)| data.name.as_deref() == Some(name))
            .and_then(|(_, data)| data.transform_3d)
            .expect("styled entity")
            .scale_2d()
    };

    assert_eq!(scale("panel"), [1.5, 1.0]);
    assert_eq!(scale("child"), [1.25, 0.75]);

    let source_child = source
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("child"))
        .and_then(|(_, data)| data.transform_3d)
        .expect("source child")
        .scale_2d();
    assert_eq!(source_child, [0.1, 0.1]);
}

#[test]
fn negative_padding_is_rejected() {
    let document = SceneDocument::from_json(
        r#"{
            "format_version": 10,
            "metadata": { "name": "weave-padding-invalid" },
            "entities": [{
                "id": "panel",
                "components": {
                    "sindri.ui.shape": { "kind": "rect", "anchor": "center" }
                }
            }]
        }"#,
    )
    .expect("scene parses");
    let source = World::from_scene(&document).expect("scene loads").world;
    let sheet = parse("#panel { padding: -8px; }").expect("Weave parses");

    let error = PresentationWorld::resolve(
        &source,
        &sheet,
        Viewport {
            width: 1_200.0,
            height: 800.0,
        },
    )
    .expect_err("negative padding is invalid");

    assert!(error.to_string().contains("padding"));
    assert!(error.to_string().contains("-8px"));
}

#[test]
fn padding_and_margin_reach_the_box_side_by_side() {
    let document = SceneDocument::from_json(
        r#"{
            "format_version": 10,
            "metadata": { "name": "weave-box" },
            "entities": [
                {
                    "id": "panel",
                    "name": "panel",
                    "components": {
                        "sindri.ui.shape": { "kind": "rect", "anchor": "center" }
                    }
                },
                {
                    "id": "child",
                    "name": "child",
                    "parent": "panel",
                    "components": {
                        "sindri.ui.shape": { "kind": "rect", "anchor": "center" }
                    }
                }
            ]
        }"#,
    )
    .expect("scene parses");
    let source = World::from_scene(&document).expect("scene loads").world;
    // 800 pixels high is two overlay units, so 40px is 0.1.
    let sheet = parse(
        r"
            #panel { width: 800px; padding: 40px 80px; padding-top: 0; }
            #child { margin: 10%; margin-bottom: 40px; }
        ",
    )
    .expect("Weave parses");
    let styled = PresentationWorld::resolve(
        &source,
        &sheet,
        Viewport {
            width: 1_200.0,
            height: 800.0,
        },
    )
    .expect("styles resolve");

    let sides = |name: &str, field: &str| -> Vec<f32> {
        let (_, data) = styled
            .world()
            .entities()
            .find(|(_, data)| data.name.as_deref() == Some(name))
            .expect("entity is there");
        data.components["sindri.ui.box"][field]
            .as_array()
            .expect("four sides")
            .iter()
            .map(|side| side.as_f64().expect("a number") as f32)
            .collect()
    };
    let near = |got: Vec<f32>, want: [f32; 4]| {
        assert!(
            got.iter()
                .zip(want)
                .all(|(got, want)| (got - want).abs() < 1.0e-5),
            "{got:?} is not {want:?}"
        );
    };
    // The later `padding-top` overrides one side of the shorthand, as in CSS.
    near(sides("panel", "padding"), [0.0, 0.2, 0.1, 0.2]);
    // A percentage is of the panel's content width: 2.0 less 0.4 of padding.
    near(sides("child", "margin"), [0.16, 0.16, 0.1, 0.16]);
}

#[test]
fn calc_mixes_units_in_sizes_and_sides() {
    let document = SceneDocument::from_json(
        r#"{
            "format_version": 10,
            "metadata": { "name": "weave-calc" },
            "entities": [
                { "id": "panel", "name": "panel",
                  "components": { "sindri.ui.shape": { "kind": "rect" } } },
                { "id": "child", "name": "child", "parent": "panel",
                  "components": { "sindri.ui.shape": { "kind": "rect" } } }
            ]
        }"#,
    )
    .expect("scene parses");
    let source = World::from_scene(&document).expect("scene loads").world;
    // 800 pixels high is two units: 40px is a tenth, 25vh is half a unit.
    let sheet = parse(
        r"
            :root { --inset: 20px; }
            #panel { width: 800px; height: 400px; }
            #child {
                width: calc(100% - 2 * var(--inset));
                height: calc(25vh + 40px);
                padding: calc(var(--inset) * 2) 0;
            }
        ",
    )
    .expect("Weave parses");
    let styled = PresentationWorld::resolve(
        &source,
        &sheet,
        Viewport {
            width: 1_200.0,
            height: 800.0,
        },
    )
    .expect("styles resolve");
    let (_, child) = styled
        .world()
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("child"))
        .expect("the child is there");
    let size = child.transform_3d.expect("sized").scale_2d();
    // The panel is two units wide; less 40px (a tenth) of insets.
    assert!((size[0] - 1.9).abs() < 1.0e-5, "{size:?}");
    assert!((size[1] - 0.6).abs() < 1.0e-5, "{size:?}");
    let top = child.components["sindri.ui.box"]["padding"][0]
        .as_f64()
        .expect("a side");
    assert!((top - 0.1).abs() < 1.0e-6, "{top}");
}

/// The width `clamp(MIN, VAL, MAX)` gives one panel on a screen `width`
/// pixels across and 800 high, in overlay units (400 pixels to a unit).
fn clamped_width(width: f32) -> f32 {
    let document = SceneDocument::from_json(
        r#"{
            "format_version": 10,
            "metadata": { "name": "weave-clamp" },
            "entities": [
                { "id": "card", "name": "card",
                  "components": { "sindri.ui.shape": { "kind": "rect" } } }
            ]
        }"#,
    )
    .expect("scene parses");
    let source = World::from_scene(&document).expect("scene loads").world;
    let sheet = parse(
        r"
            :root { --least: 200px; }
            #card { width: clamp(var(--least), 40vw, 400px); height: min(10vh, 60px); }
        ",
    )
    .expect("Weave parses");
    let styled = PresentationWorld::resolve(
        &source,
        &sheet,
        Viewport {
            width,
            height: 800.0,
        },
    )
    .expect("styles resolve");
    let (_, card) = styled.world().entities().next().expect("the card");
    let size = card.transform_3d.expect("sized").scale_2d();
    // 10vh is 80px here, so min() picks 60px: 0.15 units.
    assert!((size[1] - 0.15).abs() < 1.0e-5, "{size:?}");
    size[0] * 400.0
}

#[test]
fn clamp_keeps_a_card_between_its_bounds_on_any_screen() {
    // 40vw of a phone, a tablet and a monitor: 160, 320 and 768 pixels.
    assert!(
        (clamped_width(400.0) - 200.0).abs() < 1.0e-2,
        "held at its least"
    );
    assert!(
        (clamped_width(800.0) - 320.0).abs() < 1.0e-2,
        "a share of the screen"
    );
    assert!(
        (clamped_width(1920.0) - 400.0).abs() < 1.0e-2,
        "held at its most"
    );
}
