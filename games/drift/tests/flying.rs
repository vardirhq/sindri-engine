//! Flying the block-built shuttle: seated at its controls, the keys fire the
//! thrusters that push the way asked, each through where it is mounted, and a
//! gyroscope turns and steadies the ship.

use drift::Run;
use glam::{Quat, Vec3};
use sindri_platform::Key;

const SHUTTLE: [f32; 3] = [13.0, -2.0, -17.0];
const SEAT_BLOCKS: [f32; 3] = [2.8, 1.0, 5.0];

/// Opens with the diver beside the shuttle's seat, and sits down.
fn seated(without: &[&str]) -> Run {
    let seat = Vec3::from_array(SHUTTLE) + Vec3::from_array(SEAT_BLOCKS) * 0.5;
    let mut game = Run::open_at((seat + Vec3::new(0.6, 0.8, 0.0)).to_array()).expect("DRIFT opens");
    for id in without {
        let entity = game.run.entity(id).expect("the part exists");
        game.run
            .world
            .despawn_recursive(entity)
            .expect("the part is removed");
    }
    game.wait(0.3);
    game.tap(Key::F);
    assert!(game.has_said("Drift: at the controls"), "{:?}", game.said);
    game
}

fn shuttle_velocity(game: &Run) -> Vec3 {
    let shuttle = game.run.entity("shuttle").unwrap();
    Vec3::from_array(
        game.run
            .session
            .physics3d()
            .world()
            .linear_velocity(shuttle)
            .unwrap(),
    )
}

fn shuttle_spin(game: &Run) -> Vec3 {
    let shuttle = game.run.entity("shuttle").unwrap();
    Vec3::from_array(
        game.run
            .session
            .physics3d()
            .world()
            .angular_velocity(shuttle)
            .unwrap(),
    )
}

/// The shuttle's own X axis, its nose being -X, in the world.
fn shuttle_axis_x(game: &Run) -> Vec3 {
    let shuttle = game.run.entity("shuttle").unwrap();
    let placed = game.run.world.world_transform(shuttle).unwrap();
    Quat::from_array(placed.rotation).normalize() * Vec3::X
}

fn active(game: &Run, id: &str) -> bool {
    let entity = game.run.entity(id).unwrap();
    game.run.world.is_active(entity)
}

#[test]
fn the_main_engines_push_the_ship_nose_first_and_the_pilot_rides_in_the_seat() {
    let mut game = seated(&[]);
    let seat_start = game.position("shuttle-seat");
    let diver_offset = Vec3::from_array(game.position("diver")) - Vec3::from_array(seat_start);
    game.press(Key::W);
    game.wait(0.2);
    assert!(
        active(&game, "shuttle-flame-0") && active(&game, "shuttle-flame-1"),
        "the mains burn"
    );
    assert!(!active(&game, "shuttle-flame-2"), "the retros do not");
    game.wait(1.8);
    game.release(Key::W);
    let along = shuttle_velocity(&game).dot(-shuttle_axis_x(&game));
    assert!(along > 1.0, "the shuttle flies nose first: {along} m/s");
    game.step();
    assert!(
        !active(&game, "shuttle-flame-0"),
        "the flames go out when the keys do"
    );
    let seat_now = Vec3::from_array(game.position("shuttle-seat"));
    assert!(
        seat_now.distance(Vec3::from_array(seat_start)) > 1.0,
        "the ship moved"
    );
    let offset = Vec3::from_array(game.position("diver")) - seat_now;
    assert!(
        offset.distance(diver_offset) < 0.5,
        "the pilot stays in the seat: {diver_offset} then {offset}"
    );
}

#[test]
fn losing_a_main_engine_turns_the_ship_when_it_burns() {
    let mut whole = seated(&[]);
    whole.press(Key::W);
    whole.wait(1.5);
    let balanced = shuttle_spin(&whole).length();
    let mut damaged = seated(&["shuttle-thruster-0"]);
    damaged.press(Key::W);
    damaged.wait(1.5);
    let lopsided = shuttle_spin(&damaged).length();
    // The gyroscope fights it, so the turn settles where the engine's torque
    // and the gyroscope's hold balance: small, but plainly there.
    assert!(
        lopsided > 0.02 && lopsided > balanced * 1.8,
        "one engine off centre turns the ship: {balanced} then {lopsided} rad/s"
    );
}

#[test]
fn the_gyroscope_rolls_the_ship_and_then_holds_it_steady() {
    let mut game = seated(&[]);
    game.press(Key::E);
    game.wait(1.5);
    let rolling = shuttle_spin(&game);
    let about_nose = rolling.dot(shuttle_axis_x(&game)).abs();
    assert!(about_nose > 0.4, "rolling about the nose: {rolling}");
    game.release(Key::E);
    game.wait(2.0);
    let steady = shuttle_spin(&game).length();
    assert!(
        steady < 0.05,
        "the gyroscope stops the roll: {steady} rad/s"
    );
    game.tap(Key::F);
    assert!(game.has_said("Drift: left the controls"), "{:?}", game.said);
}

#[test]
fn standing_up_in_flight_keeps_the_ships_speed() {
    let mut game = seated(&[]);
    game.press(Key::W);
    game.wait(2.0);
    game.release(Key::W);
    game.wait(0.5);
    let ship = shuttle_velocity(&game);
    let diver = game.run.entity("diver").unwrap();
    let riding = Vec3::from_array(
        game.run
            .session
            .physics3d()
            .world()
            .linear_velocity(diver)
            .unwrap(),
    );
    assert!(
        riding.distance(ship) < 0.3,
        "seated, the pilot moves with the ship: {riding} vs {ship}"
    );
    game.tap(Key::F);
    game.wait(0.5);
    let ship = shuttle_velocity(&game);
    let walking = Vec3::from_array(
        game.run
            .session
            .physics3d()
            .world()
            .linear_velocity(diver)
            .unwrap(),
    );
    assert!(
        walking.distance(ship) < 0.5,
        "standing up, the pilot is still carried: {walking} vs {ship}"
    );
}
