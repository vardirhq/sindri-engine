//! The measured fixture remains a real loadable, scripted project.

use std::{path::Path, sync::Arc};

use sindri_causeway::project::{ProjectPlayer, open};
use sindri_render::{FrameCommand, TextureId, Viewport};
use sindri_runtime::STEP;
use sindri_scene::{CameraView, TextureBindings};

fn step(player: &mut ProjectPlayer) -> bool {
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
        .iter()
        .any(|line| line.message == "3D benchmark ready")
}

#[test]
fn the_fixed_workload_reaches_ready_and_extracts_all_three_geometry_kinds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/benchmark3d");
    let mut opened = open(&root, [960.0, 540.0]).unwrap();
    let player = &mut opened.player;
    let mut ready = false;
    for _ in 0..30 {
        ready |= step(player);
    }
    assert!(
        ready,
        "the scene and script reach the benchmark's ready goal"
    );
    let lighting = player.scene.lighting(&player.world).unwrap();
    assert!(
        player
            .scene
            .environment(&player.world)
            .unwrap()
            .unwrap()
            .shadows
            .enabled
    );
    assert!(lighting.directional_intensity > 0.0);
    let poses: Vec<_> = player
        .world
        .entities()
        .map(|(id, data)| (id, data.transform_3d))
        .collect();
    for _ in 0..60 {
        assert!(!step(player), "ready is announced once");
    }
    assert_eq!(
        poses,
        player
            .world
            .entities()
            .map(|(id, data)| (id, data.transform_3d))
            .collect::<Vec<_>>()
    );
    let mut textures = TextureBindings::new();
    for (name, index) in [("cube", 0), ("surface", 1), ("floor", 2)] {
        textures.bind(format!("textures/{name}.png"), TextureId::new(index));
    }
    let frame = player
        .scene
        .extract(
            &player.world,
            Viewport::new(960, 540),
            CameraView::default(),
            &textures,
        )
        .unwrap();
    let mut counts = [0; 3];
    let mut models = Vec::new();
    for pass in frame.passes() {
        match &pass.command {
            FrameCommand::TexturedCube { .. } => counts[0] += 1,
            FrameCommand::RetainedTexturedMesh { geometry, .. } => {
                counts[1] += 1;
                assert_eq!(geometry.vertices().len(), 5);
                assert_eq!(geometry.indices().len(), 18);
            }
            FrameCommand::Model { asset, .. } => {
                counts[2] += 1;
                models.push(asset);
            }
            _ => panic!("unexpected command in the 3D workload"),
        }
    }
    assert_eq!(counts, [101, 100, 100], "300 props and the cube floor");
    assert!(
        models.iter().all(|asset| Arc::ptr_eq(models[0], asset)),
        "repeated models share one decoded resource"
    );
}
