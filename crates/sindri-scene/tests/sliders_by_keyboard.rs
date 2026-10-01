//! A focused slider moved by the arrows or a pad, as a native range input is:
//! along its axis the arrows change its value, across it they move focus.

use serde_json::json;
use sindri_core::{EntityData, EntityId, Presses, Transform3D, World};
use sindri_scene::{SceneExtractor, ScreenExtent, ScreenUi, UiInput};

fn element(world: &mut World, y: f32, component: (&str, serde_json::Value)) -> EntityId {
    world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [0.0, y, 0.0],
            scale: [1.0, 0.2, 1.0],
            ..Transform3D::default()
        }),
        components: [(component.0.to_owned(), component.1)].into(),
        ..EntityData::default()
    })
}

fn step(ui: &mut ScreenUi, world: &mut World, extractor: &SceneExtractor, input: &UiInput) {
    ui.update(
        world,
        extractor.components(),
        ScreenExtent::new(800.0, 600.0),
        &Presses::default(),
    )
    .unwrap();
    ui.read_controls(world, input);
}

fn value(world: &World, slider: EntityId) -> f64 {
    world.get(slider).unwrap().components["sindri.ui.slider"]["value"]
        .as_f64()
        .unwrap()
}

#[test]
fn the_arrows_move_a_focused_slider_by_its_step_and_focus_across_it() {
    let mut world = World::default();
    let volume = element(
        &mut world,
        0.3,
        (
            "sindri.ui.slider",
            json!({"min": 0.0, "max": 1.0, "step": 0.1, "value": 0.5}),
        ),
    );
    let fine = element(
        &mut world,
        0.0,
        (
            "sindri.ui.slider",
            json!({"min": 0.0, "max": 10.0, "value": 0.0}),
        ),
    );
    let done = element(&mut world, -0.3, ("sindri.ui.button", json!({})));
    let extractor = SceneExtractor::new().unwrap();
    let mut ui = ScreenUi::new();
    step(&mut ui, &mut world, &extractor, &UiInput::default());

    let tab = UiInput {
        next: true,
        ..UiInput::default()
    };
    let right = UiInput {
        right: true,
        ..UiInput::default()
    };
    let left = UiInput {
        left: true,
        ..UiInput::default()
    };
    let down = UiInput {
        down: true,
        ..UiInput::default()
    };
    step(&mut ui, &mut world, &extractor, &tab);
    assert_eq!(ui.focused(), Some(volume));

    step(&mut ui, &mut world, &extractor, &right);
    assert!((value(&world, volume) - 0.6).abs() < 1.0e-5);
    assert!(ui.slider_changed(volume), "a nudge is a change");
    assert_eq!(ui.focused(), Some(volume), "along the axis, focus stays");
    step(&mut ui, &mut world, &extractor, &UiInput::default());
    assert!(!ui.slider_changed(volume), "only on the step it moved");
    step(&mut ui, &mut world, &extractor, &left);
    step(&mut ui, &mut world, &extractor, &left);
    assert!((value(&world, volume) - 0.4).abs() < 1.0e-5);

    // Across the axis the arrows still move focus, to a slider with no step
    // that moves by a twentieth of its range.
    step(&mut ui, &mut world, &extractor, &down);
    assert_eq!(ui.focused(), Some(fine));
    step(&mut ui, &mut world, &extractor, &right);
    assert!((value(&world, fine) - 0.5).abs() < 1.0e-5);
    step(&mut ui, &mut world, &extractor, &down);
    assert_eq!(ui.focused(), Some(done));
}
