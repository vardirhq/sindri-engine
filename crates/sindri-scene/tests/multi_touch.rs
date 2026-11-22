//! Buttons under more than one finger: a thumb steering and a thumb pressing.

use serde_json::json;
use sindri_core::{
    EntityData, EntityId, PointerDevice, PressId, PressPhase, Presses, SceneComponent, Transform3D,
    World,
};
use sindri_scene::{SceneExtractor, ScreenExtent, ScreenUi, UiButtonComponent};
use std::time::Duration;

const WIDTH: f32 = 800.0;
const HEIGHT: f32 = 400.0;

fn button(world: &mut World, at: [f32; 2]) -> EntityId {
    world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [at[0], at[1], 0.0],
            scale: [0.3, 0.3, 1.0],
            ..Transform3D::default()
        }),
        components: [(
            UiButtonComponent::TYPE_NAME.to_owned(),
            json!({ "label": "Jump" }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    })
}

/// Where an overlay point is, in pixels from the top left.
fn pixel(at: [f32; 2]) -> [f32; 2] {
    let half = HEIGHT / 2.0;
    [WIDTH / 2.0 + at[0] * half, half - at[1] * half]
}

fn finger(raw: u64) -> PressId {
    PressId::new(PointerDevice::Touch, raw)
}

struct Screen {
    world: World,
    ui: ScreenUi,
    presses: Presses,
    jump: EntityId,
    fire: EntityId,
}

impl Screen {
    fn new() -> Self {
        let mut world = World::default();
        let jump = button(&mut world, [1.2, -0.6]);
        let fire = button(&mut world, [0.7, -0.6]);
        Self {
            world,
            ui: ScreenUi::new(),
            presses: Presses::default(),
            jump,
            fire,
        }
    }

    fn step(&mut self) {
        let components = SceneExtractor::new()
            .expect("the builtin components register")
            .components()
            .clone();
        self.ui
            .update(
                &mut self.world,
                &components,
                ScreenExtent::new(WIDTH, HEIGHT),
                &self.presses,
            )
            .expect("registered");
        self.presses.advance(Duration::from_millis(16));
    }

    fn down(&mut self, id: PressId, at: [f32; 2]) {
        self.presses.begin(id, pixel(at));
    }

    fn up(&mut self, id: PressId) {
        self.presses.finish(id, PressPhase::Ended);
    }
}

#[test]
fn a_second_finger_presses_a_button_while_the_first_steers() {
    let mut screen = Screen::new();
    let steering = finger(1);
    screen.down(steering, [-1.2, -0.5]);
    screen.step();

    let thumb = finger(2);
    screen.down(thumb, [1.2, -0.6]);
    screen.step();
    assert!(
        screen.ui.is_held(screen.jump),
        "the second thumb holds jump"
    );
    assert!(screen.ui.holds_press(thumb));
    assert!(
        !screen.ui.holds_press(steering),
        "the steering thumb is free"
    );

    screen.up(thumb);
    screen.step();
    assert!(screen.ui.is_pressed(screen.jump), "letting go clicks it");
    assert!(!screen.ui.is_held(screen.jump));
}

#[test]
fn a_button_stays_held_when_the_finger_before_it_lifts() {
    let mut screen = Screen::new();
    screen.down(finger(1), [-1.2, -0.5]);
    screen.down(finger(2), [1.2, -0.6]);
    screen.step();
    screen.up(finger(1));
    screen.step();
    screen.step();
    assert!(screen.ui.is_held(screen.jump));
}

#[test]
fn a_finger_on_a_button_first_does_not_block_a_second_button() {
    let mut screen = Screen::new();
    screen.down(finger(1), [1.2, -0.6]);
    screen.step();
    screen.down(finger(2), [0.7, -0.6]);
    screen.step();
    assert!(screen.ui.is_held(screen.jump));
    assert!(screen.ui.is_held(screen.fire), "two buttons, two thumbs");
}

#[test]
fn a_finger_that_slides_off_lets_go_and_does_not_click() {
    let mut screen = Screen::new();
    let thumb = finger(3);
    screen.down(thumb, [1.2, -0.6]);
    screen.step();
    screen.presses.move_to(thumb, pixel([0.0, 0.6]));
    screen.step();
    assert!(!screen.ui.is_held(screen.jump));
    screen.up(thumb);
    screen.step();
    assert!(!screen.ui.is_pressed(screen.jump));
}

#[test]
fn the_stick_waits_for_a_thumb_that_lands_off_the_buttons() {
    let mut screen = Screen::new();
    // The jump thumb is down first: it is jumping, not steering.
    screen.down(finger(1), [1.2, -0.6]);
    screen.step();
    assert!(!screen.ui.stick().is_engaged());

    // The steering thumb lands afterwards and takes the stick.
    screen.down(finger(2), [-1.2, -0.5]);
    screen.step();
    assert!(screen.ui.stick().is_engaged());
    screen.presses.move_to(finger(2), pixel([-0.5, -0.5]));
    screen.step();
    assert!(screen.ui.stick().value()[0] > 0.5, "pulled right");
    assert!(screen.ui.is_held(screen.jump), "and jump is still held");
}

#[test]
fn a_look_pad_reads_how_far_its_thumb_moved() {
    let mut screen = Screen::new();
    let thumb = finger(4);
    screen.down(thumb, [1.2, -0.6]);
    screen.step();
    screen.presses.move_to(thumb, pixel([1.3, -0.4]));
    screen.step();
    let moved = screen.ui.drag(screen.jump);
    assert!((moved[0] - 0.1).abs() < 1.0e-3 && (moved[1] - 0.2).abs() < 1.0e-3);
    // Past the edge, still the same thumb.
    screen.presses.move_to(thumb, pixel([1.6, -0.4]));
    screen.step();
    assert!((screen.ui.drag(screen.jump)[0] - 0.3).abs() < 1.0e-3);
    screen.step();
    let still = screen.ui.drag(screen.jump);
    assert!(still[0].abs() < 1.0e-6 && still[1].abs() < 1.0e-6, "still");
}
