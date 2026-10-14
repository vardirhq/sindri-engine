//! The coins sparkle and a running hero kicks up dust.
//!
//! Both are `sindri.particles` emitters: each coin's plays from the prefab,
//! and the hero's is switched on by `hero.decay` while it runs and puffs
//! once when it lands.

use platformer::Run;
use sindri_platform::Key;

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

#[test]
fn every_coin_sparkles_while_the_hero_stands_still() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 120);
    let hero = run.entity("hero").expect("the hero");
    assert_eq!(run.session.effects().emitter_live(hero), 0, "no dust standing");
    let sparkling = run
        .world
        .entities()
        .filter(|(entity, _)| run.session.effects().emitter_live(*entity) > 0)
        .count();
    assert_eq!(sparkling, 10, "each of the ten coins");
}

#[test]
fn running_kicks_up_dust_and_stopping_lets_it_settle() {
    let mut run = Run::open().expect("the project opens");
    let hero = run.entity("hero").expect("the hero");
    step(&mut run, 30);
    run.key(Key::ArrowRight, true);
    step(&mut run, 30);
    let running = run.session.effects().emitter_live(hero);
    assert!(running >= 3, "{running} specks of dust after half a second running");
    run.key(Key::ArrowRight, false);
    step(&mut run, 60);
    assert_eq!(run.session.effects().emitter_live(hero), 0, "settled");
}
