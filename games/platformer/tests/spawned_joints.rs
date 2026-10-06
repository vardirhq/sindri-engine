//! The game places and removes an authored joint mechanism through Decay.
use platformer::Run;
use sindri_core::EntityId;
use sindri_platform::Key;

fn named(run: &Run, name: &str) -> EntityId {
    run.world
        .entities()
        .find_map(|(entity, data)| (data.name.as_deref() == Some(name)).then_some(entity))
        .unwrap()
}

#[test]
fn spawned_windmill_resolves_local_endpoints_and_survives_repeated_placement() {
    let mut run = Run::open().unwrap();
    assert!(run.step(1.0 / 60.0).is_empty());
    for _ in 0..2 {
        run.key(Key::V, true);
        assert!(run.step(1.0 / 60.0).is_empty());
        run.key(Key::V, false);
        let rotor = named(&run, "Spawned windmill rotor");
        let anchor = named(&run, "Spawned windmill axle");
        let mut forward = false;
        let mut backward = false;
        for _ in 0..360 {
            let notes = run.step(1.0 / 60.0);
            assert!(notes.is_empty(), "{notes:?}");
            let speed = run.physics.world().angular_velocity(rotor).unwrap();
            forward |= speed > 1.0;
            backward |= speed < -1.0;
            let [x, y] = run.world.world_transform(rotor).unwrap().position_2d();
            let [ax, ay] = run.world.world_transform(anchor).unwrap().position_2d();
            assert!((x - ax).hypot(y - ay) < 0.02);
            assert!((ax - 5.2).abs() < 1e-4);
            assert_eq!(run.physics.world().joint_count(), 5);
            assert!(run.world.get(rotor).unwrap().source_id.is_none());
        }
        assert!(forward && backward);
        run.key(Key::V, true);
        assert!(run.step(1.0 / 60.0).is_empty());
        run.key(Key::V, false);
        assert!(run.step(1.0 / 60.0).is_empty());
        assert!(!run.world.contains(rotor));
        assert!(!run.world.contains(anchor));
        assert_eq!(run.physics.world().joint_count(), 4);
    }
}
