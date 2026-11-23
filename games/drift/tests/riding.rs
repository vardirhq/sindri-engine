//! Riding a moving ship: aboard a block-built ship that flies and turns, the
//! diver stands on its deck, falls towards its floor and walks along it,
//! carried with it, measured in the ship's own blocks.

use drift::Run;
use glam::{Quat, Vec3};
use serde_json::json;
use sindri_platform::Key;

/// Where a world point is in the shuttle's own blocks.
fn in_shuttle(game: &Run, point: [f32; 3]) -> Vec3 {
    let shuttle = game.run.entity("shuttle").expect("the shuttle");
    let placed = game.run.world.world_transform(shuttle).expect("placed");
    let rotation = Quat::from_array(placed.rotation).normalize();
    rotation.inverse() * (Vec3::from_array(point) - Vec3::from_array(placed.position))
        / Vec3::from_array(placed.scale)
}

/// The shuttle set flying and turning, and the diver standing on its deck
/// amidships, facing its bow.
fn aboard_a_flying_shuttle() -> Run {
    let shuttle_at = [13.0, -2.0, -17.0];
    // Amidships, a capsule's half-height above the deck's top (block y = 1).
    let diver = [
        shuttle_at[0] + 6.0,
        shuttle_at[1] + 0.5 + 0.8,
        shuttle_at[2] + 2.5,
    ];
    let mut game = Run::open_at(diver).expect("DRIFT opens");
    let shuttle = game.run.entity("shuttle").expect("the shuttle");
    let body = game
        .run
        .world
        .get_mut(shuttle)
        .unwrap()
        .components
        .get_mut("sindri.physics3d.rigid_body")
        .unwrap();
    body["linear_velocity"] = json!([1.5, 0.0, 0.5]);
    body["angular_velocity"] = json!([0.0, 0.15, 0.0]);
    game
}

#[test]
fn standing_aboard_a_flying_turning_ship_the_diver_stays_on_its_deck() {
    let mut game = aboard_a_flying_shuttle();
    game.wait(0.5);
    assert!(
        game.has_said("Drift: awake in the cabin") || game.has_said("Drift: floating in the hold")
    );
    let start = in_shuttle(&game, game.position("diver"));
    let ship_start = game.position("shuttle");
    game.wait(3.0);
    let ship_now = game.position("shuttle");
    let moved = Vec3::from_array(ship_now).distance(Vec3::from_array(ship_start));
    assert!(moved > 4.0, "the ship flew: {moved} m");
    let now = in_shuttle(&game, game.position("diver"));
    assert!(
        (now.y - start.y).abs() < 0.1,
        "on the deck all along: {start:?} {now:?}"
    );
    assert!(
        (now.x - start.x).abs() < 0.4 && (now.z - start.z).abs() < 0.4,
        "carried with the ship, not left behind: {start:?} {now:?}"
    );
}

#[test]
fn walking_aboard_a_turning_ship_moves_along_its_deck() {
    let mut game = aboard_a_flying_shuttle();
    game.wait(0.5);
    let start = in_shuttle(&game, game.position("diver"));
    // The diver faces world +X, the shuttle's stern; walk that way.
    game.press(Key::W);
    game.wait(1.0);
    game.release(Key::W);
    game.wait(0.5);
    let now = in_shuttle(&game, game.position("diver"));
    assert!(
        now.x > start.x + 3.0,
        "walked several blocks along the deck: {start:?} {now:?}"
    );
    assert!((now.y - start.y).abs() < 0.1, "still on the deck: {now:?}");
    assert!(
        (now.z - start.z).abs() < 0.6,
        "straight along it: {start:?} {now:?}"
    );
}

#[test]
fn eye_stays_above_the_diver_in_a_rolling_ship() {
    let mut game = aboard_a_flying_shuttle();
    let shuttle = game.run.entity("shuttle").expect("the shuttle");
    let body = game
        .run
        .world
        .get_mut(shuttle)
        .unwrap()
        .components
        .get_mut("sindri.physics3d.rigid_body")
        .unwrap();
    body["angular_velocity"] = json!([0.65, 0.0, 0.0]);
    game.wait(1.5);

    let transform = game
        .run
        .world
        .world_transform(shuttle)
        .expect("ship transform");
    let rotation = Quat::from_array(transform.rotation).normalize();
    assert!(
        (rotation * Vec3::Y).dot(Vec3::Y) < 0.8,
        "the ship actually rolled"
    );
    let diver = in_shuttle(&game, game.position("diver"));
    let eye = in_shuttle(&game, game.position("eye"));
    let offset = eye - diver;
    assert!(offset.y > 1.0, "eye stays above the diver: {offset:?}");
    assert!(
        offset.x.abs() < 0.2 && offset.z.abs() < 0.2,
        "eye follows ship up: {offset:?}"
    );
}

