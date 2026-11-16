//! Under the Lid (`docs/lamplighter-plan.md`, stage 2): sunfalls wander and come
//! and go, and a Seeker sweeps the land, finds Wick and calls the gloom, unless
//! a sunfall's pool hides him.

use lamplighter::Run;

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn position(run: &Run, id: &str) -> [f32; 2] {
    let entity = run
        .entity(id)
        .unwrap_or_else(|| panic!("{id} is in the wilds"));
    let at = run
        .world
        .world_transform(entity)
        .expect("it has a place")
        .position;
    [at[0], at[1]]
}

fn place(run: &mut Run, id: &str, at: [f32; 2]) {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    let data = run.world.get_mut(entity).expect("it exists");
    let transform = data.transform_3d.as_mut().expect("a place");
    transform.position[0] = at[0];
    transform.position[1] = at[1];
}

fn component(run: &Run, id: &str, name: &str, field: &str) -> f64 {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    run.world.get(entity).expect("it exists").components[name][field]
        .as_f64()
        .unwrap_or_else(|| panic!("{id}'s {name}.{field} is a number"))
}

/// The sunfall by the start wanders, and a few seconds in its crack closes:
/// its shaft and its pool fade to nothing.
#[test]
fn a_sunfall_wanders_and_its_crack_closes() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 30);
    let first = position(&run, "sunfall-1");
    assert!(
        component(&run, "sunfall-1", "sindri.light2d", "intensity") > 1.5,
        "open at the start"
    );
    step(&mut run, 120);
    let later = position(&run, "sunfall-1");
    assert!(
        (later[0] - first[0]).hypot(later[1] - first[1]) > 0.02,
        "it has wandered: {first:?} to {later:?}"
    );
    // Its time comes from where it fell; this one's crack closes about eight
    // seconds in and takes four to fade.
    step(&mut run, 60 * 12);
    assert!(
        component(&run, "sunfall-1", "sindri.light2d", "intensity") < 0.05,
        "its pool has gone dark"
    );
    let entity = run.entity("sunfall-1").expect("the sunfall");
    let params =
        &run.world.get(entity).expect("it exists").components["sindri.sprite"]["shader_params"];
    assert!(
        params[0].as_f64().expect("a number") < 0.05,
        "and its shaft"
    );
}

/// A Seeker's circle that falls on Wick finds him: it burns brighter and the
/// gloom hear it. He outruns it, and it loses him; in a little while the gloom
/// stop hearing.
#[test]
fn a_seeker_finds_wick_and_loses_him() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 10);
    let calm = component(&run, "seeker-1", "sindri.light2d", "intensity");
    let wick = position(&run, "wick");
    place(&mut run, "seeker-1", wick);
    step(&mut run, 30);
    assert!(run.board("found") > 0.9, "found");
    let fierce = component(&run, "seeker-1", "sindri.light2d", "intensity");
    assert!(fierce > calm + 0.8, "burning brighter: {calm} to {fierce}");

    place(&mut run, "wick", [wick[0] + 12.0, wick[1] - 8.0]);
    step(&mut run, 60 * 4);
    assert!(
        run.board("found") < 0.05,
        "lost, and the gloom no longer hear it"
    );
}

/// Standing in an open sunfall's pool, Wick is hidden: a Seeker's circle on him
/// does not find him.
#[test]
fn an_open_sunfall_hides_wick_from_a_seeker() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 10);
    let pool = position(&run, "sunfall-1");
    place(&mut run, "wick", pool);
    place(&mut run, "seeker-1", pool);
    step(&mut run, 30);
    assert!(run.board("found") < 0.05, "hidden in cracklight");
}
