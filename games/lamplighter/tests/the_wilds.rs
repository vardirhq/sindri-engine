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
    let start = position(&run, "wick");
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
        let (dx, dy) = (at[0] - start[0], at[1] - start[1]);
        assert!(
            dx.abs() <= 27.0 && dy.abs() <= 27.0,
            "{kind} is near Wick: {at:?}"
        );
        assert!(
            dx.hypot(dy) > 2.5,
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
    let start_x = position(&run, "wick")[0];
    set(&mut run, start_x + 60.0);
    step(&mut run, 2);
    let away = grown(&run);
    assert!(
        away.iter().all(|(_, _, at)| at[0] > start_x + 25.0),
        "what was round the start is gone"
    );
    assert!(
        away.len() > 40,
        "and new land grows round him: {}",
        away.len()
    );
    set(&mut run, start_x);
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
    let start_x = position(&run, "wick")[0];
    move_wick(&mut run, start_x + 80.0);
    step(&mut run, 2);
    move_wick(&mut run, start_x);
    step(&mut run, 2);
    assert!(
        !grown(&run)
            .iter()
            .any(|(_, _, there)| (there[0] - at[0]).hypot(there[1] - at[1]) < 1.0e-4),
        "it does not grow back"
    );
}

/// The walls the land raises over water near Wick: each one's middle and half
/// size.
fn walls(run: &Run) -> Vec<([f32; 3], [f32; 2])> {
    run.world
        .entities()
        .filter(|(_, data)| data.name.as_deref() == Some("Map wall"))
        .filter_map(|(entity, data)| {
            let half = &data.components["sindri.physics2d.collider"]["shape"]["half_extents"];
            #[allow(clippy::cast_possible_truncation)]
            let half = [half[0].as_f64()? as f32, half[1].as_f64()? as f32];
            Some((run.world.world_transform(entity)?.position, half))
        })
        .collect()
}

/// Water stops Wick: the land raises walls over what cannot be walked on near
/// him, wherever he is.
#[test]
fn water_stops_wick() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 2);
    let near = walls(&run);
    assert!(!near.is_empty(), "the wilds have water near the start");
    let inside = |walls: &[([f32; 3], [f32; 2])], x: f32, y: f32| {
        walls
            .iter()
            .any(|(at, half)| (x - at[0]).abs() <= half[0] && (y - at[1]).abs() <= half[1])
    };
    // A wall with open ground two squares north of its middle, to walk south from.
    let (wall, _) = near
        .iter()
        .find(|(at, _)| {
            !inside(&near, at[0], at[1] + 1.0)
                && !inside(&near, at[0], at[1] + 2.0)
                && !inside(&near, at[0] + 0.4, at[1] + 1.0)
                && !inside(&near, at[0] - 0.4, at[1] + 1.0)
        })
        .copied()
        .expect("a shore to stand on");
    let wick = run.entity("wick").expect("Wick");
    let data = run.world.get_mut(wick).expect("Wick");
    let mut placed = data.transform_3d.expect("placed");
    placed.position = [wall[0], wall[1] + 2.0, 0.0];
    data.transform_3d = Some(placed);
    step(&mut run, 2);
    run.input.apply(InputEvent::KeyPressed(Key::S));
    step(&mut run, 90);
    run.input.apply(InputEvent::KeyReleased(Key::S));
    let stopped = position(&run, "wick");
    assert!(
        stopped[1] > wall[1] + 0.3,
        "Wick stops at the water's edge: the wall is at {wall:?}, he is at {stopped:?}"
    );
    // And far away, the walls near the start come down and stand near him.
    let data = run.world.get_mut(wick).expect("Wick");
    let mut placed = data.transform_3d.expect("placed");
    placed.position = [4_000.0, -3_000.0, 0.0];
    data.transform_3d = Some(placed);
    step(&mut run, 2);
    let there = position(&run, "wick");
    assert!(
        walls(&run)
            .iter()
            .all(|(at, _)| (at[0] - there[0]).abs() < 60.0 && (at[1] - there[1]).abs() < 60.0),
        "the walls near the start came down, and any standing are near him"
    );
}

/// However far Wick goes, the world is moved back round him: he is near the
/// origin, the land has moved with him, and he stands on the same square.
#[test]
fn far_out_the_world_is_moved_back_round_wick() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 2);
    let wick = run.entity("wick").expect("Wick");
    let ground = run.entity("ground").expect("the ground");
    let data = run.world.get_mut(wick).expect("Wick");
    let mut placed = data.transform_3d.expect("placed");
    placed.position = [100_000.3, -70_000.6, 0.0];
    data.transform_3d = Some(placed);
    let before = position(&run, "ground");
    step(&mut run, 2);
    let here = position(&run, "wick");
    assert!(
        here[0].abs() <= 512.0 && here[1].abs() <= 512.0,
        "near the origin: {here:?}"
    );
    let land = position(&run, "ground");
    let moved = [land[0] - before[0], land[1] - before[1]];
    assert!(
        (here[0] - (100_000.3 + moved[0])).abs() < 0.05
            && (here[1] - (-70_000.6 + moved[1])).abs() < 0.05,
        "he moved with the land: moved {moved:?}, he is at {here:?}"
    );
    let origin = &run.world.get(ground).expect("it exists").components["sindri.floating_origin"];
    #[allow(clippy::cast_possible_truncation)]
    let recorded = [
        origin["moved"][0].as_f64().expect("a number") as f32,
        origin["moved"][1].as_f64().expect("a number") as f32,
    ];
    assert!(
        (recorded[0] - moved[0]).abs() < 1.0e-3 && (recorded[1] - moved[1]).abs() < 1.0e-3,
        "and how far is kept: {recorded:?} {moved:?}"
    );
    assert!(!grown(&run).is_empty(), "the land is dressed out there too");
}

