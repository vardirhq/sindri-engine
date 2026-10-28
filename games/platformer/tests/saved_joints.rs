//! A Decay-spawned nested windmill retains its physical endpoints after save/reopen.
use platformer::Run;
use sindri_core::World;
use sindri_platform::Key;

#[test]
fn saved_spawned_windmill_reopens_with_its_own_axle_and_motor() {
    let mut run = Run::open().unwrap();
    assert!(run.step(1.0 / 60.0).is_empty());
    run.key(Key::V, true);
    assert!(run.step(1.0 / 60.0).is_empty());
    run.key(Key::V, false);
    assert!(run.step(1.0 / 60.0).is_empty());
    run.world.assign_missing_source_ids("saved").unwrap();
    let saved = run
        .world
        .to_scene_with_references(run.prefabs(), run.components())
        .unwrap();
    let text = saved.to_canonical_json().unwrap();
    assert!(!text.contains("prefab_identity"));
    let document = sindri_core::SceneDocument::from_json(&text).unwrap();
    let mut restored = Run::open().unwrap();
    restored.world = World::from_scene_with(&document, restored.prefabs())
        .unwrap()
        .world;
    let named = |name| {
        restored
            .world
            .entities()
            .find_map(|(entity, data)| (data.name.as_deref() == Some(name)).then_some(entity))
            .unwrap()
    };
    let rotor = named("Spawned windmill rotor");
    let axle = named("Spawned windmill axle");
    let mut forward = false;
    let mut backward = false;
    for _ in 0..360 {
        let notes = restored.step(1.0 / 60.0);
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(restored.physics().world().joint_count(), 5);
        let speed = restored.physics().world().angular_velocity(rotor).unwrap();
        forward |= speed > 1.0;
        backward |= speed < -1.0;
        let [x, y] = restored.world.world_transform(rotor).unwrap().position_2d();
        let [ax, ay] = restored.world.world_transform(axle).unwrap().position_2d();
        assert!((x - ax).hypot(y - ay) < 0.02);
        assert!((ax - 5.2).abs() < 0.0001, "reopened axle at {ax}, {ay}");
    }
    assert!(forward && backward);
}
