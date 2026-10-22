//! Playing Causeway the way a person does: the title, a click on a warden,
//! a click on the ground, a key to end the turn -- and the board answering.
//!
//! Every click goes the whole way round. A cell's top is projected through
//! the camera the frame is drawn with, the pointer is put on that pixel, and
//! the game has to turn the pointer's ray back into the cell. A test that
//! called the rules directly would prove the rules, and the autoplay test
//! already does that; this proves a person can reach them.

// The board holds whole numbers the scripts wrote, so they compare exactly.
#![allow(clippy::float_cmp)]

mod rotation;
mod support;

use sindri_platform::Key;
use support::*;

#[test]
fn the_title_begins_a_crossing_and_the_hollow_climb_out() {
    let mut game = Game::open();
    assert_eq!(game.phase(), TITLE, "the game opens on its title");
    game.tap_key(Key::Enter);
    game.until_players_turn();
    assert_eq!(
        game.board("island"),
        0.0,
        "the crossing starts on the first island"
    );
    assert_eq!(game.board("round"), 1.0);
    assert_eq!(
        game.pieces("bulwark").len(),
        1,
        "the wardens stand on the island"
    );
    assert_eq!(game.pieces("mason").len(), 1);
    assert_eq!(game.pieces("lamplighter").len(), 1);
    assert!(
        game.pieces("husk").len() >= 2,
        "the first Hollow have climbed out"
    );
}

#[test]
fn clicking_a_warden_and_then_the_ground_walks_it_there() {
    let mut game = Game::open();
    game.tap_key(Key::Enter);
    game.until_players_turn();
    let start = game.cell_of(game.pieces("bulwark")[0]);
    // Wherever the Hollow happened to climb out, a free neighbour within a
    // step of the Bulwark's height is somewhere it can walk.
    let goal = game.free_step_from(start);
    game.click_cell(start);
    game.click_cell(goal);
    game.until_players_turn();
    assert_eq!(
        game.cell_of(game.pieces("bulwark")[0]),
        goal,
        "the click on the ground was turned back into that cell, and the warden walked to it"
    );
}

#[test]
fn ending_the_turn_lets_the_hollow_strike_and_the_next_round_begin() {
    let mut game = Game::open();
    game.tap_key(Key::Enter);
    game.until_players_turn();
    game.tap_key(Key::E);
    game.until_players_turn();
    assert_eq!(game.board("round"), 2.0, "the round moved on");
    assert!(
        game.board("hearth") <= 7.0 && game.board("hearth") > 0.0,
        "the Hearth is still burning"
    );
}

#[test]
fn the_keyboard_alone_can_move_a_warden() {
    let mut game = Game::open();
    game.tap_key(Key::Enter);
    game.until_players_turn();
    // The first warden is taken up and the cursor stands on it. Enter on a
    // reachable cell walks it there.
    let start = game.cell_of(game.pieces("bulwark")[0]);
    game.tap_key(Key::ArrowLeft);
    game.tap_key(Key::Enter);
    game.until_players_turn();
    assert_eq!(
        game.cell_of(game.pieces("bulwark")[0]),
        (start.0 - 1, start.1)
    );
}

#[test]
fn a_phone_held_upright_plays_by_tapping() {
    let mut game = Game::open_at(PHONE);
    game.tap_key(Key::Enter);
    game.until_players_turn();
    let start = game.cell_of(game.pieces("bulwark")[0]);
    let goal = game.free_step_from(start);
    game.tap_cell(start);
    game.tap_cell(goal);
    game.until_players_turn();
    assert_eq!(
        game.cell_of(game.pieces("bulwark")[0]),
        goal,
        "two taps took up the warden and walked it, with no mouse and no keys"
    );
}
