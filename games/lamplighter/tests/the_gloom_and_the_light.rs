//! The gloom keep to the dark and hunt: they creep toward Wick where little
//! light reaches them, hold where a light begins, wind up and pounce from its edge,
//! scorch his lantern when a pounce lands, and thin where a light is strong.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn position(run: &Run, id: &str) -> [f32; 2] {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    let at = run
        .world
        .get(entity)
        .expect("it exists")
        .transform_3d
        .expect("it has a place")
        .position;
    [at[0], at[1]]
}

fn alpha(run: &Run, id: &str) -> f64 {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    run.world.get(entity).expect("it exists").components["sindri.sprite"]["tint"][3]
        .as_f64()
        .expect("an alpha")
}

fn scale_x(run: &Run, id: &str) -> f32 {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    run.world
        .get(entity)
        .expect("it exists")
        .transform_3d
        .expect("it has a place")
        .scale[0]
}

/// How far Wick's lantern reaches, which a scorch shrinks.
fn lantern_radius(run: &Run) -> f64 {
    let light = run.entity("wick-light").expect("Wick carries a light");
    run.world.get(light).expect("it exists").components["sindri.light2d"]["radius"]
        .as_f64()
        .expect("a radius")
}

/// Wick's first step, after which the gloom hunt; he is back where he started.
fn first_step(run: &mut Run) {
    run.input.apply(InputEvent::KeyPressed(Key::D));
    step(run, 3);
    run.input.apply(InputEvent::KeyReleased(Key::D));
    run.input.apply(InputEvent::KeyPressed(Key::A));
    step(run, 3);
    run.input.apply(InputEvent::KeyReleased(Key::A));
    step(run, 20);
}

/// Holds the walk key for `frames` frames.
fn walk_east(run: &mut Run, frames: u32) {
    run.input.apply(InputEvent::KeyPressed(Key::D));
    step(run, frames);
    run.input.apply(InputEvent::KeyReleased(Key::D));
}

#[test]
fn a_gloom_creeps_through_the_dark_toward_wick() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    first_step(&mut run);
    let start = position(&run, "gloom-1")[0];
    step(&mut run, 60);
    let early = position(&run, "gloom-1")[0];
    assert!(
        early < start - 0.3,
        "it creeps in the dark: {start} to {early}"
    );
}

#[test]
fn a_gloom_winds_up_at_the_edge_of_the_light_and_a_pounce_scorches_the_lantern() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    first_step(&mut run);
    let (mut crouched, mut scorched) = (false, false);
    for _ in 0..600 {
        step(&mut run, 1);
        // The wind-up is low and wide: its scale across goes past the breathing.
        crouched |= scale_x(&run, "gloom-1") > 2.3;
        scorched |= lantern_radius(&run) < 7.5;
    }
    assert!(crouched, "it crouches before it pounces");
    assert!(scorched, "and a pounce that lands shrinks Wick's light");
}

#[test]
fn a_lantern_that_is_put_out_starts_the_field_over() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    run.set_board("lost", 1.0);
    step(&mut run, 20);
    assert!(lantern_radius(&run) < 3.0, "out, the lantern is a spark");
    // After a moment the field is rebuilt, and nothing is lost in it.
    let mut again = false;
    for _ in 0..300 {
        step(&mut run, 1);
        if run.board("lost") < 0.5 {
            again = true;
            break;
        }
    }
    assert!(again, "the field starts again");
}

#[test]
fn a_gloom_far_off_does_not_notice_wick() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    let far = |run: &Run| {
        let (gloom, wick) = (position(run, "gloom-3"), position(run, "wick"));
        (gloom[0] - wick[0]).hypot(gloom[1] - wick[1])
    };
    let start = far(&run);
    step(&mut run, 120);
    assert!(
        far(&run) > start - 1.0,
        "it lurks where it is and does not stalk: {start} to {}",
        far(&run)
    );
}

#[test]
fn a_gloom_in_a_strong_light_thins() {
    let mut run = Run::open().expect("the project opens");
    // It waits where it is, in front of the lantern's beam, rather than pouncing
    // past him into the dark behind.
    run.set_board("truce", 1.0);
    step(&mut run, 5);
    // Wick walks up to it, so the lantern is on it.
    for _ in 0..40 {
        if position(&run, "gloom-1")[0] - position(&run, "wick")[0] < 2.0 {
            break;
        }
        walk_east(&mut run, 6);
    }
    let mut thinnest: f64 = 1.0;
    for _ in 0..200 {
        step(&mut run, 1);
        thinnest = thinnest.min(alpha(&run, "gloom-1"));
    }
    assert!(thinnest < 0.7, "lit by the lantern it thins: {thinnest}");
}

