//! Ride the authored ferry through the real Hero and Ferry scripts.
use platformer::Run;
use sindri_core::EntityId;
use sindri_platform::Key;

const STEP: f32 = 1.0 / 60.0;

fn aboard() -> (Run, EntityId, EntityId) {
    let mut run = Run::open().unwrap();
    let hero = run.entity("hero").unwrap();
    let ferry = run.entity("gap-ferry").unwrap();
    run.world
        .get_mut(hero)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [13.95, 3.76, 0.5];
    (run, hero, ferry)
}

fn step(run: &mut Run) {
    let notes = run.step(STEP);
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_stationary_rider_is_carried_once_across_the_gap_and_back() {
    let (mut run, hero, ferry) = aboard();
    let mut forward = false;
    let mut backward = false;
    for _ in 0..420 {
        let previous = run.position(ferry)[0];
        step(&mut run);
        let [x, y] = run.position(hero);
        let [platform_x, platform_y] = run.position(ferry);
        let delta = platform_x - previous;
        forward |= delta > 0.01;
        backward |= delta < -0.01;
        assert!((12.93..=15.07).contains(&platform_x));
        assert!(
            (x - platform_x - 1.0).abs() < 0.003,
            "relative x: {x}, {platform_x}"
        );
        assert!((y - platform_y - 0.51).abs() < 0.003);
        let motion = run.physics().character_motion(hero).unwrap();
        assert!(motion.grounded && !motion.slide.started_penetrating);
        let carry = motion.platform.as_ref().unwrap();
        assert_eq!(carry.entity, ferry);
        assert!((carry.motion.translation[0] - delta).abs() < 0.001);
        assert!((motion.translation[0] - delta).abs() < 0.001);
    }
    assert!(forward && backward, "the rider experiences both directions");
    assert!(run.board("falls").abs() < f32::EPSILON);
}

#[test]
fn jumping_leaves_carry_then_landing_resumes_it() {
    let (mut run, hero, ferry) = aboard();
    for _ in 0..60 {
        step(&mut run);
    }
    run.key(Key::Space, true);
    for _ in 0..3 {
        step(&mut run);
    }
    let airborne_x = run.position(hero)[0];
    for _ in 0..20 {
        step(&mut run);
        let motion = run.physics().character_motion(hero).unwrap();
        assert!(!motion.grounded && motion.platform.is_none());
        assert!((run.position(hero)[0] - airborne_x).abs() < 0.001);
    }
    run.key(Key::Space, false);
    let mut landed = false;
    for _ in 0..120 {
        step(&mut run);
        let motion = run.physics().character_motion(hero).unwrap();
        landed |= motion.grounded
            && motion
                .platform
                .as_ref()
                .is_some_and(|carry| carry.entity == ferry);
    }
    assert!(landed, "the airborne hero lands back on the moving ferry");
    assert!(run.board("falls").abs() < f32::EPSILON);
}

#[test]
fn a_running_jump_boards_the_ferry_from_the_starting_field() {
    let mut run = Run::open().unwrap();
    let hero = run.entity("hero").unwrap();
    let ferry = run.entity("gap-ferry").unwrap();
    run.key(Key::ArrowRight, true);
    let mut jumping = false;
    let mut braking = false;
    let mut carried_frames = 0;
    for _ in 0..300 {
        step(&mut run);
        let x = run.position(hero)[0];
        if !jumping && x > 11.6 {
            run.key(Key::Space, true);
            jumping = true;
        }
        if !braking && x > 15.8 {
            run.key(Key::ArrowRight, false);
            run.key(Key::Space, false);
            braking = true;
        }
        let motion = run.physics().character_motion(hero).unwrap();
        if braking && motion.grounded && motion.platform.as_ref().is_some_and(|p| p.entity == ferry)
        {
            carried_frames += 1;
        }
    }
    assert!(
        carried_frames > 60,
        "boarded and rode for a second: {carried_frames}"
    );
    assert!(run.board("falls").abs() < f32::EPSILON);
}

#[test]
fn dropping_through_the_ferry_stops_carry_and_respawns_from_the_pit() {
    let (mut run, hero, _) = aboard();
    for _ in 0..10 {
        step(&mut run);
    }
    run.key(Key::ArrowDown, true);
    for _ in 0..3 {
        step(&mut run);
    }
    run.key(Key::ArrowDown, false);
    let x = run.position(hero)[0];
    for _ in 0..10 {
        step(&mut run);
        let motion = run.physics().character_motion(hero).unwrap();
        assert!(!motion.grounded && motion.platform.is_none());
        assert!((run.position(hero)[0] - x).abs() < 0.001);
    }
    for _ in 0..120 {
        step(&mut run);
    }
    assert!((run.board("falls") - 1.0).abs() < f32::EPSILON);
    assert!((run.position(hero)[0] - 2.5).abs() < 0.001);
    assert!(run.physics().character_motion(hero).unwrap().grounded);
}
