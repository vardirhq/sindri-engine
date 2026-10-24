//! The platformer played by a test: it stands on the ground it was painted
//! on, and a player running right and jumping at each gap and wall reaches
//! the flag.

use std::collections::BTreeSet;

use lamplighter::Run;
use sindri_core::EntityId;
use sindri_decay::AudioCommand;
use sindri_platform::{InputEvent, Key, MouseButton};
use sindri_scene::TilemapComponent;

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run) {
    let notes = run.step(STEP);
    assert!(notes.is_empty(), "the game reported: {notes:?}");
}

/// Entities of a scene entered after the first carry the scene name as a prefix.
fn find(run: &Run, id: &str) -> Option<EntityId> {
    let scene = run.session.scene().expect("a scene is playing");
    run.entity(&format!("{scene}/{id}")).or_else(|| run.entity(id))
}

/// Presses a drawn button as a player would.
fn press(run: &mut Run, name: &str) {
    let entity = find(run, name).unwrap_or_else(|| panic!("{name} is in the current scene"));
    let [x, y] = run.on_screen(entity).unwrap_or_else(|| panic!("{name} is drawn"));
    run.input.apply(InputEvent::PointerMoved { x, y });
    step(run);
    run.input.apply(InputEvent::ButtonPressed(MouseButton::Left));
    step(run);
    step(run);
    run.input.apply(InputEvent::ButtonReleased(MouseButton::Left));
    step(run);
    step(run);
}

/// Every clip the scripts asked to play since this was last asked.
fn listen(run: &mut Run, heard: &mut BTreeSet<String>, music: &mut BTreeSet<String>) {
    for command in run.session.take_audio_commands() {
        match command {
            AudioCommand::Play { clip, .. } => {
                heard.insert(clip);
            }
            AudioCommand::Loop { clip, .. } => {
                music.insert(clip);
            }
            _ => {}
        }
    }
}

/// The level's tiles, read from the scene as a player would see them.
fn level(run: &Run) -> (TilemapComponent, [f32; 2]) {
    let entity = find(run, "level").expect("the level");
    let tilemap = run
        .components()
        .get::<TilemapComponent>(&run.world, entity)
        .expect("a readable tilemap")
        .expect("the level is a tilemap");
    (tilemap, run.position(entity))
}

/// The height of the ground at a column, or `None` over a pit. Grass tufts
/// are decoration and do not count.
fn ground_at(tilemap: &TilemapComponent, corner: [f32; 2], column: i32) -> Option<f32> {
    let column = u32::try_from(column).ok()?;
    (0..tilemap.rows).find_map(|row| {
        let tile = tilemap.tile(column, row)?;
        let solid = tilemap.palette[tile as usize] != "tuft";
        #[allow(clippy::cast_precision_loss)]
        solid.then(|| corner[1] - row as f32)
    })
}

#[test]
fn the_hero_stands_on_the_painted_ground() {
    let mut run = Run::open().expect("the project opens");
    let hero = run.entity("hero").expect("the hero");
    for _ in 0..90 {
        step(&mut run);
    }
    assert!(run.physics().character_motion(hero).unwrap().grounded);
    let crate_entity = run.entity("wind-crate").unwrap();
    assert!(
        run.physics()
            .world()
            .continuous_collision(crate_entity)
            .unwrap()
    );
    let [x, y] = run.position(hero);
    // The first field's surface is at y = 3, and the hero's feet are half a
    // unit below its middle.
    assert!((y - 3.5).abs() < 0.08, "standing on the grass at {x}, {y}");
    assert!((run.board("coins_total") - 10.0).abs() < f32::EPSILON);
}