#[test]
fn diver_remains_on_the_deck_when_ship_rolls() {
    let mut game = aboard_a_flying_shuttle();
    let shuttle = game.run.entity("shuttle").expect("the shuttle");
    let body = game
        .run
        .world
        .get_mut(shuttle)
        .unwrap()
        .components
        .get_mut("sindri.physics3d.rigid_body")
        .unwrap();
    body["angular_velocity"] = json!([0.65, 0.0, 0.0]);
    game.wait(0.2);
    let start = in_shuttle(&game, game.position("diver"));
    game.wait(1.3);
    let now = in_shuttle(&game, game.position("diver"));
    assert!(
        (now.y - start.y).abs() < 0.3,
        "diver maintains deck height: {start:?} -> {now:?}"
    );
    assert!(
        (now.x - start.x).abs() < 0.5 && (now.z - start.z).abs() < 0.5,
        "diver stays in place: {start:?} -> {now:?}"
    );
}

#[test]
fn diver_stays_on_deck_through_a_long_roll() {
    let mut game = aboard_a_flying_shuttle();
    let shuttle = game.run.entity("shuttle").expect("the shuttle");
    let body = game
        .run
        .world
        .get_mut(shuttle)
        .unwrap()
        .components
        .get_mut("sindri.physics3d.rigid_body")
        .unwrap();
    body["angular_velocity"] = json!([0.65, 0.0, 0.0]);
    game.wait(0.2);
    let start = in_shuttle(&game, game.position("diver"));
    let mut turn = 0.0;
    let mut last =
        Quat::from_array(game.run.world.world_transform(shuttle).unwrap().rotation).normalize();
    for _ in 0..600 {
        game.step();
        let next =
            Quat::from_array(game.run.world.world_transform(shuttle).unwrap().rotation).normalize();
        turn += 2.0 * last.dot(next).abs().clamp(0.0, 1.0).acos();
        last = next;
    }
    assert!(turn > 5.5, "the ship makes almost a full turn: {turn}");
    let now = in_shuttle(&game, game.position("diver"));
    assert!(
        (now - start).length() < 0.5,
        "diver remains aboard: {start:?} -> {now:?}"
    );
}

#[test]
fn diver_stays_on_deck_through_a_long_pitch() {
    let mut game = aboard_a_flying_shuttle();
    let shuttle = game.run.entity("shuttle").expect("the shuttle");
    let body = game
        .run
        .world
        .get_mut(shuttle)
        .unwrap()
        .components
        .get_mut("sindri.physics3d.rigid_body")
        .unwrap();
    body["angular_velocity"] = json!([0.0, 0.0, 0.65]);
    game.wait(0.2);
    let start = in_shuttle(&game, game.position("diver"));
    game.wait(3.0);
    let now = in_shuttle(&game, game.position("diver"));
    assert!(
        (now - start).length() < 0.5,
        "diver remains aboard: {start:?} -> {now:?}"
    );
}

#[test]
fn leaving_a_turning_ship_keeps_world_velocity() {
    let mut game = aboard_a_flying_shuttle();
    game.wait(0.5);
    let diver = game.run.entity("diver").expect("the diver");
    let shuttle = game.run.entity("shuttle").expect("the shuttle");
    let transform = game
        .run
        .world
        .world_transform(shuttle)
        .expect("ship transform");
    let rotation = Quat::from_array(transform.rotation).normalize();
    let edge = Vec3::from_array(transform.position)
        + rotation * (Vec3::new(22.0, 2.6, 5.0) * Vec3::from_array(transform.scale));
    game.place("diver", edge.to_array());
    game.wait(0.2);
    game.press(Key::W);
    let mut previous = Vec3::ZERO;
    let mut exited = false;
    for _ in 0..180 {
        previous = Vec3::from_array(
            game.run
                .session
                .physics3d()
                .world()
                .linear_velocity(diver)
                .unwrap(),
        );
        game.step();
        if game.has_said("Drift: weightless") {
            exited = true;
            break;
        }
    }
    assert!(exited, "the diver left the ship: {:?}", game.said);
    let after = Vec3::from_array(
        game.run
            .session
            .physics3d()
            .world()
            .linear_velocity(diver)
            .unwrap(),
    );
    assert!(
        after.distance(previous) < 0.5,
        "velocity on exit: {previous:?} -> {after:?}"
    );
}

