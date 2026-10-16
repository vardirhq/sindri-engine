//! Open the real feature project, run its Decay clock and observe frame state.
use sindri_causeway::project::{ProjectPlayer, open};
use sindri_core::{EntityId, World};
use sindri_runtime::STEP;
use std::{collections::BTreeSet, path::Path};

fn entity(world: &World, name: &str) -> EntityId {
    world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some(name))
        .unwrap()
        .0
}
fn step(player: &mut ProjectPlayer) -> Vec<String> {
    let report = player
        .session
        .step(&mut player.world, &player.input, (960.0, 540.0), STEP)
        .unwrap();
    assert!(
        report.scripts.failures.is_empty(),
        "{:?}",
        report.scripts.failures
    );
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    report
        .scripts
        .printed
        .into_iter()
        .map(|line| line.message)
        .collect()
}
fn project() -> sindri_causeway::project::OpenedProject {
    open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/daynight"),
        [960.0, 540.0],
    )
    .unwrap()
}

#[test]
fn a_full_day_reaches_every_phase_and_keeps_running_after_the_goal() {
    play_day(12, 6);
    play_day(2, 12);
}

fn play_day(day_seconds: u32, start_hour: u32) {
    let mut opened = project();
    let player = &mut opened.player;
    let camera = entity(&player.world, "Camera");
    let properties = &mut player
        .world
        .get_mut(camera)
        .unwrap()
        .components
        .get_mut("sindri.script")
        .unwrap()["properties"];
    properties["day_seconds"] = serde_json::json!(day_seconds);
    properties["start_hour"] = serde_json::json!(start_hour);
    let carrier = entity(&player.world, "Carrier");
    let original = player.world.world_transform(carrier).unwrap();
    let mut seen = BTreeSet::new();
    let mut goals = 0;
    let mut night_seen = false;
    for _ in 0..800 {
        for message in step(player) {
            if message == "Day/night ready" {
                goals += 1;
            }
            seen.insert(message);
        }
        let lighting = player.scene.lighting(&player.world).unwrap();
        if player.scene.sun(&player.world).unwrap().is_none() {
            let locals: Vec<_> = lighting.local_lights.iter().flatten().collect();
            if locals.len() == 2 {
                night_seen = true;
                for name in ["Work spot", "Blue point"] {
                    let id = entity(&player.world, name);
                    let pose = player.world.world_transform(id).unwrap();
                    assert!(locals.iter().any(|light| {
                        light
                            .point
                            .position
                            .into_iter()
                            .zip(pose.position)
                            .all(|(a, b)| (a - b).abs() < 1e-5)
                    }));
                    assert!(player.world.is_active(id));
                }
            }
        }
    }
    for phase in ["sunrise", "midday", "sunset", "night"] {
        assert!(seen.contains(&format!("Day/night {phase}")), "{seen:?}");
    }
    assert_eq!(goals, 1);
    assert!(night_seen);
    let finished = player.world.world_transform(carrier).unwrap();
    assert_ne!(original, finished);
    let sun = entity(&player.world, "Sun");
    let previous_sun = player.world.world_transform(sun).unwrap();
    for _ in 0..30 {
        assert!(!step(player).iter().any(|line| line == "Day/night ready"));
    }
    assert_ne!(finished, player.world.world_transform(carrier).unwrap());
    assert_ne!(previous_sun, player.world.world_transform(sun).unwrap());
    let frame = player
        .scene
        .extract(
            &player.world,
            sindri_render::Viewport::new(960, 540),
            sindri_scene::CameraView::default(),
            &sindri_scene::TextureBindings::new(),
        )
        .unwrap();
    assert_eq!(
        frame
            .passes()
            .iter()
            .filter(|pass| matches!(pass.command, sindri_render::FrameCommand::Model { .. }))
            .count(),
        2
    );
}

#[test]
fn frozen_hours_isolate_sun_direction_and_night_illumination() {
    let mut directions = Vec::new();
    let mut poses = Vec::new();
    for (hour, night_lights) in [(10, true), (14, true), (0, true), (0, false)] {
        let mut opened = project();
        let player = &mut opened.player;
        let camera = entity(&player.world, "Camera");
        let properties = &mut player
            .world
            .get_mut(camera)
            .unwrap()
            .components
            .get_mut("sindri.script")
            .unwrap()["properties"];
        properties["preview_hour"] = serde_json::json!(hour);
        properties["move_carrier"] = serde_json::json!(false);
        properties["night_lights"] = serde_json::json!(night_lights);
        assert!(step(player).iter().any(|line| line == "Day/night ready"));
        let sun = player.scene.sun(&player.world).unwrap();
        assert_eq!(sun.is_some(), hour != 0);
        let lights = player.scene.lighting(&player.world).unwrap();
        if hour != 0 {
            directions.push(sun.unwrap().direction);
        }
        assert_eq!(
            lights.local_lights.iter().flatten().count(),
            if hour == 0 && night_lights { 2 } else { 0 }
        );
        poses.push(
            player
                .world
                .world_transform(entity(&player.world, "Carrier"))
                .unwrap(),
        );
    }
    assert!(poses.windows(2).all(|pair| pair[0] == pair[1]));
    assert!(
        directions[0]
            .into_iter()
            .zip(directions[1])
            .any(|(a, b)| (a - b).abs() > 0.1)
    );
}
