//! Playing on a phone held sideways: the left thumb on the stick, the right
//! thumb on the look pad and the buttons, and no keys at all.

use drift::Run;

/// The controls, as `art/touch.py` names them.
const CONTROLS: &[&str] = &[
    "touch-stick-ring",
    "touch-up",
    "touch-down",
    "touch-grab",
    "touch-brake",
    "touch-use",
    "touch-reel",
    "touch-roll-left",
    "touch-roll-right",
    "touch-clip",
];

const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 720.0;

/// A tap on a button: down for one step, then up, and a step for the
/// scripts reading `Controls` to hear it.
fn tap(game: &mut Run, id: &str) {
    let at = game.on_screen(id);
    game.touch(9, at);
    game.step();
    game.lift(9);
    game.step();
    game.step();
}

fn eye_forward(game: &Run) -> glam::Vec3 {
    let eye = game.run.entity("eye").expect("the eye");
    let placed = game.run.world.world_transform(eye).expect("placed");
    glam::Quat::from_array(placed.rotation).normalize() * glam::Vec3::NEG_Z
}

#[test]
fn the_controls_are_on_screen_before_any_finger_lands() {
    let mut game = Run::open_in_hold().expect("DRIFT opens");
    game.step();
    for id in CONTROLS {
        let [x, y] = game.on_screen(id);
        assert!(
            (0.0..WIDTH).contains(&x) && (0.0..HEIGHT).contains(&y),
            "{id} is on screen: {x} {y}"
        );
    }
    assert!(
        game.on_screen("touch-stick-ring")[0] < WIDTH / 2.0,
        "stick on the left"
    );
    assert!(
        game.on_screen("touch-up")[0] > WIDTH / 2.0,
        "up on the right"
    );
}

#[test]
fn thumbs_alone_unclip_push_off_fly_and_look() {
    let mut game = Run::open_in_hold().expect("DRIFT opens");
    game.wait(0.5);
    assert!(game.has_said("Drift: holding on"), "{:?}", game.said);

    tap(&mut game, "touch-clip");
    assert!(game.has_said("Drift: unclipped"), "{:?}", game.said);
    tap(&mut game, "touch-up");
    assert!(game.has_said("Drift: pushed off"), "{:?}", game.said);
    let pushed = game.position("diver");
    game.wait(0.5);
    let coasting = game.position("diver")[0] - pushed[0];

    // Left thumb pushes the stick forward; the right thumb, down afterwards,
    // drags the look pad sideways at the same time.
    let ring = game.on_screen("touch-stick-ring");
    game.touch(1, ring);
    game.step();
    game.slide(1, [ring[0], ring[1] - 200.0]);
    let pad = [WIDTH * 0.7, HEIGHT * 0.3];
    game.touch(2, pad);
    game.step();
    let facing = eye_forward(&game);
    let from = game.position("diver");
    for i in 1..=30 {
        #[allow(clippy::cast_precision_loss)]
        game.slide(2, [pad[0] + 4.0 * i as f32, pad[1]]);
        game.step();
    }
    let flown = game.position("diver")[0] - from[0];
    assert!(
        flown > coasting + 0.15,
        "the stick fires the suit's jets forward: {flown} against coasting {coasting}"
    );
    let turned = eye_forward(&game);
    assert!(
        facing.angle_between(turned) > 0.1,
        "dragging the look pad turned the view: {facing} to {turned}"
    );
    game.lift(1);
    game.lift(2);
    game.step();
}

#[test]
fn the_up_thumb_down_first_does_not_take_the_stick() {
    let mut game = Run::open_in_hold().expect("DRIFT opens");
    game.wait(0.5);
    tap(&mut game, "touch-clip");
    tap(&mut game, "touch-up");
    assert!(game.has_said("Drift: pushed off"), "{:?}", game.said);
    game.wait(0.2);

    // Up is held first, then the stick is pushed to the right.
    let up = game.on_screen("touch-up");
    game.touch(1, up);
    game.step();
    let ring = game.on_screen("touch-stick-ring");
    game.touch(2, ring);
    game.step();
    game.slide(2, [ring[0] + 200.0, ring[1]]);
    let from = game.position("diver");
    game.wait(0.6);
    let to = game.position("diver");
    // Facing down the hold (+X), right is +Z and up is +Y.
    assert!(
        to[2] > from[2] + 0.05,
        "the stick strafed right: {from:?} {to:?}"
    );
    assert!(to[1] > from[1] + 0.05, "and up rose: {from:?} {to:?}");
}

#[test]
fn holding_grab_catches_a_handhold() {
    let mut game = Run::open_in_hold().expect("DRIFT opens");
    game.wait(0.5);
    tap(&mut game, "touch-clip");
    tap(&mut game, "touch-up");
    assert!(game.has_said("Drift: pushed off"), "{:?}", game.said);
    let grab = game.on_screen("touch-grab");
    game.touch(1, grab);
    assert!(
        game.until("Drift: caught the far handhold", 8.0),
        "{:?}",
        game.said
    );
}
