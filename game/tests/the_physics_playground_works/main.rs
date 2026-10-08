//! The Physics Playground, played: the room's tools, its global buttons and
//! every contraption, observed through what a person would see move.

mod lab;
mod support;
mod track;

use sindri_platform::Key;
use support::{Playground, said};

const WIDE: (f32, f32) = (1280.0, 720.0);

pub(crate) fn open() -> Playground {
    Playground::open(WIDE.0, WIDE.1)
}

#[test]
fn the_room_opens_on_the_whole_room_with_every_control_on_screen() {
    let playground = open();
    assert_eq!(playground.text("toy-label"), "THE WHOLE ROOM");
    assert!(
        playground.text("stats").contains("bodies"),
        "{}",
        playground.text("stats")
    );
    for name in [
        "btn-drop",
        "btn-balls",
        "btn-gravity",
        "btn-debug",
        "btn-reset",
        "tool-grab",
        "tool-blast",
        "tool-spawn",
        "tool-probe",
        "toy-prev",
        "toy-next",
        "act-1",
        "act-4",
    ] {
        assert!(
            playground
                .session
                .screen_ui()
                .rect(playground.id(name))
                .is_some(),
            "{name}"
        );
    }
}

#[test]
fn a_hundred_balls_pour_in_and_settle_without_a_failure() {
    let mut playground = open();
    let before = playground.tagged("loose");
    playground.click("btn-balls");
    playground.play(120);
    assert!(
        playground.tagged("loose") >= before + 100,
        "{}",
        playground.tagged("loose")
    );
    playground.play(120);
}

#[test]
fn grabbing_a_crate_lifts_it_and_letting_go_throws_it() {
    let mut playground = open();
    let start = playground.at("pile-crate-2");
    let pixel = playground.pixel_of_world(start);
    playground.press_at(pixel);
    for step in 1..=20u8 {
        let target = [
            start[0] + f32::from(step) * 0.2,
            // Up to just under the test track's deck, which roofs the pile.
            start[1] + f32::from(step) * 0.22,
        ];
        let pixel = playground.pixel_of_world(target);
        playground.move_to(pixel);
    }
    let lifted = playground.at("pile-crate-2");
    assert!(lifted[1] > start[1] + 3.0, "{start:?} -> {lifted:?}");
    assert!(said("Playground grabbed"));
    playground.release();
    playground.play(10);
    let thrown = playground.at("pile-crate-2");
    assert!(
        thrown[0] > lifted[0] + 0.5,
        "kept its speed: {lifted:?} -> {thrown:?}"
    );
}

#[test]
fn a_blast_scatters_the_pile() {
    let mut playground = open();
    playground.key(Key::B);
    let ball = playground.at("pile-ball-8");
    let pixel = playground.pixel_of_world([ball[0] - 0.6, ball[1] - 0.4]);
    playground.press_at(pixel);
    playground.release();
    playground.play(15);
    let after = playground.at("pile-ball-8");
    assert!(
        after.iter().zip(ball).any(|(a, b)| (a - b).abs() > 1.0),
        "{ball:?} -> {after:?}"
    );
    assert!(said("Playground blast"));
}

#[test]
fn spawning_drops_the_chosen_thing_where_the_pointer_is() {
    let mut playground = open();
    playground.click("tool-spawn");
    playground.click("tool-spawn");
    assert_eq!(playground.text("tool-spawn-label"), "SPAWN CRATE");
    let before = playground.tagged("loose");
    let pixel = playground.pixel_of_world([0.0, 6.0]);
    playground.press_at(pixel);
    playground.release();
    assert_eq!(playground.tagged("loose"), before + 1);
}

#[test]
fn gravity_cycles_through_five_worlds_and_upside_down_lifts_the_pile() {
    let mut playground = open();
    let names = ["MOON", "ZERO-G", "UPSIDE DOWN"];
    for name in names {
        playground.click("btn-gravity");
        assert_eq!(
            playground.text("btn-gravity-label"),
            format!("GRAVITY: {name}")
        );
    }
    let start = playground.at("pile-ball-3");
    playground.play(90);
    assert!(playground.at("pile-ball-3")[1] > start[1] + 3.0);
    playground.click("btn-gravity");
    playground.click("btn-gravity");
    assert_eq!(playground.text("btn-gravity-label"), "GRAVITY: EARTH");
}

#[test]
fn reset_puts_the_room_back() {
    let mut playground = open();
    let start = playground.at("pile-crate-0");
    playground.key(Key::B);
    let pixel = playground.pixel_of_world([start[0], start[1] - 0.3]);
    playground.press_at(pixel);
    playground.release();
    playground.play(30);
    let moved = playground.at("pile-crate-0");
    assert!((moved[0] - start[0]).abs() + (moved[1] - start[1]).abs() > 0.3);
    playground.click("btn-reset");
    playground.play(2);
    let back = playground.at("pile-crate-0");
    assert!(
        (back[0] - start[0]).abs() < 0.05 && (back[1] - start[1]).abs() < 0.05,
        "{back:?}"
    );
}

