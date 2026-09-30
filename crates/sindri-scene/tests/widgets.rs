use serde_json::json;
use sindri_core::{EntityData, EntityId, Presses, Transform3D, World};
use sindri_scene::{SceneExtractor, ScreenExtent, ScreenUi, UiHierarchy, UiInput};

fn widget(world: &mut World, component: &str, payload: serde_json::Value, y: f32) -> EntityId {
    world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [0.0, y, 0.0],
            scale: [1.0, 0.3, 1.0],
            ..Transform3D::default()
        }),
        components: [(component.to_owned(), payload)].into_iter().collect(),
        ..EntityData::default()
    })
}

#[test]
fn keyboard_navigation_toggles_and_commits_unicode_once() {
    let extractor = SceneExtractor::new().unwrap();
    let mut world = World::default();
    let toggle = widget(
        &mut world,
        "sindri.ui.toggle",
        json!({"checked":false}),
        0.4,
    );
    let text = widget(
        &mut world,
        "sindri.ui.text_input",
        json!({"value":"", "max_length":3}),
        0.0,
    );
    let disabled = widget(
        &mut world,
        "sindri.ui.toggle",
        json!({"checked":false,"disabled":true}),
        -0.4,
    );
    let mut ui = ScreenUi::new();
    ui.update(
        &mut world,
        extractor.components(),
        ScreenExtent::new(800.0, 600.0),
        &Presses::default(),
    )
    .unwrap();
    ui.read_controls(
        &mut world,
        &UiInput {
            next: true,
            ..UiInput::default()
        },
    );
    assert_eq!(ui.focused(), Some(toggle));
    ui.read_controls(
        &mut world,
        &UiInput {
            activate: true,
            ..UiInput::default()
        },
    );
    assert_eq!(
        world.get(toggle).unwrap().components["sindri.ui.toggle"]["checked"],
        true
    );
    assert!(ui.changed(toggle));
    ui.read_controls(
        &mut world,
        &UiInput {
            next: true,
            text: "å猫🙂extra".to_owned(),
            ..UiInput::default()
        },
    );
    assert_eq!(ui.focused(), Some(text));
    assert_eq!(
        world.get(text).unwrap().components["sindri.ui.text_input"]["value"],
        "å猫🙂"
    );
    ui.read_controls(
        &mut world,
        &UiInput {
            backspace: true,
            submit: true,
            ..UiInput::default()
        },
    );
    assert_eq!(
        world.get(text).unwrap().components["sindri.ui.text_input"]["value"],
        "å猫"
    );
    assert!(ui.submitted(text));
    // No new input produces no change or submit event.
    ui.update(
        &mut world,
        extractor.components(),
        ScreenExtent::new(800.0, 600.0),
        &Presses::default(),
    )
    .unwrap();
    ui.read_controls(&mut world, &UiInput::default());
    assert!(!ui.changed(text));
    assert!(!ui.submitted(text));
    ui.read_controls(
        &mut world,
        &UiInput {
            next: true,
            ..UiInput::default()
        },
    );
    assert_eq!(ui.focused(), Some(toggle));
    assert!(!ui.changed(disabled));
    world.despawn_recursive(toggle).unwrap();
    ui.read_controls(&mut world, &UiInput::default());
    assert_eq!(ui.focused(), None);
}

#[test]
fn nested_scroll_moves_content_and_clips_hidden_buttons() {
    let extractor = SceneExtractor::new().unwrap();
    let mut world = World::default();
    let scroll = widget(
        &mut world,
        "sindri.ui.scroll",
        json!({"content_height":2.0,"offset":0.5}),
        0.0,
    );
    let child = widget(
        &mut world,
        "sindri.ui.button",
        json!({"label":"hidden"}),
        0.0,
    );
    world.set_parent(child, Some(scroll)).unwrap();
    let hierarchy = UiHierarchy::of(&world, extractor.components()).unwrap();
    assert!((hierarchy.placement(child).unwrap().offset.y - 0.5).abs() < 1.0e-6);
    let clip = hierarchy.clip_pixels(&world, child, [800, 600]).unwrap();
    assert_eq!(clip, [250, 255, 300, 90]);
    // The same region on a target with twice the pixels is cut in its own.
    let dense = hierarchy.clip_pixels(&world, child, [1600, 1200]).unwrap();
    assert_eq!(dense, [500, 510, 600, 180]);
    // Seen through a camera that shows the overlay at half size, centred, as
    // the editor's Scene view can, the cut follows where it is drawn.
    let half = glam::Mat4::from_scale(glam::Vec3::new(0.5 * 600.0 / 800.0, 0.5, 1.0));
    let seen = hierarchy
        .clip_pixels_through(
            &world,
            child,
            ScreenExtent::new(800.0, 600.0),
            half,
            [800, 600],
        )
        .unwrap();
    assert_eq!(seen, [325, 278, 150, 44]);
    let mut ui = ScreenUi::new();
    ui.update(
        &mut world,
        extractor.components(),
        ScreenExtent::new(800.0, 600.0),
        &Presses::default(),
    )
    .unwrap();
    // A hidden child has a placement but is never a pointer target.
    assert_eq!(ui.element_at([0.0, 0.5]), None);
}

