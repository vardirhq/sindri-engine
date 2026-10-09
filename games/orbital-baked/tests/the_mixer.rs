//! The pause screen's mixer: three sliders, each a bus, moved by pointer or by
//! the keyboard, heard at once and remembered for the next session.

use orbital_baked::Run;
use sindri_platform::{Key, MemorySaves};

const STEP: f32 = 1.0 / 60.0;

fn settle(run: &mut Run, steps: usize) {
    for _ in 0..steps {
        run.step(STEP);
    }
}

fn press(run: &mut Run, key: Key) {
    run.hold(key);
    run.step(STEP);
    run.let_go(key);
    settle(run, 2);
}

fn paused() -> Run {
    let mut run = Run::open().expect("the project opens");
    settle(&mut run, 6);
    run.click("TitleStart");
    settle(&mut run, 4);
    press(&mut run, Key::Escape);
    run
}

#[test]
fn the_arrows_turn_the_music_down_and_it_is_remembered() {
    let mut run = paused();
    let music = run.find("PauseMusic").expect("a music slider");
    let fill = run.find("PauseMusicFill").expect("its fill");
    assert!((run.session.scripts().audio_volume("music") - 1.0).abs() < 1.0e-6);

    // Resume has focus when the pause opens; down past Quit and Master.
    for _ in 0..3 {
        press(&mut run, Key::ArrowDown);
    }
    assert_eq!(run.session.screen_ui().focused(), Some(music));
    for _ in 0..4 {
        press(&mut run, Key::ArrowLeft);
    }
    assert!((run.session.scripts().audio_volume("music") - 0.8).abs() < 1.0e-5);
    assert!((run.session.saves().number("volume_music", 1.0) - 0.8).abs() < 1.0e-5);
    assert!((run.fill(fill).expect("a fill") - 0.8).abs() < 1.0e-5);
    assert!(
        (run.session.scripts().audio_volume("master") - 1.0).abs() < 1.0e-6,
        "only the bus that was moved"
    );

    // The next session opens with the volume the last one left.
    let saves = run.session.saves().to_document();
    let mut run = Run::open().expect("the project opens");
    run.session
        .keep_saves_in(Box::new(MemorySaves::holding(saves)));
    settle(&mut run, 2);
    assert!((run.session.scripts().audio_volume("music") - 0.8).abs() < 1.0e-5);
}

#[test]
fn a_press_on_the_track_sets_the_master() {
    let mut run = paused();
    run.click("PauseMaster");
    // The middle of the track is half way.
    assert!((run.session.scripts().audio_volume("master") - 0.5).abs() < 0.05);
}
