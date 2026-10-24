//! The shell around the levels: the title, the level select, the pause screen
//! and the settings, each its own scene, played by a test as a person plays
//! them: by pressing the buttons that are drawn.
//!
//! What it holds is the route between them and what a person keeps: leaving a
//! level to pause finds it as it was, starting it from a menu begins it over,
//! reaching the beacon saves it, and a volume survives leaving the screen.

use lamplighter::Run;
use sindri_core::EntityId;
use sindri_decay::AudioCommand;
use sindri_platform::{InputEvent, Key, MouseButton};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

/// An entity of the scene being played. The first scene a run opens keeps the
/// ids its file spells; every scene entered afterwards by `Scene.go` carries
/// its own name in front of them, so a screen's `play` is `title.scene/play`.
fn find(run: &Run, id: &str) -> Option<EntityId> {
    run.entity(&format!("{}/{id}", scene(run)))
        .or_else(|| run.entity(id))
}

fn scene(run: &Run) -> String {
    run.session.scene().expect("a scene is playing").to_owned()
}

/// Presses the middle of a drawn element and lets go, as a finger would, then
/// lets the scene change that asks for happen.
fn press(run: &mut Run, name: &str) {
    let entity = find(run, name).unwrap_or_else(|| panic!("{name} is in {}", scene(run)));
    let [x, y] = run
        .on_screen(entity)
        .unwrap_or_else(|| panic!("{name} is drawn on {}", scene(run)));
    run.input.apply(InputEvent::PointerMoved { x, y });
    step(run, 1);
    run.input
        .apply(InputEvent::ButtonPressed(MouseButton::Left));
    step(run, 2);
    run.input
        .apply(InputEvent::ButtonReleased(MouseButton::Left));
    step(run, 2);
}

fn flag(run: &Run, key: &str) -> bool {
    run.session.saves().flag(key, false)
}

fn number(run: &Run, key: &str, fallback: f64) -> f64 {
    run.session.saves().number(key, fallback)
}

fn text(run: &Run, name: &str) -> String {
    let entity = find(run, name).unwrap_or_else(|| panic!("{name}"));
    run.world.get(entity).expect("it exists").components["sindri.ui.text"]["text"]
        .as_str()
        .expect("words")
        .to_owned()
}

#[test]
fn the_game_opens_on_the_title_and_play_starts_the_first_level() {
    let mut run = Run::open_title().expect("the project opens");
    step(&mut run, 3);
    assert_eq!(scene(&run), "title.scene");
    press(&mut run, "play");
    assert_eq!(scene(&run), "level-1.scene");
    assert!(find(&run, "hero").is_some(), "the hero is there");
}

#[test]
fn the_title_reaches_the_levels_and_the_settings_and_back() {
    let mut run = Run::open_title().expect("the project opens");
    step(&mut run, 3);
    press(&mut run, "levels");
    assert_eq!(scene(&run), "levels.scene");
    press(&mut run, "back");
    assert_eq!(scene(&run), "title.scene");
    press(&mut run, "settings");
    assert_eq!(scene(&run), "settings.scene");
    press(&mut run, "back");
    assert_eq!(scene(&run), "title.scene");
}

#[test]
fn pausing_freezes_the_level_and_resuming_finds_it_as_it_was() {
    let mut run = Run::open_title().expect("the project opens");
    step(&mut run, 3);
    press(&mut run, "play");
    let hero = find(&run, "hero").expect("the hero");
    run.key(Key::ArrowRight, true);
    step(&mut run, 60);
    run.key(Key::ArrowRight, false);
    let before = run.position(hero);
    assert!(before[0] > 3.0, "the hero ran: {before:?}");

    press(&mut run, "pause-button");
    assert_eq!(scene(&run), "pause.scene");
    step(&mut run, 120);
    press(&mut run, "resume");
    assert_eq!(scene(&run), "level-1.scene");
    let hero = find(&run, "hero").expect("the hero is still there");
    let after = run.position(hero);
    assert!(
        (after[0] - before[0]).abs() < 0.5,
        "the pause held the level still: {before:?} then {after:?}"
    );
    assert!(after[0] > 3.0, "and did not send it back to the start");
}

#[test]
fn escape_pauses_and_escape_resumes() {
    let mut run = Run::open().expect("the first level opens");
    step(&mut run, 5);
    run.key(Key::Escape, true);
    step(&mut run, 2);
    run.key(Key::Escape, false);
    step(&mut run, 2);
    assert_eq!(scene(&run), "pause.scene");
    run.key(Key::Escape, true);
    step(&mut run, 2);
    run.key(Key::Escape, false);
    step(&mut run, 2);
    assert_eq!(scene(&run), "level-1.scene");
}