#[test]
fn a_player_running_and_jumping_reaches_the_beacon() {
    let mut run = Run::open_title().expect("the project opens");
    for _ in 0..3 {
        step(&mut run);
    }
    press(&mut run, "play");
    let hero = find(&run, "hero").expect("the hero");
    let (tilemap, corner) = level(&run);
    run.key(Key::ArrowRight, true);
    let mut holding_jump = 0;
    let mut heard = BTreeSet::new();
    let mut music = BTreeSet::new();
    for frame in 0..60 * 40 {
        step(&mut run);
        listen(&mut run, &mut heard, &mut music);
        if run.board("won") > 0.0 {
            assert!(run.board("falls") < 1.0, "it never fell");
            assert!(run.board("coins") >= 3.0, "and picked up coins on the way");
            // The sounds made in the Sound Lab: each one heard, and each one
            // a file the project ships rather than one only the editor finds.
            assert!(
                music.contains("audio/dusk-road.ogg"),
                "Dusk Road music looped: {music:?}"
            );
            let manifest = std::fs::read_to_string(lamplighter::project().join("sindri.toml"))
                .expect("the manifest");
            for clip in [
                "audio/jump.wav",
                "audio/land.wav",
                "audio/coin.wav",
                "audio/goal.wav",
            ] {
                assert!(heard.contains(clip), "{clip} was played: {heard:?}");
                assert!(lamplighter::project().join("assets").join(clip).is_file());
                assert!(manifest.contains(&format!("\"{clip}\"")), "{clip} ships");
            }
            assert!(lamplighter::project().join("assets/audio/dusk-road.ogg").is_file());
            assert!(manifest.contains("\"audio/dusk-road.ogg\""), "the music ships");
            eprintln!("reached the beacon after {:.1} s", f64::from(frame) / 60.0);
            for _ in 0..(60 * 3) {
                step(&mut run);
                listen(&mut run, &mut heard, &mut music);
            }
            assert_eq!(run.session.scene(), Some("levels.scene"));
            assert!(run.session.saves().flag("beacon_1", false));
            assert!(run.session.saves().number("embers_1", 0.0) >= 3.0);
            return;
        }
        if holding_jump > 0 {
            holding_jump -= 1;
            if holding_jump == 0 {
                run.key(Key::Space, false);
            }
            continue;
        }
        let [x, y] = run.position(hero);
        let settled = run
            .physics()
            .character_motion(hero)
            .is_some_and(|motion| motion.grounded);
        #[allow(clippy::cast_possible_truncation)]
        let column = (x - corner[0]).floor() as i32;
        let feet = y - 0.5;
        let underfoot = ground_at(&tilemap, corner, column);
        let blocked = (1..=2).any(|ahead| {
            ground_at(&tilemap, corner, column + ahead).is_none_or(|top| {
                top > feet + 0.5 || underfoot.is_some_and(|under| top < under - 0.5 && ahead == 1)
            })
        });
        if settled && blocked {
            run.key(Key::Space, true);
            holding_jump = 40;
        }
    }
    panic!(
        "the beacon was not reached; the hero is at {:?} with {} coins",
        run.position(hero),
        run.board("coins")
    );
}

/// Lighting the first lamp stores a safe respawn point and keeps it after a fall.
#[test]
fn a_lit_lantern_is_the_respawn_checkpoint() {
    let mut run = Run::open().expect("the level opens");
    let hero = run.entity("hero").expect("the hero");
    {
        let data = run.world.get_mut(hero).expect("the hero data");
        let position = &mut data.transform_3d.as_mut().expect("the hero transform").position;
        position[0] = 9.0;
        position[1] = 4.0;
    }
    for _ in 0..90 {
        step(&mut run);
    }

    run.key(Key::ArrowRight, true);
    let mut lit = false;
    for _ in 0..180 {
        step(&mut run);
        if run.board("checkpoint_index") >= 1.0 {
            lit = true;
            break;
        }
    }
    run.key(Key::ArrowRight, false);
    assert!(lit, "Wick touched the first checkpoint lantern");
    assert!((run.board("checkpoint_x") - 10.5).abs() < 0.01);

    run.world
        .get_mut(hero)
        .expect("the hero data")
        .transform_3d
        .as_mut()
        .expect("the hero transform")
        .position = [15.0, -4.0, 0.5];
    for _ in 0..90 {
        step(&mut run);
    }
    assert!((run.board("falls") - 1.0).abs() < f32::EPSILON, "the fall was counted");
    let [x, y] = run.position(hero);
    assert!((x - 10.5).abs() < 0.05, "respawned at the lit lantern: {x}");
    assert!((y - 3.5).abs() < 0.05, "stands safely at the checkpoint: {y}");
}

/// The camera follows the hero along the level, and stays inside its bounds.
#[test]
fn the_camera_follows_the_hero() {
    let mut run = Run::open().expect("the project opens");
    let camera = run.entity("camera").expect("the camera");
    let hero = run.entity("hero").expect("the hero");
    run.key(Key::ArrowRight, true);
    let mut furthest = 0.0_f32;
    // Give the low steps and boardwalk time to be traversed before checking
    // that the camera has left its initial left-bound clamp.
    for _ in 0..120 {
        step(&mut run);
        let [x, y] = run.position(camera);
        furthest = furthest.max(x);
        assert!(
            y >= 4.5 - 1.0e-3 && x >= 8.0 - 1.0e-3,
            "inside its bounds: {x}, {y}"
        );
    }
    let hero_x = run.position(hero)[0];
    assert!(
        furthest > 10.0 && (furthest - hero_x).abs() < 2.0,
        "it followed the hero to {hero_x}: {furthest}"
    );
}

/// The ray sees the painted colliders while excluding the hero's compound and
/// coin sensors. Standing, jumping and landing change the player's HUD probe.
#[test]
fn ground_clearance_probe_measures_the_real_floor_through_a_jump() {
    let mut run = Run::open().unwrap();
    for _ in 0..90 {
        step(&mut run);
    }
    assert!(run.board("ground_clearance") < 0.03);
    let label = run.entity("ground-clearance").unwrap();
    let text = &run.world.get(label).unwrap().components["sindri.ui.text"]["text"];
    assert!(text.as_str().unwrap().starts_with("Ground: 0.0"));
    run.key(Key::Space, true);
    for _ in 0..15 {
        step(&mut run);
    }
    assert!(run.board("ground_clearance") > 1.0);
    run.key(Key::Space, false);
    for _ in 0..120 {
        step(&mut run);
    }
    assert!(run.board("ground_clearance") < 0.03);
}