#[test]
fn the_field_opens_calm_until_wick_first_moves() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    step(&mut run, 900);
    assert!(
        lantern_radius(&run) > 7.99,
        "left standing at the start, nothing has hunted him: {}",
        lantern_radius(&run)
    );
}

/// Where the lantern's beam points, in degrees.
fn beam_direction(run: &Run) -> f64 {
    let light = run.entity("wick-light").expect("Wick carries a light");
    run.world.get(light).expect("it exists").components["sindri.light2d"]["cone_direction"]
        .as_f64()
        .expect("a direction")
}

#[test]
fn the_lanterns_beam_turns_to_the_way_wick_faces() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    let facing = |run: &mut Run, key: Key| {
        run.input.apply(InputEvent::KeyPressed(key));
        step(run, 30);
        run.input.apply(InputEvent::KeyReleased(key));
        step(run, 10);
        beam_direction(run)
    };
    assert!((beam_direction(&run) + 90.0).abs() < 1.0, "south at first");
    assert!(facing(&mut run, Key::D).abs() < 3.0, "east is 0 degrees");
    assert!((facing(&mut run, Key::W) - 90.0).abs() < 3.0, "north is 90");
    let west = facing(&mut run, Key::A);
    assert!((west.abs() - 180.0).abs() < 3.0, "west is 180: {west}");
}

/// Puts an entity at a place, and stops it.
fn place(run: &mut Run, id: &str, at: [f32; 2]) {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    let data = run.world.get_mut(entity).expect("it exists");
    let transform = data.transform_3d.as_mut().expect("it has a place");
    transform.position[0] = at[0];
    transform.position[1] = at[1];
}

/// Turns Wick to face a way, and puts him back at `at`.
fn face_and_stand(run: &mut Run, key: Key, at: [f32; 2]) {
    run.input.apply(InputEvent::KeyPressed(key));
    step(run, 2);
    run.input.apply(InputEvent::KeyReleased(key));
    place(run, "wick", at);
    step(run, 20);
}

fn gone(run: &Run, id: &str) -> bool {
    run.entity(id).is_none()
}

#[test]
fn a_gloom_in_the_beam_stops_and_burns_out() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    face_and_stand(&mut run, Key::D, [0.0, 0.0]);
    place(&mut run, "gloom-1", [3.2, 0.0]);
    step(&mut run, 40);
    let held = position(&run, "gloom-1");
    step(&mut run, 20);
    let later = position(&run, "gloom-1");
    assert!(
        (later[0] - held[0]).abs() < 0.1 && (later[1] - held[1]).abs() < 0.1,
        "the light holds it where it is: {held:?} then {later:?}"
    );
    assert!(!gone(&run, "gloom-1"), "not yet out");
    step(&mut run, 150);
    assert!(
        gone(&run, "gloom-1"),
        "kept in the light, it burns out and is gone"
    );
}

#[test]
fn a_gloom_the_light_has_turned_from_is_not_burned() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    face_and_stand(&mut run, Key::A, [0.0, 0.0]);
    place(&mut run, "gloom-1", [3.2, 0.0]);
    step(&mut run, 150);
    assert!(
        !gone(&run, "gloom-1") || run.board("gloom_out") < 0.5,
        "with the beam pointed away it is not burned"
    );
}

#[test]
fn cover_makes_a_gloom_bold_where_an_exposed_one_holds_back() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    // Wick at the west of a boulder, looking east, so its far side is lit and
    // out of his sight. One gloom is behind it, one is out in the open.
    run.set_board("truce", 1.0);
    face_and_stand(&mut run, Key::D, [1.0, 3.0]);
    place(&mut run, "gloom-2", [7.2, 3.0]);
    place(&mut run, "gloom-1", [7.2, 1.0]);
    step(&mut run, 10);
    // They hold still until the truce ends, so both start from the same footing.
    run.set_board("truce", 0.0);
    step(&mut run, 2);
    let (hidden, open) = (position(&run, "gloom-2"), position(&run, "gloom-1"));
    // Short enough that the one in the open is still winding up.
    step(&mut run, 50);
    let (hidden_now, open_now) = (position(&run, "gloom-2"), position(&run, "gloom-1"));
    let gained = |from: [f32; 2], to: [f32; 2]| {
        ((from[0] - 1.0).hypot(from[1] - 3.0)) - ((to[0] - 1.0).hypot(to[1] - 3.0))
    };
    assert!(
        gained(hidden, hidden_now) > 0.4,
        "behind cover it creeps on into the light: {hidden:?} to {hidden_now:?}"
    );
    assert!(
        gained(open, open_now) < 0.2,
        "in the open and lit it holds back: {open:?} to {open_now:?}"
    );
}