#[test]
fn lighting_the_beacon_saves_it_and_a_replay_starts_the_level_over() {
    let mut run = Run::open_title().expect("the project opens");
    step(&mut run, 3);
    press(&mut run, "play");
    step(&mut run, 5);
    assert!(!flag(&run, "beacon_1"));

    // The flag reached, as the level's own script would say it.
    run.set_board("coins", 4.0);
    run.set_board("won", 1.0);
    step(&mut run, 60 * 3);
    assert_eq!(
        scene(&run),
        "levels.scene",
        "the banner is read, then the levels"
    );
    assert!(flag(&run, "beacon_1"), "the beacon is saved");
    assert!((number(&run, "embers_1", 0.0) - 4.0).abs() < f64::EPSILON);
    assert_eq!(
        text(&run, "level-1-label"),
        "1  Dusk Road  -  lit  -  4 embers"
    );

    press(&mut run, "level-1");
    assert_eq!(scene(&run), "level-1.scene");
    assert!(run.board("won") < 0.5, "the level begins again, unwon");
    assert!(run.board("coins") < 0.5, "with nothing collected");
    assert!(flag(&run, "beacon_1"), "and what was earned is kept");
}

#[test]
fn a_volume_is_kept_when_the_settings_are_left() {
    let mut run = Run::open_title().expect("the project opens");
    step(&mut run, 3);
    press(&mut run, "settings");
    assert_eq!(text(&run, "music-value"), "Music 100");
    press(&mut run, "music-down");
    press(&mut run, "music-down");
    assert_eq!(text(&run, "music-value"), "Music 80");
    press(&mut run, "motion");
    assert!(flag(&run, "reduced_motion"));
    press(&mut run, "back");
    assert_eq!(scene(&run), "title.scene");
    assert!((number(&run, "volume_music", 1.0) - 0.8).abs() < 1.0e-9);
    let heard = run
        .session
        .take_audio_commands()
        .into_iter()
        .filter_map(|command| match command {
            AudioCommand::SetVolume { bus, volume } if bus == "music" => Some(volume),
            _ => None,
        })
        .next_back()
        .expect("the music bus was set");
    assert!((heard - 0.8).abs() < 1.0e-6, "to what was chosen: {heard}");
}

#[test]
fn settings_go_back_to_the_pause_screen_they_were_opened_from() {
    let mut run = Run::open().expect("the first level opens");
    step(&mut run, 5);
    press(&mut run, "pause-button");
    press(&mut run, "settings");
    assert_eq!(scene(&run), "settings.scene");
    press(&mut run, "back");
    assert_eq!(scene(&run), "pause.scene");
    press(&mut run, "resume");
    assert_eq!(scene(&run), "level-1.scene");
}

#[test]
fn the_touch_stick_and_jump_work_together() {
    let mut run = Run::open().expect("the first level opens");
    step(&mut run, 90);
    run.set_board("touch", 1.0);
    step(&mut run, 2);

    let hero = find(&run, "hero").expect("the hero");
    let start = run.position(hero);
    let stick = find(&run, "touch-stick-ring").expect("the left virtual stick");
    let [stick_x, stick_y] = run
        .on_screen(stick)
        .expect("the stick is drawn in touch mode");
    let jump = find(&run, "touch-jump").expect("the right jump button");
    let [jump_x, jump_y] = run.on_screen(jump).expect("jump is drawn in touch mode");

    // The first finger owns the virtual stick; the second holds jump without
    // taking steering away from it.
    run.input.apply(InputEvent::TouchStarted {
        id: 1,
        x: stick_x,
        y: stick_y,
    });
    step(&mut run, 1);
    run.input.apply(InputEvent::TouchMoved {
        id: 1,
        x: stick_x + 240.0,
        y: stick_y,
    });
    run.input.apply(InputEvent::TouchStarted {
        id: 2,
        x: jump_x,
        y: jump_y,
    });
    step(&mut run, 30);

    let end = run.position(hero);
    assert!(
        end[0] > start[0] + 1.0,
        "the left thumb moved Wick while the other thumb held jump: {start:?} to {end:?}"
    );
    assert!(
        end[1] > start[1] + 1.0,
        "the right thumb made Wick jump while steering: {start:?} to {end:?}"
    );
}