#[test]
fn walking_and_jumping_aboard_a_rolling_ship() {
    let mut game = aboard_a_flying_shuttle();
    let shuttle = game.run.entity("shuttle").expect("the shuttle");
    let body = game
        .run
        .world
        .get_mut(shuttle)
        .unwrap()
        .components
        .get_mut("sindri.physics3d.rigid_body")
        .unwrap();
    body["angular_velocity"] = json!([0.4, 0.0, 0.0]);
    game.wait(0.5);
    let start = in_shuttle(&game, game.position("diver"));
    game.press(Key::W);
    game.wait(0.8);
    game.release(Key::W);
    game.wait(0.3);
    let walked = in_shuttle(&game, game.position("diver"));
    assert!(
        (walked - start).length() > 2.0,
        "walked: {start:?} -> {walked:?}"
    );
    assert!(
        (walked.y - start.y).abs() < 0.3,
        "still standing: {start:?} -> {walked:?}"
    );
    game.tap(Key::Space);
    game.wait(0.2);
    let jumped = in_shuttle(&game, game.position("diver"));
    assert!(
        jumped.y > walked.y + 0.3,
        "jump rises from deck: {walked:?} -> {jumped:?}"
    );
}

#[test]
fn wall_stops_diver_aboard_a_rolling_ship() {
    let mut game = aboard_a_flying_shuttle();
    let shuttle = game.run.entity("shuttle").expect("the shuttle");
    let body = game
        .run
        .world
        .get_mut(shuttle)
        .unwrap()
        .components
        .get_mut("sindri.physics3d.rigid_body")
        .unwrap();
    body["angular_velocity"] = json!([0.3, 0.0, 0.0]);
    game.wait(0.5);
    let start = in_shuttle(&game, game.position("diver"));
    game.press(Key::D);
    game.wait(3.0);
    game.release(Key::D);
    game.wait(0.2);
    let now = in_shuttle(&game, game.position("diver"));
    assert!(
        (now.z - start.z).abs() > 1.0,
        "diver walked toward the wall: {start:?} -> {now:?}"
    );
    assert!(
        now.z > 1.3 && now.z < 8.7,
        "diver remains inside walls: {now:?}"
    );
    assert!(
        now.y > 2.0 && now.y < 4.0,
        "diver remains near deck: {now:?}"
    );
}

#[test]
fn standing_under_ship_gravity_does_not_accelerate_the_ship() {
    let mut game = aboard_a_flying_shuttle();
    let shuttle = game.run.entity("shuttle").expect("the shuttle");
    let body = game
        .run
        .world
        .get_mut(shuttle)
        .unwrap()
        .components
        .get_mut("sindri.physics3d.rigid_body")
        .unwrap();
    body["linear_velocity"] = json!([0.0, 0.0, 0.0]);
    body["angular_velocity"] = json!([0.0, 0.0, 0.0]);
    game.wait(1.0);
    let before = Vec3::from_array(
        game.run
            .session
            .physics3d()
            .world()
            .linear_velocity(shuttle)
            .unwrap(),
    );
    game.wait(5.0);
    let after = Vec3::from_array(
        game.run
            .session
            .physics3d()
            .world()
            .linear_velocity(shuttle)
            .unwrap(),
    );
    assert!(
        (after - before).length() < 0.01,
        "idle passenger accelerated ship: {before:?} -> {after:?}"
    );
}

#[test]
fn passenger_settles_onto_the_deck_from_just_above_it() {
    let mut game = Run::open_at([19.0, -0.65, -14.5]).expect("DRIFT opens");
    let shuttle = game.run.entity("shuttle").expect("the shuttle");
    let body = game
        .run
        .world
        .get_mut(shuttle)
        .unwrap()
        .components
        .get_mut("sindri.physics3d.rigid_body")
        .unwrap();
    body["linear_velocity"] = json!([0.0, 0.0, 0.0]);
    body["angular_velocity"] = json!([0.0, 0.0, 0.0]);
    game.wait(2.0);
    let position = in_shuttle(&game, game.position("diver"));
    assert!(
        position.y < 2.64,
        "passenger floats above the deck: {position:?}"
    );
}
