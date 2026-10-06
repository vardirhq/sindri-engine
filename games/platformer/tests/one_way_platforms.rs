//! Play the authored planks through the game's real Decay input path.
use platformer::Run;
use sindri_platform::Key;
const STEP: f32 = 1.0 / 60.0;
fn step(run: &mut Run) {
    let notes = run.step(STEP);
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn jump_from_below_land_drop_and_jump_again() {
    let mut run = Run::open().unwrap();
    let hero = run.entity("hero").unwrap();
    run.world
        .get_mut(hero)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position[0] = 7.0;
    for _ in 0..90 {
        step(&mut run);
    }
    assert!((run.position(hero)[1] - 3.5).abs() < 0.08);
    run.key(Key::Space, true);
    let mut highest = 0.0_f32;
    for _ in 0..60 {
        step(&mut run);
        highest = highest.max(run.position(hero)[1]);
    }
    run.key(Key::Space, false);
    assert!(highest > 6.2, "rose through the planks: {highest}");
    assert!(
        (run.position(hero)[1] - 6.0).abs() < 0.08,
        "landed on planks: {:?}",
        run.position(hero)
    );
    run.key(Key::ArrowDown, true);
    for _ in 0..90 {
        step(&mut run);
    }
    run.key(Key::ArrowDown, false);
    assert!(
        (run.position(hero)[1] - 3.5).abs() < 0.08,
        "ordinary floor: {:?}",
        run.position(hero)
    );
    run.key(Key::Space, true);
    for _ in 0..65 {
        step(&mut run);
    }
    assert!(
        (run.position(hero)[1] - 6.0).abs() < 0.08,
        "drop-through expired"
    );
}

#[test]
fn underside_sensor_overlap_does_not_grant_a_jump() {
    let mut run = Run::open().unwrap();
    let hero = run.entity("hero").unwrap();
    let transform = run
        .world
        .get_mut(hero)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap();
    transform.position[0] = 7.0;
    transform.position[1] = 5.8;
    run.key(Key::Space, true);
    step(&mut run);
    let planks = run.entity("one-way-planks").unwrap();
    assert!(
        run.physics
            .events()
            .iter()
            .any(|event| (event.first == hero && event.second == planks)
                || (event.first == planks && event.second == hero)),
        "a pickup sensor really overlaps the plank"
    );
    step(&mut run);
    assert!(
        run.position(hero)[1] < 5.8,
        "overlapping the underside never grants jump permission"
    );
}
