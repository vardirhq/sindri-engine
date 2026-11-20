//! The wilds are heard (`docs/lamplighter-plan.md`, stage 6): Wick's feet as
//! his walk lands them (its clip's `step` events), what he gathers, what he
//! builds and the lantern fed, each a Sound Lab effect the project ships.

use std::collections::BTreeSet;

use lamplighter::Run;
use sindri_decay::AudioCommand;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32, heard: &mut BTreeSet<String>) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
        for command in run.session.take_audio_commands() {
            if let AudioCommand::Play { clip, .. } | AudioCommand::Loop { clip, .. } = command {
                heard.insert(clip);
            }
        }
    }
}

fn press(run: &mut Run, key: Key, heard: &mut BTreeSet<String>) {
    run.input.apply(InputEvent::KeyPressed(key));
    step(run, 2, heard);
    run.input.apply(InputEvent::KeyReleased(key));
    step(run, 2, heard);
}

#[test]
fn wick_is_heard_walking_gathering_and_building() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    run.set_board("truce", 1.0);
    let mut heard = BTreeSet::new();
    step(&mut run, 10, &mut heard);
    assert!(
        heard.contains("audio/under-the-lid.ogg"),
        "its music: {heard:?}"
    );
    run.input.apply(InputEvent::KeyPressed(Key::D));
    step(&mut run, 60, &mut heard);
    run.input.apply(InputEvent::KeyReleased(Key::D));
    step(&mut run, 10, &mut heard);
    assert!(heard.contains("audio/step.wav"), "his feet: {heard:?}");

    run.set_board("wood", 4.0);
    run.set_board("stone", 2.0);
    press(&mut run, Key::F, &mut heard);
    press(&mut run, Key::F, &mut heard);
    assert!(heard.contains("audio/build.wav"), "a fire built: {heard:?}");

    run.set_board("resin", 1.0);
    run.set_board("oil", 0.3);
    press(&mut run, Key::R, &mut heard);
    assert!(
        heard.contains("audio/feed.wav"),
        "the lantern fed: {heard:?}"
    );

    let manifest =
        std::fs::read_to_string(lamplighter::project().join("sindri.toml")).expect("the manifest");
    for clip in &heard {
        assert!(manifest.contains(&format!("\"{clip}\"")), "{clip} ships");
        assert!(lamplighter::project().join("assets").join(clip).is_file());
    }
}
