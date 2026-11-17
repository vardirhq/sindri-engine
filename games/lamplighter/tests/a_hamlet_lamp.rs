//! A hamlet's lamp (`docs/lamplighter-plan.md`, stage 1): each hamlet the seed
//! builds has a lamp post by its well, dark until Wick lights it; lit, it is a
//! safe place, and out in the wilds lighting lamps does not end the run. Its
//! folk come out once it is lit, and thank Wick with what they can spare.

use lamplighter::Run;
use sindri_core::EntityId;

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn place(run: &mut Run, entity: EntityId, at: [f32; 2]) {
    let transform = run
        .world
        .get_mut(entity)
        .expect("it exists")
        .transform_3d
        .as_mut()
        .expect("a place");
    transform.position[0] = at[0];
    transform.position[1] = at[1];
}

/// A hamlet's lamp post: a lamp that is not one of the two by the start.
fn hamlet_lamp(run: &Run) -> Option<(EntityId, [f32; 2])> {
    run.world
        .entities()
        .find(|(_, data)| {
            data.name.as_deref() == Some("lamp-post")
                && data
                    .components
                    .get("sindri.script")
                    .is_some_and(|script| script["script"] == "Lantern")
        })
        .map(|(entity, _)| {
            let at = run.world.world_transform(entity).expect("a place").position;
            (entity, [at[0], at[1]])
        })
}

#[test]
fn wick_lights_a_hamlets_lamp_and_it_shelters_him() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    run.set_board("truce", 1.0);
    step(&mut run, 10);
    let wick = run.entity("wick").expect("Wick");
    let start = run.world.world_transform(wick).expect("a place").position;
    // Walk out until a hamlet is grown.
    let mut found = None;
    'search: for reach in [40.0_f32, 80.0, 120.0, 160.0, 200.0] {
        for (dx, dy) in [
            (1.0, 0.0),
            (0.0, 1.0),
            (-1.0, 0.0),
            (0.0, -1.0),
            (0.7, 0.7),
            (-0.7, 0.7),
        ] {
            place(
                &mut run,
                wick,
                [start[0] + dx * reach, start[1] + dy * reach],
            );
            step(&mut run, 3);
            if let Some(lamp) = hamlet_lamp(&run) {
                found = Some(lamp);
                break 'search;
            }
        }
    }
    let (lamp, at) = found.expect("a hamlet with a lamp somewhere");
    let lit = |run: &Run| {
        run.world.get(lamp).expect("the lamp").components["sindri.light2d"]["intensity"]
            .as_f64()
            .expect("a number")
    };
    assert!(lit(&run) < 0.01, "it is dark");
    let lit_before = run.board("lit");
    place(&mut run, wick, [at[0], at[1] - 0.6]);
    step(&mut run, 30);
    assert!(lit(&run) > 1.0, "Wick lights it: {}", lit(&run));
    assert!(
        (run.board("lit") - lit_before - 1.0).abs() < 1e-3,
        "and it counts"
    );
    assert!(run.board("won") < 0.5, "but ends nothing out here");
    run.set_board("courage", 0.3);
    let berries = run.board("berries");
    step(&mut run, 60 * 2);
    assert!(run.board("courage") > 0.4, "its light gives him courage");
    assert!(
        (run.board("helped") - 1.0).abs() < 1e-3,
        "someone comes out"
    );
    assert!(
        run.board("berries") >= berries + 2.0 - 1e-3 || run.board("eaten") > 0.0,
        "and gives him food"
    );
    step(&mut run, 60);
    assert!((run.board("helped") - 1.0).abs() < 1e-3, "once");
}
