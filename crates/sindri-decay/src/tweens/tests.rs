use super::{LIMIT, Track, Tweens};
use decay_runtime::Value;
use sindri_core::{Easing, EntityData, World};

fn track(owner: sindri_core::EntityId) -> Track {
    Track::new(
        owner,
        Value::Number(0.0),
        Value::Number(1.0),
        1.0,
        Easing::Linear,
    )
}

fn number(tweens: &Tweens, id: u64) -> f64 {
    match tweens.get(id).expect("track").value() {
        Value::Number(n) => n,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_delayed_yoyo_waits_then_runs_there_and_back_for_its_loops() {
    let owner = World::default().spawn(EntityData::default());
    let mut tweens = Tweens::default();
    let mut pulse = track(owner);
    pulse.delay = 0.5;
    pulse.loops = 2;
    pulse.yoyo = true;
    let id = tweens.insert(pulse).expect("insert");
    tweens.advance(owner, 0.5);
    assert!(number(&tweens, id).abs() < 1.0e-9, "held through the delay");
    tweens.advance(owner, 0.75);
    assert!((number(&tweens, id) - 0.75).abs() < 1.0e-9);
    tweens.advance(owner, 0.5);
    assert!(
        (number(&tweens, id) - 0.75).abs() < 1.0e-9,
        "on its way back"
    );
    assert!(!tweens.get(id).expect("track").done());
    tweens.advance(owner, 10.0);
    assert!(tweens.get(id).expect("track").done());
    assert!(
        number(&tweens, id).abs() < 1.0e-9,
        "a yoyo's second play ends at the start"
    );
}

#[test]
fn an_endless_loop_never_finishes_and_a_sequence_waits_its_turn() {
    let owner = World::default().spawn(EntityData::default());
    let mut tweens = Tweens::default();
    let first = tweens.insert(track(owner)).expect("insert");
    let mut endless = track(owner);
    endless.loops = 0;
    endless.after = Some(first);
    let second = tweens.insert(endless).expect("insert");
    tweens.advance(owner, 0.6);
    assert!(
        number(&tweens, second).abs() < 1.0e-9,
        "waits for the first"
    );
    tweens.advance(owner, 0.6);
    assert!(tweens.get(first).expect("track").done());
    tweens.advance(owner, 0.25);
    assert!((number(&tweens, second) - 0.25).abs() < 1.0e-9);
    tweens.advance(owner, 100.0);
    assert!(!tweens.get(second).expect("track").done(), "for ever");
    // A sequence whose predecessor is gone goes on without it.
    let mut orphan = track(owner);
    orphan.after = Some(first);
    let orphan = tweens.insert(orphan).expect("insert");
    tweens.dispose(first);
    tweens.advance(owner, 0.5);
    assert!((number(&tweens, orphan) - 0.5).abs() < 1.0e-9);
}

#[test]
fn owners_advance_independently_and_reclamation_never_reuses_handles() {
    let mut world = World::default();
    let first = world.spawn(EntityData::default());
    let second = world.spawn(EntityData::default());
    let mut tweens = Tweens::default();
    let a = tweens.insert(track(first)).expect("insert");
    let b = tweens.insert(track(second)).expect("insert");
    tweens.advance(first, 0.5);
    assert!((tweens.get(a).expect("track").progress() - 0.5).abs() < f64::EPSILON);
    assert!(tweens.get(b).expect("track").progress().abs() < f64::EPSILON);
    tweens.retain(|owner| owner == second);
    assert!(tweens.get(a).is_none());
    let c = tweens.insert(track(first)).expect("insert");
    assert!(c > b);
    tweens.remove_owner(first);
    assert!(tweens.get(c).is_none());
    tweens.clear();
    assert!(tweens.get(b).is_none());
    assert!(tweens.insert(track(second)).expect("insert") > c);
}

#[test]
fn retained_handles_are_bounded_and_disposal_reclaims_capacity() {
    let owner = World::default().spawn(EntityData::default());
    let mut tweens = Tweens::default();
    for _ in 0..LIMIT {
        tweens.insert(track(owner)).expect("within bound");
    }
    assert!(tweens.insert(track(owner)).is_err());
    tweens.dispose(1);
    assert!(tweens.insert(track(owner)).is_ok());
}
