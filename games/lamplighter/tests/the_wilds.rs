//! The wilds (`docs/lamplighter-plan.md`, stage 1): Wick walks a generated land
//! as a pixel sprite standing in 3D space, the tilted camera following him, his
//! lantern lighting the way.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn position(run: &Run, id: &str) -> [f32; 3] {
    let entity = run
        .entity(id)
        .unwrap_or_else(|| panic!("{id} is in the wilds"));
    run.world
        .world_transform(entity)
        .expect("it has a place")
        .position
}

#[test]
fn wick_walks_the_wilds_and_the_tilted_camera_follows_from_behind() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 5);
    let start = position(&run, "wick");

    run.input.apply(InputEvent::KeyPressed(Key::W));
    step(&mut run, 60);
    run.input.apply(InputEvent::KeyReleased(Key::W));
    step(&mut run, 30);
    let north = position(&run, "wick");
    assert!(
        north[1] > start[1] + 2.0,
        "Wick walks north across the land: {start:?} to {north:?}"
    );

    // The camera stands behind him and above, and follows him there.
    let camera = position(&run, "camera");
    assert!(
        (camera[0] - north[0]).abs() < 1.5,
        "level with him: {camera:?}"
    );
    assert!(
        camera[1] < north[1] - 5.0,
        "behind him, looking ahead: {camera:?}"
    );
    assert!(camera[2] > 10.0, "and above: {camera:?}");

    // Standing up, he keeps to the ground however far down the screen he walks,
    // and carries his lantern at his hands rather than a little up the screen.
    assert!(
        north[2].abs() < 1.0e-4,
        "Wick stands on the ground: {north:?}"
    );
    let lantern = position(&run, "wick-lantern");
    assert!(
        (0.85..1.05).contains(&(lantern[2] - north[2])),
        "the lantern is at his hands: {lantern:?}"
    );
    assert!(
        (lantern[1] - north[1]).abs() < 1.0,
        "and round him, not up the screen: {lantern:?}"
    );
}

#[test]
fn a_lamp_posts_flame_is_up_its_post_not_up_the_screen() {
    let run = Run::open_scene("wilds.scene").expect("the wilds open");
    let post = position(&run, "lantern-1");
    let flame = position(&run, "lantern-1-light");
    assert!(
        (flame[0] - post[0]).abs() < 1.0e-4 && (flame[1] - post[1]).abs() < 1.0e-4,
        "the light is at the post: {post:?} {flame:?}"
    );
    let entity = run.entity("lantern-1-light").expect("its light");
    let light = &run.world.get(entity).expect("it exists").components["sindri.light2d"];
    assert!(
        light["height"].as_f64().expect("a height") > 2.0,
        "high up it"
    );
}

/// The dead trees, boulders, oaks, bushes and ground cover the scatter has
/// made, with where each stands.
fn grown(run: &Run) -> Vec<(sindri_core::EntityId, String, [f32; 3])> {
    run.world
        .entities()
        .filter_map(|(entity, data)| {
            let texture = data.components.get("sindri.sprite")?["texture"].as_str()?;
            let kind = texture.split('#').nth(1)?;
            let grows = [
                "dead-tree",
                "boulder",
                "oak",
                "bush",
                "tufts",
                "pebbles",
                "flowers",
            ];
            grows.iter().any(|g| kind.starts_with(g)).then(|| {
                let at = run.world.world_transform(entity).expect("placed").position;
                (entity, kind.to_owned(), at)
            })
        })
        .collect()
}

#[test]
fn the_wilds_grow_round_wick_and_stand_facing_the_camera() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 2);
    let grown = grown(&run);
    assert!(
        grown.len() > 40,
        "a land's worth of things: {}",
        grown.len()
    );
    assert!(
        grown
            .iter()
            .any(|(_, kind, _)| kind.starts_with("dead-tree"))
    );
    for (entity, kind, at) in &grown {
        let sprite = &run.world.get(*entity).expect("it exists").components["sindri.sprite"];
        assert_eq!(sprite["billboard"], true, "{kind} stands up");
        assert!(at[2].abs() < 0.1, "{kind} stands on the ground: {at:?}");
        assert!(
            at[0].abs() <= 27.0 && at[1].abs() <= 27.0,
            "{kind} is near Wick: {at:?}"
        );
        assert!(
            at[0].hypot(at[1]) > 2.5,
            "Wick's start is clear: {kind} at {at:?}"
        );
    }
    for id in ["wick", "lantern-1", "gloom-1"] {
        let entity = run.entity(id).expect("it is in the wilds");
        let sprite = &run.world.get(entity).expect("it exists").components["sindri.sprite"];
        assert_eq!(sprite["billboard"], true, "{id} stands up");
    }
}

/// The land goes on: walking away, what is behind Wick is unmade and what is
/// ahead is made, and walking back finds the same things where they were.
#[test]
fn walking_on_grows_what_is_ahead_and_the_same_things_come_back() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 2);
    let home: Vec<_> = grown(&run)
        .into_iter()
        .map(|(_, kind, at)| (kind, at))
        .collect();
    let far = run.entity("wick").expect("Wick");
    // Carried rather than walked, so the trees in the way do not stop him.
    let set = |run: &mut Run, x: f32| {
        let data = run.world.get_mut(far).expect("Wick");
        let mut placed = data.transform_3d.expect("placed");
        placed.position[0] = x;
        data.transform_3d = Some(placed);
    };
    set(&mut run, 60.0);
    step(&mut run, 2);
    let away = grown(&run);
    assert!(
        away.iter().all(|(_, _, at)| at[0] > 25.0),
        "what was round the start is gone"
    );
    assert!(
        away.len() > 40,
        "and new land grows round him: {}",
        away.len()
    );
    set(&mut run, 0.0);
    step(&mut run, 2);
    let back: Vec<_> = grown(&run)
        .into_iter()
        .map(|(_, kind, at)| (kind, at))
        .collect();
    for thing in &home {
        assert!(back.contains(thing), "{thing:?} grows back where it stood");
    }
}

/// What is taken away does not grow back: a felled tree stays felled.
#[test]
fn what_is_taken_stays_taken() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 2);
    let (tree, _, at) = grown(&run)
        .into_iter()
        .find(|(_, kind, _)| kind.starts_with("dead-tree"))
        .expect("a dead tree");
    run.world.despawn_recursive(tree).expect("felled");
    step(&mut run, 2);
    let ground = run.entity("ground").expect("the ground");
    let scatter = &run.world.get(ground).expect("it exists").components["sindri.scatter"];
    assert_eq!(
        scatter["taken"].as_array().map(Vec::len),
        Some(1),
        "remembered"
    );
    let wick = run.entity("wick").expect("Wick");
    let move_wick = |run: &mut Run, x: f32| {
        let data = run.world.get_mut(wick).expect("Wick");
        let mut placed = data.transform_3d.expect("placed");
        placed.position[0] = x;
        data.transform_3d = Some(placed);
    };
    move_wick(&mut run, 80.0);
    step(&mut run, 2);
    move_wick(&mut run, 0.0);
    step(&mut run, 2);
    assert!(
        !grown(&run)
            .iter()
            .any(|(_, _, there)| (there[0] - at[0]).hypot(there[1] - at[1]) < 1.0e-4),
        "it does not grow back"
    );
}