fn lay_out(ui: &mut ScreenUi, world: &mut World, presses: &Presses) {
    let extractor = SceneExtractor::new().unwrap();
    ui.update(
        world,
        extractor.components(),
        ScreenExtent::new(800.0, 600.0),
        presses,
    )
    .unwrap();
}

/// A press at `pixel` and nothing after it, read as one frame.
fn press_at(ui: &mut ScreenUi, world: &mut World, pixel: [f32; 2]) {
    use sindri_core::{PointerDevice, PressId};
    let mut presses = Presses::default();
    presses.begin(PressId::new(PointerDevice::Mouse, 1), pixel);
    lay_out(ui, world, &presses);
    ui.read_controls(world, &UiInput::default());
}

#[test]
fn a_focused_field_keeps_space_and_enter_and_a_press_elsewhere_lets_go() {
    let mut world = World::default();
    let field = widget(
        &mut world,
        "sindri.ui.text_input",
        json!({"value":"", "max_length":8}),
        0.4,
    );
    let mut ui = ScreenUi::new();
    // (400, 180) is the middle of the field; (400, 550) is nothing at all.
    press_at(&mut ui, &mut world, [400.0, 180.0]);
    assert_eq!(ui.focused(), Some(field));
    assert!(ui.editing_text(&world));

    lay_out(&mut ui, &mut world, &Presses::default());
    ui.read_controls(
        &mut world,
        &UiInput {
            activate: true,
            submit: true,
            text: " ".to_owned(),
            ..UiInput::default()
        },
    );
    assert_eq!(
        world.get(field).unwrap().components["sindri.ui.text_input"]["value"],
        " "
    );
    assert!(ui.submitted(field));
    assert!(
        !ui.is_pressed(field),
        "Enter submits a field; it does not press it"
    );

    press_at(&mut ui, &mut world, [400.0, 550.0]);
    assert_eq!(ui.focused(), None);
    assert!(!ui.editing_text(&world));
}

#[test]
fn focus_lets_go_on_blur_escape_and_disable() {
    let mut world = World::default();
    let toggle = widget(
        &mut world,
        "sindri.ui.toggle",
        json!({"checked":false}),
        0.4,
    );
    let mut ui = ScreenUi::new();
    lay_out(&mut ui, &mut world, &Presses::default());
    let tab = UiInput {
        next: true,
        ..UiInput::default()
    };
    for leave in [
        UiInput {
            blur: true,
            ..UiInput::default()
        },
        UiInput {
            escape: true,
            ..UiInput::default()
        },
    ] {
        ui.read_controls(&mut world, &tab);
        assert_eq!(ui.focused(), Some(toggle));
        ui.read_controls(&mut world, &leave);
        assert_eq!(ui.focused(), None);
    }
    ui.read_controls(&mut world, &tab);
    world
        .get_mut(toggle)
        .unwrap()
        .components
        .get_mut("sindri.ui.toggle")
        .unwrap()["disabled"] = json!(true);
    ui.read_controls(
        &mut world,
        &UiInput {
            activate: true,
            ..UiInput::default()
        },
    );
    assert_eq!(ui.focused(), None);
    assert_eq!(
        world.get(toggle).unwrap().components["sindri.ui.toggle"]["checked"],
        false,
        "a control switched off while focused is not pressed by the key"
    );
}

