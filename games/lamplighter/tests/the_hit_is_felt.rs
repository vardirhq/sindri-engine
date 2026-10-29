//! A pounce that lands is felt: the screen reddens, Wick flashes and blinks while he cannot be
//! hurt again, and he is thrown back; the lantern's bar on the HUD shrinks with the scorch.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn place(run: &mut Run, id: &str, at: [f32; 2]) {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    let data = run.world.get_mut(entity).expect("it exists");
    let transform = data.transform_3d.as_mut().expect("a place");
    transform.position[0] = at[0];
    transform.position[1] = at[1];
}

fn alpha(run: &Run, id: &str) -> f64 {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    run.world.get(entity).expect("it exists").components["sindri.sprite"]["tint"][3]
        .as_f64()
        .expect("an alpha")
}

fn fill(run: &Run, id: &str) -> f64 {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    run.world.get(entity).expect("it exists").components["sindri.ui.image"]["fill"]["amount"]
        .as_f64()
        .expect("a fill")
}

#[test]
fn a_landed_pounce_flashes_the_screen_blinks_wick_and_empties_the_bar() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    run.input.apply(InputEvent::KeyPressed(Key::D));
    step(&mut run, 3);
    run.input.apply(InputEvent::KeyReleased(Key::D));
    place(&mut run, "wick", [0.0, 0.0]);
    place(&mut run, "brute-1", [2.5, 0.0]);
    let (mut flash, mut blink, mut low) = (0.0_f32, 1.0_f64, 1.0_f64);
    for _ in 0..150 {
        step(&mut run, 1);
        flash = flash.max(run.board("hit"));
        blink = blink.min(alpha(&run, "wick-sprite"));
        low = low.min(fill(&run, "hud-lantern-fill"));
    }
    assert!(flash > 0.9, "the screen reddens at the blow: {flash}");
    assert!(
        blink < 0.6,
        "Wick blinks while he cannot be hurt again: {blink}"
    );
    assert!(low < 0.7, "the lantern's bar empties: {low}");
}
