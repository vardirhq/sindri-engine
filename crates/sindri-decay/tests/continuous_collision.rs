//! Spawn-window control and scene synchronization preserve live solver state.
use std::time::Duration;

use serde_json::json;
use sindri_core::{
    CommandBuffer, CommandHistory, EntityData, EntityId, SceneComponent, Transform3D, World,
    WorldCommand,
};
use sindri_decay::{Physics2d, ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_physics::RigidBody2d;
use sindri_platform::InputState;
use sindri_scene::{SceneExtractor, ScenePhysics2d};

const BODY: &str = "sindri.physics2d.rigid_body";
const STEP: Duration = Duration::from_nanos(16_666_667);

#[test]
fn spawn_window_and_undoable_ccd_edits_keep_velocity_and_joints() {
    let (extractor, mut world, entity, anchor) = authored_bodies();
    let registry = extractor.components();
    let mut physics = ScenePhysics2d::top_down().unwrap();
    let mut scripts = Scripts::new();
    let mut sources = ScriptSources::new();
    sources.insert(
        "ccd.decay",
        r"
        script Bullet {
            fn start() {
                Physics.set_continuous_collision(this.entity, true);
                Physics.set_velocity(this.entity, 7.0, 0.0);
                if !Physics.continuous_collision(this.entity) { this.transform.position.y = 99.0; }
            }
        }",
    );
    let input = InputState::default();
    let (backend, events) = physics.for_scripts();
    let report = scripts.advance(
        &mut world,
        registry,
        ScriptFrame::new(&sources, &input, 1.0 / 60.0).with_physics(Physics2d {
            world: backend,
            events,
        }),
    );
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert!(world.get(entity).unwrap().transform_3d.unwrap().position[1].abs() < f32::EPSILON);
    let authored: RigidBody2d =
        serde_json::from_value(world.get(entity).unwrap().components[BODY].clone()).unwrap();
    assert!(authored.continuous_collision);
    // Disable collisions with the anchor so its sole purpose is joint lifecycle.
    world
        .get_mut(anchor)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position[0] = 10.0;
    physics.step(&mut world, registry, STEP).unwrap();
    assert!(physics.world().continuous_collision(entity).unwrap());
    physics
        .world_mut()
        .connect_distance(entity, anchor, 100.0)
        .unwrap();
    let mut edited = world.get(entity).unwrap().components[BODY].clone();
    edited["continuous_collision"] = json!(false);
    registry.validate_payload(BODY, &edited).unwrap();
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetComponent {
        entity,
        type_name: BODY.into(),
        payload: edited,
    });
    let mut history = CommandHistory::default();
    history
        .apply(commands.into_transaction("Toggle CCD"), &mut world)
        .unwrap();
    physics.step(&mut world, registry, STEP).unwrap();
    assert!(!physics.world().continuous_collision(entity).unwrap());
    assert!((physics.world().linear_velocity(entity).unwrap()[0] - 7.0).abs() < 1.0e-3);
    assert_eq!(physics.world().joint_count(), 1);
    history.undo(&mut world).unwrap();
    physics.step(&mut world, registry, STEP).unwrap();
    assert!(physics.world().continuous_collision(entity).unwrap());
    assert!((physics.world().linear_velocity(entity).unwrap()[0] - 7.0).abs() < 1.0e-3);
    assert_eq!(physics.world().joint_count(), 1);
}

fn authored_bodies() -> (SceneExtractor, World, EntityId, EntityId) {
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.register::<ScriptComponent>("Script").unwrap();
    let registry = extractor.components();
    let mut payload = registry.default_payload(BODY).unwrap().clone();
    assert_eq!(payload["continuous_collision"], json!(false));
    // Prove old scenes can start a script before their body is synchronized.
    payload
        .as_object_mut()
        .unwrap()
        .remove("continuous_collision");
    let mut world = World::default();
    let entity = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [
            (BODY.to_owned(), payload),
            (
                "sindri.physics2d.collider".to_owned(),
                registry
                    .default_payload("sindri.physics2d.collider")
                    .unwrap()
                    .clone(),
            ),
            (
                ScriptComponent::TYPE_NAME.to_owned(),
                json!({"source": "ccd.decay", "script": "Bullet"}),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let anchor = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [(
            "sindri.physics2d.collider".to_owned(),
            registry
                .default_payload("sindri.physics2d.collider")
                .unwrap()
                .clone(),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    (extractor, world, entity, anchor)
}