fn turn(playground: &Playground, name: &str) -> f32 {
    let rotation = playground
        .world
        .world_transform(playground.id(name))
        .expect("a transform")
        .rotation;
    2.0 * rotation[2].atan2(rotation[3])
}

#[test]
fn the_wrecking_ball_winds_up_swings_through_the_castle_and_can_be_cut_loose() {
    let mut playground = open();
    playground.key(Key::E);
    assert_eq!(playground.text("toy-label"), "WRECKING BALL");
    playground.key(Key::Digit1);
    playground.play(240);
    assert!(
        turn(&playground, "wreck-arm") < -1.0,
        "{}",
        turn(&playground, "wreck-arm")
    );
    let crown = playground.at("castle-crown");
    playground.key(Key::Digit2);
    playground.play(150);
    let fallen = playground.at("castle-crown");
    assert!(
        fallen[1] < crown[1] - 2.0,
        "the castle came down: {crown:?} -> {fallen:?}"
    );
    playground.key(Key::Digit1);
    playground.play(120);
    playground.key(Key::Digit3);
    playground.play(60);
    assert!(said("wrecking ball cut loose"));
    let ball = playground.at("wreck-arm");
    assert!(ball[1] < 4.0, "fell when cut: {ball:?}");
    playground.key(Key::Digit4);
    playground.play(5);
    let back = playground.at("castle-crown");
    assert!((back[1] - crown[1]).abs() < 0.2, "rebuilt: {back:?}");
}

pub(crate) fn hold(playground: &mut Playground, key: Key, steps: usize) {
    playground
        .input
        .apply(sindri_platform::InputEvent::KeyPressed(key));
    playground.play(steps);
    playground
        .input
        .apply(sindri_platform::InputEvent::KeyReleased(key));
    playground.play(2);
}

#[test]
fn the_crane_drives_lowers_grabs_the_crown_and_lifts_it_off_the_castle() {
    let mut playground = open();
    playground.key(Key::E);
    playground.key(Key::E);
    assert_eq!(playground.text("toy-label"), "GANTRY CRANE");
    let crown = playground.at("castle-crown");
    let start = playground.at("crane-trolley");
    while playground.at("crane-trolley")[0] < crown[0] - 0.6 {
        hold(&mut playground, Key::Digit2, 3);
        assert!(
            playground.at("crane-trolley")[0] < start[0] + 12.0,
            "drove past"
        );
    }
    assert!(
        (playground.at("crane-trolley")[1] - start[1]).abs() < 0.05,
        "stays on its rail"
    );
    playground.play(60);
    playground.play(120);
    playground.key(Key::Digit3);
    playground.play(150);
    let hook = playground.at("crane-hook");
    assert!(
        hook[1] < crown[1] + 1.6,
        "lowered onto the castle: {hook:?} over {crown:?}"
    );
    playground.key(Key::Digit4);
    assert!(said("crane grabbed"), "found the crown under the hook");
    playground.key(Key::Digit3);
    playground.play(120);
    let lifted = playground.at("castle-crown");
    assert!(
        lifted[1] > crown[1] + 0.5,
        "lifted off the lintel: {crown:?} -> {lifted:?}"
    );
    hold(&mut playground, Key::Digit1, 60);
    playground.play(60);
    let carried = playground.at("castle-crown");
    assert!(
        carried[0] < crown[0] - 2.5,
        "carried along the girder: {carried:?}"
    );
    playground.key(Key::Digit4);
    assert!(said("crane dropped"));
    playground.play(90);
    assert!(
        playground.at("castle-crown")[1] < carried[1] - 2.0,
        "fell when dropped"
    );
}

pub(crate) fn select(playground: &mut Playground, label: &str) {
    for _ in 0..9 {
        if playground.text("toy-label") == label {
            return;
        }
        playground.key(Key::E);
    }
    panic!("no toy {label}");
}

#[test]
fn the_cannon_breaks_glass_and_without_ccd_its_shells_pass_straight_through() {
    let mut playground = open();
    select(&mut playground, "CANNON GALLERY");
    playground.play(60);
    for _ in 0..6 {
        playground.key(Key::Digit3);
        playground.play(12);
    }
    playground.play(120);
    assert!(said("pane"), "a shell broke a pane");
    assert!(
        !said("passed through"),
        "with CCD on nothing passes through glass"
    );
    assert!(
        playground.text("hint").contains("6 fired"),
        "{}",
        playground.text("hint")
    );

    playground.key(Key::R);
    playground.play(60);
    playground.key(Key::Digit4);
    assert!(said("cannon ccd false"));
    for _ in 0..6 {
        playground.key(Key::Digit3);
        playground.play(12);
    }
    playground.play(120);
    assert!(
        said("passed through"),
        "without CCD a shell tunnels: {}",
        playground.text("hint")
    );
}

