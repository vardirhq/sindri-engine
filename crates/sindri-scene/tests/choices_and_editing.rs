//! Radio groups, dropdowns, caret editing, directional focus, and where text
//! is pressed: the screen UI read the way a host reads it, one step at a time.

use serde_json::json;
use sindri_core::{
    EntityData, EntityId, PointerDevice, PressId, PressPhase, Presses, Transform3D, World,
};
use sindri_scene::{SceneExtractor, ScreenExtent, ScreenUi, UiInput};

/// 800 by 600: one overlay unit is 300 pixels, and the middle is (400, 300).
const EXTENT: (f32, f32) = (800.0, 600.0);

fn element(
    world: &mut World,
    at: [f32; 2],
    size: [f32; 2],
    components: &[(&str, serde_json::Value)],
) -> EntityId {
    world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [at[0], at[1], 0.0],
            scale: [size[0], size[1], 1.0],
            ..Transform3D::default()
        }),
        components: components
            .iter()
            .map(|(name, payload)| ((*name).to_owned(), payload.clone()))
            .collect(),
        ..EntityData::default()
    })
}

fn pixel(at: [f32; 2]) -> [f32; 2] {
    [400.0 + at[0] * 300.0, 300.0 - at[1] * 300.0]
}

struct Screen {
    ui: ScreenUi,
    world: World,
    extractor: SceneExtractor,
}

impl Screen {
    fn new(world: World) -> Self {
        let mut screen = Self {
            ui: ScreenUi::new(),
            world,
            extractor: SceneExtractor::new().unwrap(),
        };
        screen.step(&Presses::default(), &UiInput::default());
        screen
    }

    fn step(&mut self, presses: &Presses, input: &UiInput) {
        self.ui
            .update(
                &mut self.world,
                self.extractor.components(),
                ScreenExtent::new(EXTENT.0, EXTENT.1),
                presses,
            )
            .unwrap();
        self.ui.read_controls(&mut self.world, input);
    }

    fn keys(&mut self, input: UiInput) {
        self.step(&Presses::default(), &input);
    }

    /// A press and a release at `at`, as two steps.
    fn click(&mut self, at: [f32; 2]) {
        let id = PressId::new(PointerDevice::Mouse, 1);
        let mut presses = Presses::default();
        presses.begin(id, pixel(at));
        self.step(&presses, &UiInput::default());
        presses.advance(std::time::Duration::from_millis(16));
        presses.finish(id, PressPhase::Ended);
        self.step(&presses, &UiInput::default());
    }

    fn field(&self, entity: EntityId, component: &str, name: &str) -> serde_json::Value {
        self.world.get(entity).unwrap().components[component][name].clone()
    }
}

#[test]
fn a_radio_group_keeps_exactly_one_chosen() {
    let mut world = World::default();
    let options: Vec<EntityId> = (0..3)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let y = 0.5 - i as f32 * 0.5;
            element(
                &mut world,
                [0.0, y],
                [1.0, 0.3],
                &[(
                    "sindri.ui.toggle",
                    json!({"group": "speed", "checked": i == 0}),
                )],
            )
        })
        .collect();
    let mut screen = Screen::new(world);
    screen.click([0.0, 0.0]);
    let checked: Vec<bool> = options
        .iter()
        .map(|o| screen.field(*o, "sindri.ui.toggle", "checked") == json!(true))
        .collect();
    assert_eq!(checked, [false, true, false]);

    // Choosing the chosen one again leaves it chosen, and reports nothing.
    screen.click([0.0, 0.0]);
    assert_eq!(
        screen.field(options[1], "sindri.ui.toggle", "checked"),
        json!(true)
    );
    assert!(!screen.ui.changed(options[1]));
}

/// A dropdown at the top with a popup of three options under it.
fn dropdown(world: &mut World) -> (EntityId, EntityId, Vec<EntityId>) {
    let header = element(
        world,
        [0.0, 0.7],
        [1.0, 0.2],
        &[("sindri.ui.dropdown", json!({"selected": 0}))],
    );
    let popup = element(world, [0.0, -0.4], [1.0, 0.6], &[]);
    world.set_parent(popup, Some(header)).unwrap();
    let options = (0..3)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let y = 0.2 - i as f32 * 0.2;
            let option = element(
                world,
                [0.0, y],
                [1.0, 0.18],
                &[("sindri.ui.option", json!({"label": format!("option {i}")}))],
            );
            world.set_parent(option, Some(popup)).unwrap();
            option
        })
        .collect();
    (header, popup, options)
}

