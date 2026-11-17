//! Visions appear only when Wick has no oil and no courage.
use lamplighter::Run;

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn visible(run: &Run) -> usize {
    ["vision-watcher", "vision-tall", "vision-wick"]
        .iter()
        .filter(|name| {
            let entity = run.entity(name).expect("vision exists");
            let data = run.world.get(entity).expect("vision data");
            data.components["sindri.sprite"]["tint"][3]
                .as_f64()
                .expect("opacity") > 0.01
        })
        .count()
}

#[test]
fn visions_only_appear_in_the_dark_and_clear_when_oil_returns() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 10);
    let wick = run.entity("wick").expect("Wick");
    let data = run.world.get_mut(wick).expect("Wick");
    let mut placed = data.transform_3d.expect("transform");
    placed.position = [100.0, 100.0, 0.0];
    data.transform_3d = Some(placed);
    run.set_board("truce", 1.0);
    run.set_board("courage", 0.0);
    run.set_board("oil", 0.0);
    let mut saw_vision = false;
    for _ in 0..30 {
        step(&mut run, 60);
        if visible(&run) == 1 {
            saw_vision = true;
            break;
        }
    }
    assert!(saw_vision, "one vision appears");
    run.set_board("oil", 1.0);
    step(&mut run, 2);
    assert_eq!(visible(&run), 0, "light clears the vision");
}

#[test]
fn either_oil_or_courage_keeps_the_visions_away() {
    for (oil, courage) in [(1.0, 0.0), (0.0, 1.0)] {
        let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
        step(&mut run, 10);
        for _ in 0..(60 * 25) {
            run.set_board("oil", oil);
            run.set_board("courage", courage);
            step(&mut run, 1);
            assert_eq!(visible(&run), 0, "a vision appeared with oil or courage");
        }
    }
}