fn loose_above(playground: &Playground, height: f32) -> usize {
    playground
        .world
        .entities()
        .filter(|(entity, data)| {
            data.components.get("sindri.tags").is_some_and(|tags| {
                tags["tags"]
                    .as_array()
                    .is_some_and(|list| list.iter().any(|t| t == "loose"))
            }) && playground
                .world
                .world_transform(*entity)
                .is_some_and(|transform| transform.position[1] > height)
        })
        .count()
}

#[test]
fn multiball_runs_through_the_bumpers_drains_and_rides_the_lift_back_up() {
    let mut playground = open();
    select(&mut playground, "BUMPER PIT");
    playground.key(Key::Digit4);
    assert!(said("multiball"));
    playground.play(240);
    assert!(
        !playground.text("hint").starts_with("Score 0 "),
        "{}",
        playground.text("hint")
    );
    assert!(
        loose_above(&playground, 2.0) < 3,
        "most balls drained past the flippers"
    );
    // A round trip: the lift waits, climbs and pours its load into the chute.
    let mut poured = false;
    for _ in 0..1800 {
        playground.step();
        if said("lift pouring") && loose_above(&playground, 9.0) > 0 {
            poured = true;
            break;
        }
    }
    assert!(
        poured,
        "the lift brought balls back to the top: {}",
        playground.text("hint")
    );
}

#[test]
fn a_held_flipper_swings_up_and_drops_when_let_go() {
    let mut playground = open();
    let rest = turn(&playground, "flipper-left");
    playground
        .input
        .apply(sindri_platform::InputEvent::KeyPressed(Key::Z));
    playground.play(20);
    let up = turn(&playground, "flipper-left");
    assert!(up > rest + 0.6, "{rest} -> {up}");
    playground
        .input
        .apply(sindri_platform::InputEvent::KeyReleased(Key::Z));
    playground.play(30);
    assert!((turn(&playground, "flipper-left") - rest).abs() < 0.1);
}

fn hint_number(hint: &str, after: &str) -> f32 {
    let rest = &hint[hint
        .find(after)
        .unwrap_or_else(|| panic!("{after} in {hint}"))
        + after.len()..];
    rest.split_whitespace()
        .next()
        .and_then(|word| word.parse().ok())
        .unwrap_or_else(|| panic!("a number after {after} in {hint}"))
}

#[test]
fn the_material_lab_separates_ice_wood_and_rubber_by_friction_and_bounce() {
    let mut playground = open();
    select(&mut playground, "MATERIAL LAB");
    playground.key(Key::Digit1);
    playground.key(Key::Digit2);
    playground.play(180);
    let hint = playground.text("hint");
    let ice = hint_number(&hint, "ice ");
    let wood = hint_number(&hint, "wood ");
    let rubber = hint_number(&hint, "rubber ");
    assert!(ice > wood && wood > rubber, "{hint}");
    assert!(rubber < 1.5, "rubber grips the ramp: {hint}");
    let bounce = &hint[hint.find("BOUNCE").expect("bounce readout")..];
    let rubber = hint_number(bounce, "rubber ");
    let steel = hint_number(bounce, "steel ");
    let clay = hint_number(bounce, "clay ");
    assert!(rubber > steel && steel > clay, "{bounce}");
}

#[test]
fn dropping_the_anvil_launches_the_ball_off_the_seesaw() {
    let mut playground = open();
    select(&mut playground, "SEESAW + TRAMPOLINE");
    let rest = playground.at("seesaw-ball");
    playground.key(Key::Digit1);
    assert!(said("anvil dropped"));
    let mut highest = rest[1];
    for _ in 0..90 {
        playground.step();
        highest = highest.max(playground.at("seesaw-ball")[1]);
    }
    assert!(
        highest > rest[1] + 2.0,
        "launched: {rest:?} up to {highest}"
    );
}

#[test]
fn stiffer_trampoline_springs_give_less_under_the_same_ball() {
    let mut playground = open();
    select(&mut playground, "SEESAW + TRAMPOLINE");
    let rest = playground.at("tramp-bed")[1];
    let dip = |playground: &mut Playground| {
        playground.key(Key::Digit2);
        let mut lowest = f32::MAX;
        let mut highest = f32::MIN;
        for _ in 0..150 {
            playground.step();
            lowest = lowest.min(playground.at("tramp-bed")[1]);
            highest = highest.max(playground.at("tramp-ball")[1]);
        }
        (lowest, highest)
    };
    let (soft, bounced) = dip(&mut playground);
    assert!(soft < rest - 0.3, "the bed gave: {rest} -> {soft}");
    assert!(
        bounced > rest + 1.0,
        "and threw the ball back up: {bounced}"
    );
    for _ in 0..3 {
        playground.key(Key::Digit3);
    }
    assert!(said("trampoline 540"), "three times stiffer");
    playground.play(60);
    let (stiff, _) = dip(&mut playground);
    assert!(
        stiff > soft + 0.2,
        "soft dipped to {soft}, stiff only to {stiff}"
    );
}