/// Wick starts on dry ground: no wall stands where he does.
#[test]
fn wick_starts_on_dry_ground() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 2);
    let at = position(&run, "wick");
    for (wall, half) in walls(&run) {
        let clear =
            (at[0] - wall[0]).abs() > half[0] + 1.0 || (at[1] - wall[1]).abs() > half[1] + 1.0;
        assert!(
            clear,
            "no water within a square of the start: {wall:?} {half:?}"
        );
    }
}

/// Now and then a hamlet stands on open ground, whole: a well, houses round
/// it, some lit, and nothing growing in it. The seed puts the nearest one a
/// little way east of the start.
#[test]
fn a_hamlet_stands_east_of_the_start() {
    const WELL: [f32; 2] = [76.5, 21.5];
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 2);
    let near_well = |run: &Run, what: &str| -> Vec<[f32; 3]> {
        run.world
            .entities()
            .filter_map(|(entity, data)| {
                let texture = data.components.get("sindri.sprite")?["texture"].as_str()?;
                texture
                    .split('#')
                    .nth(1)?
                    .starts_with(what)
                    .then(|| run.world.world_transform(entity).expect("placed").position)
            })
            .filter(|at| (at[0] - WELL[0]).hypot(at[1] - WELL[1]) < 11.0)
            .collect()
    };
    assert!(near_well(&run, "well").is_empty(), "too far to be made yet");
    let wick = run.entity("wick").expect("Wick");
    let data = run.world.get_mut(wick).expect("Wick");
    let mut placed = data.transform_3d.expect("placed");
    placed.position = [WELL[0], WELL[1] - 3.0, 0.0];
    data.transform_3d = Some(placed);
    step(&mut run, 2);
    let wells = near_well(&run, "well");
    assert_eq!(wells.len(), 1, "a well in the middle");
    assert!((wells[0][0] - WELL[0]).abs() < 0.01 && (wells[0][1] - WELL[1]).abs() < 0.01);
    let houses: usize = ["bakery", "child-house", "workshop", "keeper"]
        .iter()
        .map(|house| near_well(&run, house).len())
        .sum();
    assert!(houses >= 1, "houses round it");
    let lit = run.world.entities().any(|(_, data)| {
        data.components
            .get("sindri.sprite")
            .and_then(|sprite| sprite["texture"].as_str())
            .is_some_and(|texture| texture.ends_with("-lit"))
            && data.components.contains_key("sindri.light2d")
    });
    assert!(lit, "someone is home in one of them");
    let wild = grown(&run)
        .into_iter()
        .filter(|(_, _, at)| (at[0] - WELL[0]).hypot(at[1] - WELL[1]) < 9.5)
        .count();
    assert_eq!(wild, 0, "nothing grows in the hamlet");
}

/// What lives in the air is scattered like what grows: fireflies over the
/// meadow round the start, each an emitter that glows in the dark.
#[test]
fn fireflies_glow_over_the_meadow() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 2);
    let fireflies: Vec<_> = run
        .world
        .entities()
        .filter(|(_, data)| {
            data.components
                .get("sindri.particles")
                .is_some_and(|particles| particles["glow"] == true && particles["max"] == 12)
        })
        .collect();
    assert!(
        fireflies.len() >= 3,
        "fireflies round the start: {}",
        fireflies.len()
    );
    step(&mut run, 120);
    let glowing = run
        .effects()
        .flecks()
        .iter()
        .filter(|fleck| fleck.glow)
        .count();
    assert!(glowing >= 6, "and they are in the air, glowing: {glowing}");
}

/// Wick leaves footprints as he walks, as far apart however fast he goes,
/// and none while he stands still; they fade behind him.
#[test]
fn wick_leaves_footprints_as_he_walks() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 30);
    let prints = |run: &Run| {
        run.effects()
            .flecks()
            .iter()
            .filter(|fleck| fleck.layer == -9)
            .count()
    };
    assert_eq!(prints(&run), 0, "standing still leaves none");
    run.input.apply(InputEvent::KeyPressed(Key::W));
    step(&mut run, 60);
    run.input.apply(InputEvent::KeyReleased(Key::W));
    let walked = prints(&run);
    assert!(walked >= 3, "a trail behind him: {walked}");
    step(&mut run, 60);
    assert_eq!(prints(&run), walked, "and no more once he stops");
    step(&mut run, 60 * 8);
    // A nudge from a gloom may lay a fresh print, but the trail is gone.
    let fresh = run
        .effects()
        .flecks()
        .iter()
        .filter(|fleck| fleck.layer == -9)
        .all(|fleck| fleck.remaining > 1.0);
    assert!(prints(&run) < walked && fresh, "the trail fades");
}
