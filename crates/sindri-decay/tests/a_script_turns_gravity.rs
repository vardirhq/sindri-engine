//! A script reads and changes which way is down through the scene's authored
//! Physics 2D World, and the solver follows it.

use std::time::Duration;

use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent, Transform3D, World};
use sindri_decay::{Physics2d, ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{SceneExtractor, ScenePhysics2d};

const STEP: Duration = Duration::from_nanos(16_666_667);

struct Room {
    extractor: SceneExtractor,
    world: World,
    physics: ScenePhysics2d,
    scripts: Scripts,
    sources: ScriptSources,
    crate_box: EntityId,
}

fn settings(world: &mut World, gravity: [f32; 2], disabled: bool) -> EntityId {
    world.spawn(EntityData {
        disabled,
        components: [(
            "sindri.physics2d.world".to_owned(),
            json!({"gravity": gravity, "layers": ["ground"]}),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    })
}

fn room(source: &str, worlds: &[([f32; 2], bool)]) -> Room {
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.register::<ScriptComponent>("Script").unwrap();
    let mut world = World::default();
    for &(gravity, disabled) in worlds {
        settings(&mut world, gravity, disabled);
    }
    let registry = extractor.components();
    let mut floor = registry
        .default_payload("sindri.physics2d.collider")
        .unwrap()
        .clone();
    floor["pieces"][0]["shape"]["half_extents"] = json!([5.0, 0.5]);
    world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [0.0, -0.5, 0.0],
            ..Transform3D::default()
        }),
        components: [("sindri.physics2d.collider".to_owned(), floor)]
            .into_iter()
            .collect(),
        ..EntityData::default()
    });
    let crate_box = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [0.0, 1.0, 0.0],
            ..Transform3D::default()
        }),
        components: [
            (
                "sindri.physics2d.rigid_body".to_owned(),
                registry
                    .default_payload("sindri.physics2d.rigid_body")
                    .unwrap()
                    .clone(),
            ),
            (
                "sindri.physics2d.collider".to_owned(),
                registry
                    .default_payload("sindri.physics2d.collider")
                    .unwrap()
                    .clone(),
            ),
            (
                ScriptComponent::TYPE_NAME.to_owned(),
                json!({"source": "gravity.decay", "script": "Turn"}),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert("gravity.decay", source);
    Room {
        extractor,
        world,
        physics: ScenePhysics2d::top_down().unwrap(),
        scripts: Scripts::new(),
        sources,
        crate_box,
    }
}

impl Room {
    fn step(&mut self) {
        self.physics
            .step(&mut self.world, self.extractor.components(), STEP)
            .unwrap();
    }

    fn script(&mut self) -> Vec<String> {
        let input = InputState::default();
        let (backend, events) = self.physics.for_scripts();
        let report = self.scripts.advance(
            &mut self.world,
            self.extractor.components(),
            ScriptFrame::new(&self.sources, &input, 1.0 / 60.0).with_physics(Physics2d {
                world: backend,
                events,
            }),
        );
        report.failures.iter().map(ToString::to_string).collect()
    }

    fn height(&self) -> f32 {
        self.world
            .get(self.crate_box)
            .unwrap()
            .transform_3d
            .unwrap()
            .position[1]
    }
}

#[test]
fn a_resting_crate_falls_up_when_a_script_turns_gravity_over() {
    let mut room = room(
        r#"
        script Turn {
            fn start() {
                let down = Physics.gravity();
                Game.set("down", down.y);
                Physics.set_gravity(Vec2(down.x, 0.0 - down.y));
                Game.set("up", Physics.gravity().y);
            }
        }"#,
        &[([0.0, 9.0], true), ([0.0, -12.0], false)],
    );
    // Let the crate settle and sleep before anything changes.
    for _ in 0..600 {
        room.step();
    }
    let resting = room.height();
    let failures = room.script();
    assert!(failures.is_empty(), "{failures:?}");
    let board = room.scripts.blackboard();
    assert!(
        (board.get("down", 0.0) + 12.0).abs() < 1.0e-6,
        "reads the active world"
    );
    assert!(
        (board.get("up", 0.0) - 12.0).abs() < 1.0e-6,
        "reads back what it set"
    );
    for _ in 0..30 {
        room.step();
    }
    assert!(
        room.height() > resting + 1.0,
        "{resting} -> {}",
        room.height()
    );
}

#[test]
fn turning_gravity_refuses_infinity_and_a_scene_without_world_settings() {
    for (worlds, source, needle) in [
        (
            vec![([0.0, -9.81], false)],
            "script Turn { fn start() { Physics.set_gravity(Vec2(1.0 / 0.0, 0.0)); } }",
            "finite",
        ),
        (
            vec![([0.0, -9.81], true)],
            "script Turn { fn start() { Physics.set_gravity(Vec2(0.0, 1.0)); } }",
            "Physics 2D World",
        ),
    ] {
        let mut room = room(source, &worlds);
        room.step();
        let failures = room.script();
        assert!(
            failures.iter().any(|failure| failure.contains(needle)),
            "{failures:?}"
        );
    }
}