#[test]
fn a_dropdown_opens_chooses_and_closes() {
    let mut world = World::default();
    let (header, popup, options) = dropdown(&mut world);
    let elsewhere = element(
        &mut world,
        [0.0, -0.9],
        [1.0, 0.15],
        &[("sindri.ui.button", json!({}))],
    );
    let mut screen = Screen::new(world);
    assert!(
        !screen.world.is_active(popup),
        "closed, the list is put away"
    );
    assert_eq!(
        screen.field(options[0], "sindri.ui.option", "checked"),
        json!(true)
    );

    screen.click([0.0, 0.7]);
    assert_eq!(
        screen.field(header, "sindri.ui.dropdown", "open"),
        json!(true)
    );
    assert!(screen.world.is_active(popup));

    // The third option sits at 0.7 - 0.4 - 0.2 = 0.1 in overlay units.
    screen.click([0.0, 0.1]);
    assert_eq!(
        screen.field(header, "sindri.ui.dropdown", "selected"),
        json!(2)
    );
    assert_eq!(
        screen.field(header, "sindri.ui.dropdown", "open"),
        json!(false)
    );
    assert_eq!(
        screen.field(options[2], "sindri.ui.option", "checked"),
        json!(true)
    );
    assert_eq!(
        screen.field(options[0], "sindri.ui.option", "checked"),
        json!(false)
    );
    assert!(!screen.world.is_active(popup));

    // Escape and a press elsewhere both put an open list away.
    screen.click([0.0, 0.7]);
    screen.keys(UiInput {
        escape: true,
        ..UiInput::default()
    });
    assert_eq!(
        screen.field(header, "sindri.ui.dropdown", "open"),
        json!(false)
    );
    screen.click([0.0, 0.7]);
    screen.click([0.0, -0.9]);
    assert_eq!(
        screen.field(header, "sindri.ui.dropdown", "open"),
        json!(false)
    );
    assert_eq!(screen.ui.focused(), Some(elsewhere));
}

#[test]
fn a_dropdown_is_driven_by_the_keyboard() {
    let mut world = World::default();
    let (header, _, _) = dropdown(&mut world);
    let mut screen = Screen::new(world);
    screen.keys(UiInput {
        next: true,
        ..UiInput::default()
    });
    assert_eq!(screen.ui.focused(), Some(header));
    screen.keys(UiInput {
        activate: true,
        ..UiInput::default()
    });
    assert_eq!(
        screen.field(header, "sindri.ui.dropdown", "open"),
        json!(true)
    );
    for _ in 0..2 {
        screen.keys(UiInput {
            down: true,
            ..UiInput::default()
        });
    }
    screen.keys(UiInput {
        activate: true,
        ..UiInput::default()
    });
    assert_eq!(
        screen.field(header, "sindri.ui.dropdown", "selected"),
        json!(2)
    );
    assert_eq!(
        screen.field(header, "sindri.ui.dropdown", "open"),
        json!(false)
    );
    assert_eq!(
        screen.ui.focused(),
        Some(header),
        "focus returns to the header"
    );
}

#[test]
fn a_field_edits_at_its_caret_selects_and_copies() {
    let mut world = World::default();
    let field = element(
        &mut world,
        [0.0, 0.0],
        [1.0, 0.3],
        &[(
            "sindri.ui.text_input",
            json!({"value": "hello", "max_length": 8}),
        )],
    );
    let mut screen = Screen::new(world);
    screen.click([0.0, 0.0]);
    let value = |screen: &Screen| screen.field(field, "sindri.ui.text_input", "value");
    let caret = |screen: &Screen| screen.ui.caret(&screen.world, field).unwrap();
    assert_eq!(
        caret(&screen).caret,
        5,
        "a field gains focus with its caret at the end"
    );

    for _ in 0..2 {
        screen.keys(UiInput {
            left: true,
            ..UiInput::default()
        });
    }
    screen.keys(UiInput {
        text: "X".to_owned(),
        ..UiInput::default()
    });
    assert_eq!(value(&screen), json!("helXlo"));
    assert_eq!(caret(&screen).caret, 4);
    assert_eq!(
        screen.ui.focused(),
        Some(field),
        "left and right edit, not navigate"
    );

    screen.keys(UiInput {
        home: true,
        extend: true,
        ..UiInput::default()
    });
    assert_eq!(caret(&screen).selection(), 0..4);
    screen.keys(UiInput {
        cut: true,
        ..UiInput::default()
    });
    assert_eq!(screen.ui.take_copied().as_deref(), Some("helX"));
    assert_eq!(value(&screen), json!("lo"));

    screen.keys(UiInput {
        delete: true,
        ..UiInput::default()
    });
    assert_eq!(value(&screen), json!("o"));

    // Pasted text fills only the room the field has left.
    screen.keys(UiInput {
        select_all: true,
        ..UiInput::default()
    });
    screen.keys(UiInput {
        text: "Nøva 7 is far too long".to_owned(),
        ..UiInput::default()
    });
    assert_eq!(value(&screen), json!("Nøva 7 i"));
    assert_eq!(caret(&screen).caret, 8);
}

