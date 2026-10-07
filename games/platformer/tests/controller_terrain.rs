//! Walk the authored terrain using the actual hero script, without jumping.
use platformer::Run;
use sindri_platform::Key;

const STEP: f32 = 1.0 / 60.0;

#[test]
fn the_hero_walks_up_the_low_steps() {
    let mut run = Run::open().unwrap();
    let hero = run.entity("hero").unwrap();
    for _ in 0..90 {
        assert!(run.step(STEP).is_empty());
    }
    run.key(Key::ArrowRight, true);
    let first = run.entity("low-step-a").unwrap();
    let second = run.entity("low-step-b").unwrap();
    let mut stepped_first = false;
    let mut stepped_second = false;
    for _ in 0..150 {
        assert!(run.step(STEP).is_empty());
        let motion = run.physics.character_motion(hero).unwrap();
        if motion.step_translation[1] > 0.0 {
            assert!(motion.grounded);
            let support = motion.ground.hit.unwrap().entity;
            stepped_first |= support == first;
            stepped_second |= support == second;
        }
        if run.position(hero)[0] > 6.6 {
            assert!(
                stepped_first && stepped_second,
                "both risers use the step path"
            );
            return;
        }
    }
    panic!("blocked at {:?}", run.position(hero));
}

#[test]
fn disabling_steps_keeps_the_same_riser_blocking() {
    let mut run = Run::open().unwrap();
    let hero = run.entity("hero").unwrap();
    run.world
        .get_mut(hero)
        .unwrap()
        .components
        .get_mut("sindri.physics2d.character")
        .unwrap()["step_height"] = serde_json::json!(0.0);
    for _ in 0..90 {
        assert!(run.step(STEP).is_empty());
    }
    run.key(Key::ArrowRight, true);
    for _ in 0..150 {
        assert!(run.step(STEP).is_empty());
        assert!(
            run.physics.character_motion(hero).unwrap().step_translation[1].abs() < f32::EPSILON
        );
    }
    assert!(
        run.position(hero)[0] < 4.8,
        "the riser still blocks walking"
    );
}

#[test]
fn the_hero_climbs_and_snaps_down_the_boardwalk_without_jumping() {
    let mut run = Run::open().unwrap();
    let hero = run.entity("hero").unwrap();
    run.world
        .get_mut(hero)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position[0] = 7.6;
    for _ in 0..90 {
        assert!(run.step(STEP).is_empty());
    }
    let uphill = run.entity("boardwalk-up").unwrap();
    let downhill = run.entity("boardwalk-down").unwrap();
    run.key(Key::ArrowRight, true);
    let mut climbed = false;
    let mut snapped_down = false;
    let mut peak = 0.0_f32;
    for _ in 0..150 {
        assert!(run.step(STEP).is_empty());
        let [x, y] = run.position(hero);
        peak = peak.max(y);
        let motion = run.physics.character_motion(hero).unwrap();
        assert!(!motion.slide.started_penetrating);
        if let Some(hit) = motion.ground.hit {
            if hit.entity == uphill && x > 9.0 {
                climbed |= motion.grounded && hit.normal[0] < -0.3;
            }
            if hit.entity == downhill && x > 11.0 {
                snapped_down |= motion.grounded && motion.snap_translation[1] < -0.01;
            }
        }
        if x > 12.5 {
            assert!(
                climbed && snapped_down,
                "uphill {climbed}, downhill snap {snapped_down}"
            );
            assert!(peak > 4.15 && y < 3.7, "crest {peak}, endpoint {y}");
            return;
        }
    }
    panic!("blocked on the boardwalk at {:?}", run.position(hero));
}

#[test]
fn reducing_the_slope_limit_blocks_the_same_boardwalk() {
    let mut run = Run::open().unwrap();
    let hero = run.entity("hero").unwrap();
    let data = run.world.get_mut(hero).unwrap();
    data.transform_3d.as_mut().unwrap().position[0] = 7.6;
    data.components
        .get_mut("sindri.physics2d.character")
        .unwrap()["max_slope_angle"] = serde_json::json!(0.1);
    data.components
        .get_mut("sindri.physics2d.character")
        .unwrap()["step_height"] = serde_json::json!(0.0);
    for _ in 0..90 {
        assert!(run.step(STEP).is_empty());
    }
    run.key(Key::ArrowRight, true);
    for _ in 0..150 {
        assert!(run.step(STEP).is_empty());
    }
    assert!(run.position(hero)[0] < 8.7, "the steep ramp blocks walking");
}
