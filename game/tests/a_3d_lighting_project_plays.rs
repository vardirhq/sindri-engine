//! The feature example drives real lights through Session before frame extraction.
use sindri_causeway::project::{ProjectPlayer, open};
use sindri_core::{EntityId, World};
use sindri_runtime::STEP;
use sindri_scene::LightComponent;
use std::path::Path;

fn entity(world: &World, name: &str) -> EntityId {
    world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some(name))
        .unwrap()
        .0
}
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
        .any(|line| line.message == "3D lighting ready")
}

fn check_geometry(player: &ProjectPlayer) {
    use sindri_render::{FrameCommand, TextureId, Viewport};
    use sindri_scene::{CameraView, TextureBindings};
    let mut bindings = TextureBindings::new();
    for (name, index) in [("cube", 0), ("surface", 1), ("floor", 2)] {
        bindings.bind(format!("textures/{name}.png"), TextureId::new(index));
    }
    let frame = player
        .scene
        .extract(
            &player.world,
            Viewport::new(960, 540),
            CameraView::default(),
            &bindings,
        )
        .unwrap();
    let mut counts = [0; 3];
    for pass in frame.passes() {
        match pass.command {
            FrameCommand::TexturedCube { .. } => counts[0] += 1,
            FrameCommand::RetainedTexturedMesh { .. } => counts[1] += 1,
            FrameCommand::Model { .. } => counts[2] += 1,
            _ => panic!("unexpected 3D lab geometry"),
        }
    }
    assert_eq!(counts, [4, 3, 2]);
    assert_eq!(
        frame
            .passes()
            .iter()
            .filter(|p| p.sidedness == sindri_render::MaterialSidedness::Double)
            .count(),
        1
    );
    assert_eq!(
        frame
            .passes()
            .iter()
            .filter(|p| p.sidedness == sindri_render::MaterialSidedness::Single)
            .count(),
        1
    );
    let environment = player.scene.environment(&player.world).unwrap().unwrap();
    assert!((environment.ambient_intensity - 0.025).abs() < 1e-6);
    assert_close(environment.ambient_color.map(f64::from), [1.0; 3]);
    assert_close(
        environment.background.map(f64::from),
        sindri_scene::EnvironmentComponent::default()
            .background
            .map(f64::from),
    );
    assert_close(frame.clear().color, environment.background.map(f64::from));
}

#[test]
fn lighting_lab_alternates_composed_lights_and_reaches_both_control_goals() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/lighting3d");
    for lit in [true, false] {
        let mut opened = open(&root, [960.0, 540.0]).unwrap();
        let player = &mut opened.player;
        let camera = entity(&player.world, "Camera");
        player
            .world
            .get_mut(camera)
            .unwrap()
            .components
            .get_mut("sindri.script")
            .unwrap()["properties"]["lit"] = serde_json::json!(lit);
        let sun = entity(&player.world, "Sun");
        let rig = entity(&player.world, "Light carrier");
        let original_sun = player.world.world_transform(sun).unwrap();
        let original_rig = player.world.world_transform(rig).unwrap();
        let mut ready = false;
        for i in 0..240 {
            ready |= step(player);
            if i == 5 || i == 85 {
                check_environment(player, i == 85);
                let lights = player.scene.lighting(&player.world).unwrap();
                let locals: Vec<_> = lights.local_lights.iter().flatten().collect();
                assert_eq!(locals.len(), 1);
                assert_eq!(locals[0].spot.is_some(), i == 85);
                let source = entity(
                    &player.world,
                    if i == 85 {
                        "Warm spotlight"
                    } else {
                        "Blue point"
                    },
                );
                let expected = player.world.world_transform(source).unwrap().position;
                assert!(
                    locals[0]
                        .point
                        .position
                        .into_iter()
                        .zip(expected)
                        .all(|(a, b)| (a - b).abs() < 1e-5)
                );
                assert!(lights.directional_intensity > 0.03);
                assert_ne!(player.world.world_transform(sun).unwrap(), original_sun);
                assert_ne!(player.world.world_transform(rig).unwrap(), original_rig);
                if i == 85 {
                    let direction = locals[0].spot.unwrap().direction;
                    assert!(
                        direction[0].abs() + direction[2].abs() > 0.01,
                        "parent pitch changes mounted direction"
                    );
                }
            }
        }
        assert!(ready);
        check_geometry(player);
        let lights = player.scene.lighting(&player.world).unwrap();
        assert_eq!(
            lights.local_lights.iter().flatten().count(),
            if lit { 2 } else { 0 }
        );
        assert_eq!(player.scene.sun(&player.world).unwrap().is_some(), lit);
        assert!(player.world.is_active(camera));
        for name in ["Warm spotlight", "Blue point"] {
            let id = entity(&player.world, name);
            let light: LightComponent = serde_json::from_value(
                player.world.get(id).unwrap().components["sindri.light"].clone(),
            )
            .unwrap();
            assert_eq!(light.enabled, lit);
            assert!((light.intensity - 24.0).abs() < 1e-6);
            assert!(player.world.is_active(id));
        }
        let settled = player.world.world_transform(rig).unwrap();
        for _ in 0..30 {
            assert!(!step(player));
        }
        assert_eq!(player.world.world_transform(rig).unwrap(), settled);
    }
}

fn check_environment(player: &ProjectPlayer, warm: bool) {
    use sindri_render::Viewport;
    use sindri_scene::{CameraView, TextureBindings};
    let environment = player.scene.environment(&player.world).unwrap().unwrap();
    let lighting = player.scene.lighting(&player.world).unwrap();
    let expected = if warm {
        [1.0, 0.25, 0.1]
    } else {
        [0.1, 0.2, 1.0]
    };
    assert_close(environment.ambient_color.map(f64::from), expected);
    assert_close(lighting.ambient_color.map(f64::from), expected);
    let intensity = if warm { 0.3 } else { 0.15 };
    assert!((lighting.ambient_intensity - intensity).abs() < 1e-6);
    let frame = player
        .scene
        .extract(
            &player.world,
            Viewport::new(960, 540),
            CameraView::default(),
            &TextureBindings::new(),
        )
        .unwrap();
    assert_close(frame.clear().color, environment.background.map(f64::from));
    assert!((environment.background[0] - if warm { 0.12 } else { 0.015 }).abs() < 1e-6);
}

fn assert_close<const N: usize>(actual: [f64; N], expected: [f64; N]) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 1e-6),
        "{actual:?} != {expected:?}"
    );
}