#[test]
fn the_wheel_scrolls_the_region_under_the_pointer_and_tab_skips_it() {
    let mut world = World::default();
    let scroll = widget(
        &mut world,
        "sindri.ui.scroll",
        json!({"content_height":1.0,"offset":0.0}),
        0.0,
    );
    let mut ui = ScreenUi::new();
    let mut presses = Presses::default();
    presses.begin(
        sindri_core::PressId::new(sindri_core::PointerDevice::Mouse, 1),
        [400.0, 300.0],
    );
    lay_out(&mut ui, &mut world, &presses);
    ui.read_controls(
        &mut world,
        &UiInput {
            scroll: -0.25,
            next: true,
            ..UiInput::default()
        },
    );
    let offset = world.get(scroll).unwrap().components["sindri.ui.scroll"]["offset"]
        .as_f64()
        .unwrap();
    assert!((offset - 0.25).abs() < 1.0e-6, "scrolled to {offset}");
    assert!(ui.changed(scroll));
    assert_eq!(ui.focused(), None, "a scroll region is not a tab stop");

    // Past the end is clamped to the end: 1.0 of content in 0.3 of view.
    ui.read_controls(
        &mut world,
        &UiInput {
            scroll: -5.0,
            ..UiInput::default()
        },
    );
    let offset = world.get(scroll).unwrap().components["sindri.ui.scroll"]["offset"]
        .as_f64()
        .unwrap();
    assert!((offset - 0.7).abs() < 1.0e-5, "clamped to {offset}");
}

#[test]
fn widget_payloads_that_cannot_hold_are_refused() {
    use sindri_scene::{UiScrollComponent, UiTextInputComponent};
    let field = |payload| serde_json::from_value::<UiTextInputComponent>(payload).is_ok();
    assert!(field(json!({"value":"ab", "max_length":2})));
    assert!(
        !field(json!({"value":"abc", "max_length":2})),
        "longer than it may be"
    );
    assert!(!field(json!({"value":"", "max_length":0})));
    assert!(!field(json!({"value":"", "max_length":100_000})));
    let scroll = |payload| serde_json::from_value::<UiScrollComponent>(payload).is_ok();
    assert!(scroll(json!({"content_height":2.0, "offset":0.5})));
    assert!(!scroll(json!({"content_height":-1.0})));
    assert!(!scroll(json!({"content_height":1.0, "offset":-0.5})));
}

/// A laid-out list inside a scroll region keeps its rows' size, hangs from
/// the region's top edge, and scrolls exactly as far as it is tall -- the
/// region is measured, not told.
#[test]
fn a_scroll_region_measures_the_list_it_holds() {
    let extractor = SceneExtractor::new().unwrap();
    let mut world = World::default();
    let region = widget(&mut world, "sindri.ui.scroll", json!({"offset":99.0}), 0.0);
    world.get_mut(region).unwrap().components.insert(
        "sindri.ui.layout".to_owned(),
        json!({"direction":"column","spacing":0.0,"justify":"start"}),
    );
    let rows: Vec<EntityId> = (0..10)
        .map(|_| {
            let row = world.spawn(EntityData {
                transform_3d: Some(Transform3D {
                    scale: [0.8, 0.1, 1.0],
                    ..Transform3D::default()
                }),
                components: [("sindri.ui.button".to_owned(), json!({"label":"row"}))]
                    .into_iter()
                    .collect(),
                ..EntityData::default()
            });
            world.set_parent(row, Some(region)).unwrap();
            row
        })
        .collect();
    let hierarchy = UiHierarchy::of(&world, extractor.components()).unwrap();
    let content = hierarchy.scroll_content(region).unwrap();
    assert!((content - 1.0).abs() < 1.0e-5, "measured {content}");
    // Scrolled to the end, clamped: 1.0 of rows in 0.3 of view.
    let last = hierarchy.placement(rows[9]).unwrap();
    assert!((last.offset.y - (-0.15 + 0.05)).abs() < 1.0e-5, "{last:?}");
    assert!(
        (last.size.unwrap().y - 0.1).abs() < 1.0e-6,
        "rows keep their height rather than squeezing into the view"
    );

    world
        .get_mut(region)
        .unwrap()
        .components
        .get_mut("sindri.ui.scroll")
        .unwrap()["offset"] = json!(0.0);
    let hierarchy = UiHierarchy::of(&world, extractor.components()).unwrap();
    let first = hierarchy.placement(rows[0]).unwrap();
    assert!(
        (first.offset.y - (0.15 - 0.05)).abs() < 1.0e-5,
        "the first row starts at the top: {first:?}"
    );
}
