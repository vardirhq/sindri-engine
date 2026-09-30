use super::{LIMIT, Track, Tweens};
use decay_runtime::Value;
use sindri_core::{Easing, EntityData, World};

fn track(owner: sindri_core::EntityId) -> Track {
    Track {
        owner,
        from: Value::Number(0.0),
        to: Value::Number(1.0),
        duration: 1.0,
        elapsed: 0.0,
        easing: Easing::Linear,
        paused: false,
        cancelled: false,
    }
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