#[test]
fn the_arrows_move_focus_toward_the_nearest_control_that_way() {
    let mut world = World::default();
    let mut grid = Vec::new();
    for (i, (x, y)) in [(-0.5, 0.5), (0.5, 0.5), (-0.5, -0.5), (0.5, -0.5)]
        .into_iter()
        .enumerate()
    {
        grid.push(element(
            &mut world,
            [x, y],
            [0.4, 0.3],
            &[("sindri.ui.button", json!({"autofocus": i == 0}))],
        ));
    }
    let mut screen = Screen::new(world);
    assert_eq!(
        screen.ui.focused(),
        Some(grid[0]),
        "the autofocus button takes focus when it appears"
    );
    let arrow = |screen: &mut Screen, input: UiInput| screen.keys(input);
    arrow(
        &mut screen,
        UiInput {
            down: true,
            ..UiInput::default()
        },
    );
    assert_eq!(screen.ui.focused(), Some(grid[2]));
    arrow(
        &mut screen,
        UiInput {
            right: true,
            ..UiInput::default()
        },
    );
    assert_eq!(screen.ui.focused(), Some(grid[3]));
    arrow(
        &mut screen,
        UiInput {
            up: true,
            ..UiInput::default()
        },
    );
    assert_eq!(screen.ui.focused(), Some(grid[1]));
    arrow(
        &mut screen,
        UiInput {
            up: true,
            ..UiInput::default()
        },
    );
    assert_eq!(
        screen.ui.focused(),
        Some(grid[1]),
        "nothing further up stays put"
    );

    // Let go, the arrows bring focus back only to the screen that asked.
    arrow(
        &mut screen,
        UiInput {
            escape: true,
            ..UiInput::default()
        },
    );
    arrow(
        &mut screen,
        UiInput {
            left: true,
            ..UiInput::default()
        },
    );
    assert_eq!(screen.ui.focused(), Some(grid[0]));
}

/// A game that moves with the arrows and jumps with Space keeps its buttons
/// out of it: with nothing focused and nothing asking, the arrows focus
/// nothing, so Space presses nothing.
#[test]
fn the_arrows_do_not_take_focus_for_a_screen_that_did_not_ask() {
    let mut world = World::default();
    let pause = element(
        &mut world,
        [0.8, 0.8],
        [0.2, 0.2],
        &[("sindri.ui.button", json!({}))],
    );
    let mut screen = Screen::new(world);
    for input in [
        UiInput {
            right: true,
            ..UiInput::default()
        },
        UiInput {
            down: true,
            ..UiInput::default()
        },
        UiInput {
            activate: true,
            ..UiInput::default()
        },
    ] {
        screen.keys(input);
    }
    assert_eq!(screen.ui.focused(), None);
    assert!(!screen.ui.is_pressed(pause));
}

#[test]
fn a_row_reached_by_the_keyboard_is_scrolled_into_view() {
    let mut world = World::default();
    let region = element(
        &mut world,
        [0.0, 0.0],
        [1.0, 0.6],
        &[
            ("sindri.ui.scroll", json!({})),
            (
                "sindri.ui.layout",
                json!({"direction": "column", "spacing": 0.0, "justify": "start"}),
            ),
        ],
    );
    let rows: Vec<EntityId> = (0..8)
        .map(|i| {
            let row = element(
                &mut world,
                [0.0, 0.0],
                [0.8, 0.2],
                &[("sindri.ui.button", json!({"autofocus": i == 0}))],
            );
            world.set_parent(row, Some(region)).unwrap();
            row
        })
        .collect();
    let mut screen = Screen::new(world);
    for _ in 0..5 {
        screen.keys(UiInput {
            down: true,
            ..UiInput::default()
        });
    }
    assert_eq!(screen.ui.focused(), Some(rows[5]));
    // The sixth row's bottom is 1.2 below the top of a region 0.6 tall.
    let offset = screen
        .field(region, "sindri.ui.scroll", "offset")
        .as_f64()
        .unwrap();
    assert!((offset - 0.6).abs() < 1.0e-4, "scrolled to {offset}");
}

#[test]
fn text_is_pressed_where_it_is_drawn() {
    let mut world = World::default();
    let label = element(
        &mut world,
        [-0.5, 0.5],
        [0.6, 0.2],
        &[
            (
                "sindri.ui.text",
                json!({"text": "Settings", "font": "fonts/Inter.ttf", "anchor": "top_left", "bounds": [0.6, 0.2]}),
            ),
            ("sindri.ui.button", json!({})),
        ],
    );
    let screen = Screen::new(world);
    let rect = screen.ui.rect(label).unwrap();
    // Its top-left corner sits at the anchor point plus the offset, and the
    // box runs right and down from there, as the words do.
    let corner = [-800.0 / 600.0 - 0.5, 1.0 + 0.5];
    assert!(
        (rect.center[0] - (corner[0] + 0.3)).abs() < 1.0e-5,
        "{rect:?}"
    );
    assert!(
        (rect.center[1] - (corner[1] - 0.1)).abs() < 1.0e-5,
        "{rect:?}"
    );
}
