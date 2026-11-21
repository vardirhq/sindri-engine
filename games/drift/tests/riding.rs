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
