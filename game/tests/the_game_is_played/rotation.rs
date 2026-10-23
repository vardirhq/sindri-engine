use super::support::*;
use glam::Vec3;
use sindri_platform::{InputEvent, Key, MouseButton};

fn pose(game: &Game) -> sindri_core::Transform3D {
    game.run
        .world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("World Camera"))
        .and_then(|(_, data)| data.transform_3d)
        .unwrap()
}

#[test]
fn four_smooth_turns_preserve_pitch_focus_and_zoom() {
    let mut game = Game::open();
    game.begin_crossing();
    game.play(1.0);
    let home = pose(&game);
    let lens = game
        .run
        .world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("World Camera"))
        .unwrap()
        .1
        .components["sindri.camera"]
        .clone();
    let pivot = Vec3::new(28.0, 1.4, 28.0);
    let radius = (Vec3::from(home.position) - pivot).length();
    for _ in 0..4 {
        game.tap_key(Key::L);
        game.play(0.12);
        let middle = pose(&game);
        assert!((Vec3::from(middle.position) - Vec3::from(home.position)).length() > 0.1);
        game.play(0.3);
        let turned = pose(&game);
        assert!(((Vec3::from(turned.position) - pivot).length() - radius).abs() < 0.002);
        assert!((turned.position[1] - home.position[1]).abs() < 0.002);
        assert!((turned.forward()[1] - home.forward()[1]).abs() < 0.002);
        let focus = Vec3::from(turned.position) + Vec3::from(turned.forward()) * radius;
        assert!(focus.abs_diff_eq(pivot, 0.005));
    }
    assert!(Vec3::from(pose(&game).position).abs_diff_eq(Vec3::from(home.position), 0.002));
    let final_lens = &game
        .run
        .world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("World Camera"))
        .unwrap()
        .1
        .components["sindri.camera"];
    assert_eq!(&lens, final_lens);
}

#[test]
fn picking_movement_and_combat_work_in_every_view() {
    for turns in 0..4 {
        let mut game = Game::open();
        game.begin_crossing();
        game.play(1.0);
        for _ in 0..turns {
            game.tap_key(Key::L);
            game.play(0.4);
        }
        let start = game.cell_of(game.pieces("bulwark")[0]);
        // Aim at the figure's body, not the floor below it: a foreground
        // figure may legitimately cover that floor pixel after a turn.
        let [x, y] = game.pixel_of(start, top_of(start) + 0.6);
        game.run.input.apply(InputEvent::PointerMoved { x, y });
        game.step();
        #[allow(clippy::cast_precision_loss)]
        let expected = (start.1 * 8 + start.0) as f32;
        assert_eq!(
            game.board("pointed"),
            expected,
            "body picked in view {turns}"
        );
        game.run
            .input
            .apply(InputEvent::ButtonPressed(MouseButton::Left));
        game.step();
        game.run
            .input
            .apply(InputEvent::ButtonReleased(MouseButton::Left));
        game.step();
        game.step();
        // Use keyboard confirmation for an unoccupied neighboring ground cell;
        // body picking has priority when a silhouette overlaps a ground pixel.
        let key = [
            Key::ArrowLeft,
            Key::ArrowUp,
            Key::ArrowRight,
            Key::ArrowDown,
        ][turns];
        game.tap_key(key);
        game.tap_key(Key::Enter);
        game.until_players_turn();
        assert_eq!(
            game.cell_of(game.pieces("bulwark")[0]),
            (start.0 - 1, start.1)
        );
        game.tap_key(Key::E);
        game.until_players_turn();
        assert_eq!(game.board("round"), 2.0);
    }
}

fn swipe(game: &mut Game, from: [f32; 2], to: [f32; 2]) {
    game.run.input.apply(InputEvent::TouchStarted {
        id: 1,
        x: from[0],
        y: from[1],
    });
    game.step();
    game.run.input.apply(InputEvent::TouchMoved {
        id: 1,
        x: to[0],
        y: to[1],
    });
    game.step();
    game.run.input.apply(InputEvent::TouchEnded { id: 1 });
    game.step();
    game.play(0.5);
}

#[test]
fn phone_swipes_turn_only_when_they_begin_in_empty_sea() {
    let mut game = Game::open_at(PHONE);
    game.begin_crossing();
    game.play(1.0);
    let home = pose(&game);
    let start = game.cell_of(game.pieces("bulwark")[0]);
    let point = game.pixel_of(start, top_of(start));
    swipe(&mut game, point, [point[0] + 70.0, point[1]]);
    assert!(Vec3::from(pose(&game).position).abs_diff_eq(Vec3::from(home.position), 0.002));
    // A prior mouse hover must not mask touch input on hybrid devices.
    game.run
        .input
        .apply(InputEvent::PointerMoved { x: 359.0, y: 102.0 });
    game.step();
    swipe(&mut game, [80.0, 180.0], [180.0, 180.0]);
    assert!(!Vec3::from(pose(&game).position).abs_diff_eq(Vec3::from(home.position), 0.1));
    assert_eq!(game.cell_of(game.pieces("bulwark")[0]), start);
}

#[test]
fn phone_button_rotates_and_tapping_still_moves() {
    let mut game = Game::open_at(PHONE);
    game.begin_crossing();
    game.play(1.0);
    let home = pose(&game);
    game.run.input.apply(InputEvent::TouchStarted {
        id: 1,
        x: 309.0,
        y: 102.0,
    });
    game.step();
    game.run.input.apply(InputEvent::TouchEnded { id: 1 });
    game.step();
    game.play(0.5);
    assert!(
        !Vec3::from(pose(&game).position).abs_diff_eq(Vec3::from(home.position), 0.1),
        "touch button turns the view"
    );
    let start = game.cell_of(game.pieces("bulwark")[0]);
    let goal = game.free_step_from(start);
    let [x, y] = game.pixel_of(start, top_of(start) + 0.6);
    game.run
        .input
        .apply(InputEvent::TouchStarted { id: 1, x, y });
    game.step();
    game.run.input.apply(InputEvent::TouchEnded { id: 1 });
    game.step();
    game.step();
    game.tap_cell(goal);
    game.until_players_turn();
    assert_eq!(game.cell_of(game.pieces("bulwark")[0]), goal);
}

#[test]
fn turning_does_not_end_a_turn_or_consume_a_held_touch() {
    let mut game = Game::open_at(PHONE);
    game.begin_crossing();
    game.play(1.0);
    let start = game.cell_of(game.pieces("bulwark")[0]);
    game.tap_key(Key::J);
    game.tap_key(Key::E);
    game.play(0.15);
    let goal = game.free_step_from(start);
    let point = game.pixel_of(goal, top_of(goal));
    game.run.input.apply(InputEvent::TouchStarted {
        id: 1,
        x: point[0],
        y: point[1],
    });
    game.step();
    game.play(0.2);
    game.run.input.apply(InputEvent::TouchEnded { id: 1 });
    game.step();
    game.play(0.5);
    assert_eq!(game.board("round"), 1.0);
    assert_eq!(game.cell_of(game.pieces("bulwark")[0]), start);
}
