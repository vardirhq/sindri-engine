//! The title's widgets doing something real: the callsign typed there follows
//! the pilot into the run and onto the results, and the compact HUD switch
//! takes the HUD's panels away and leaves its readings.

use orbital_baked::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn settle(run: &mut Run, steps: usize) {
    for _ in 0..steps {
        run.step(STEP);
    }
}

fn text_of(run: &Run, name: &str) -> String {
    run.text(run.find(name).expect("named")).expect("text")
}

fn press(run: &mut Run, key: Key) {
    run.hold(key);
    run.step(STEP);
    run.let_go(key);
    settle(run, 2);
}

#[test]
fn the_callsign_reaches_the_hud_and_the_results() {
    let mut run = Run::open().expect("the project opens");
    settle(&mut run, 6);
    run.click("TitleCallsign");
    for c in "Nova-7".chars() {
        run.input.apply(InputEvent::TextInput(c));
    }
    settle(&mut run, 2);
    assert_eq!(text_of(&run, "TitleCallsignText"), "Nova-7_");
    // Letters typed into the field are not steering: WASD is held back.
    press(&mut run, Key::Escape);
    assert_eq!(text_of(&run, "TitleCallsignText"), "Nova-7");

    run.click("TitleStart");
    settle(&mut run, 4);
    assert_eq!(text_of(&run, "HudPilot"), "PILOT  /  Nova-7");

    press(&mut run, Key::Escape);
    run.click("PauseQuit");
    settle(&mut run, 4);
    assert_eq!(text_of(&run, "ResultPilot"), "PILOT  /  Nova-7");
}

#[test]
fn compact_hud_drops_the_panels_and_keeps_the_readings() {
    let mut run = Run::open().expect("the project opens");
    settle(&mut run, 6);
    run.click("TitleStart");
    settle(&mut run, 4);
    let back = run.find("HudSectorBack").expect("a sector panel");
    let clock = run.find("Clock").expect("a clock");
    assert!(run.world.is_active(back), "the full HUD has its panels");

    let mut run = Run::open().expect("the project opens");
    settle(&mut run, 6);
    run.click("TitleCompact");
    run.click("TitleStart");
    settle(&mut run, 4);
    let back = run.find("HudSectorBack").expect("a sector panel");
    assert!(!run.world.is_active(back), "compact drops the panels");
    assert!(run.world.is_active(clock), "and keeps the readings");
}
