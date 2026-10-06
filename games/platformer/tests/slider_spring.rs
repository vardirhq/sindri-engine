//! A powered rail carries a spring light; Decay controls both mechanisms.
use platformer::Run;
#[test]
fn trolley_reverses_inside_its_rail_and_spring_retuning_changes_light_height() {
    let mut run = Run::open().unwrap();
    let trolley = run.entity("trolley-body").unwrap();
    let light = run.entity("spring-lantern").unwrap();
    let mut forward = false;
    let mut backward = false;
    let mut shortened = false;
    let mut shortest = f32::INFINITY;
    let mut longest = 0.0_f32;
    let mut lengthened = false;
    for frame in 0..480 {
        let notes = run.step(1.0 / 60.0);
        assert!(notes.is_empty(), "{notes:?}");
        let [x, y] = run.position(trolley);
        assert!((8.47..=10.53).contains(&x), "trolley escaped at {x}");
        assert!((y - 8.1).abs() < 0.03);
        let speed = run.physics.world().linear_velocity(trolley).unwrap()[0];
        forward |= speed > 0.4;
        backward |= speed < -0.4;
        let [lx, ly] = run.position(light);
        let length = (x - lx).hypot(y - ly);
        assert!(length < 1.9, "spring light escaped at length {length}");
        if (90..180).contains(&frame) {
            shortened |= length < 1.15;
            shortest = shortest.min(length);
        }
        if (270..360).contains(&frame) {
            lengthened |= length > 1.4;
            longest = longest.max(length);
        }
        assert_eq!(run.physics.world().joint_count(), 4);
    }
    assert!(forward && backward, "the scripted slider must reverse");
    assert!(
        shortened && lengthened,
        "rest length tuning must move the physical light: short {shortest}, long {longest}"
    );
    let spring = run.entity("lantern-spring").unwrap();
    run.world.despawn_recursive(spring).unwrap();
    assert!(run.step(1.0 / 60.0).is_empty());
    assert_eq!(run.physics.world().joint_count(), 3);
    let slider = run.entity("trolley-slider").unwrap();
    run.world.despawn_recursive(slider).unwrap();
    assert!(run.step(1.0 / 60.0).is_empty());
    assert_eq!(run.physics.world().joint_count(), 2);
}
