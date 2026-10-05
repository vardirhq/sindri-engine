//! The lantern swings through Decay forces while an authored tether holds it.

use platformer::Run;

#[test]
fn wind_moves_the_lantern_without_escaping_its_authored_tether() {
    let mut run = Run::open().unwrap();
    let lantern = run.entity("wind-lantern").unwrap();
    let hook = run.entity("lantern-anchor").unwrap();
    let mut moved = false;
    for _ in 0..300 {
        let notes = run.step(1.0 / 60.0);
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(run.physics.world().joint_count(), 1);
        let [x, y] = run.position(lantern);
        let [hx, hy] = run.position(hook);
        assert!((x - hx).hypot(y - hy) < 2.04, "lantern escaped its tether");
        moved |= (x - hx).abs() > 0.15;
    }
    assert!(moved, "the scripted wind must move the physical body");
    let tether = run.entity("lantern-tether").unwrap();
    run.world.despawn_recursive(tether).unwrap();
    assert!(run.step(1.0 / 60.0).is_empty());
    assert_eq!(run.physics.world().joint_count(), 0);
}
