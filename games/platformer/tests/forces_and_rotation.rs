//! Play the crate's real input-driven force and rotation controls.

use platformer::Run;
use sindri_platform::Key;

#[test]
fn kicking_the_wind_crate_launches_and_rotates_it() {
    let mut run = Run::open().unwrap();
    let crate_entity = run.entity("wind-crate").unwrap();
    for _ in 0..90 {
        assert!(run.step(1.0 / 60.0).is_empty());
    }
    // Generic collider dimensions are already world units; visual scale does
    // not shrink their mass. Check the authored crate really is 0.6 by 0.6.
    assert!((run.physics.world().mass(crate_entity).unwrap() - 0.36).abs() < 1.0e-4);
    let before = run.position(crate_entity);
    run.key(Key::K, true);
    let notes = run.step(1.0 / 60.0);
    assert!(notes.is_empty(), "{notes:?}");
    run.key(Key::K, false);
    let velocity = run.physics.world().linear_velocity(crate_entity).unwrap();
    assert!(
        velocity[1] > 3.0,
        "kick velocity: {velocity:?}, position: {before:?}"
    );
    assert!(
        run.physics
            .world()
            .angular_velocity(crate_entity)
            .unwrap()
            .abs()
            > 0.01
    );
    for _ in 0..20 {
        let notes = run.step(1.0 / 60.0);
        assert!(notes.is_empty(), "{notes:?}");
    }
    assert!(run.position(crate_entity)[0] > before[0] + 0.3);
    let rotation = run
        .world
        .get(crate_entity)
        .unwrap()
        .transform_3d
        .unwrap()
        .rotation;
    assert!(rotation[2].abs() > 0.01);
}
