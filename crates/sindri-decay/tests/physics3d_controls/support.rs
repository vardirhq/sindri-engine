//! Typed 3D controls use the real scene driver and independent host context.

use std::{collections::BTreeSet, time::Duration};

use decay_ir::Path;
use decay_runtime::{Host, Value};
use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent, Transform3D, World};
use sindri_decay::{
    AudioQueue, Blackboard, HostServices, Physics3d, PrefabSources, ProfileSources,
    ScriptComponent, ScriptContext, Spawning, WorldHost,
};
use sindri_platform::InputState;
use sindri_scene::{Collider3dComponent, RigidBody3dComponent, SceneExtractor, ScenePhysics3d};

pub(super) struct Fixture {
    pub world: World,
    pub physics: ScenePhysics3d,
    pub extractor: SceneExtractor,
}

impl Fixture {
    pub fn new() -> Self {
        let mut extractor = SceneExtractor::new().unwrap();
        extractor.register::<ScriptComponent>("Script").unwrap();
        Self {
            world: World::default(),
            physics: ScenePhysics3d::new([0.0; 3]).unwrap(),
            extractor,
        }
    }

    pub fn actor(&mut self, kind: &str, sensor: bool) -> EntityId {
        let registry = self.extractor.components();
        let mut body = registry
            .default_payload(RigidBody3dComponent::TYPE_NAME)
            .unwrap()
            .clone();
        body["kind"] = json!(kind);
        let mut collider = registry
            .default_payload(Collider3dComponent::TYPE_NAME)
            .unwrap()
            .clone();
        collider["pieces"][0]["sensor"] = json!(sensor);
        self.world.spawn(EntityData {
            transform_3d: Some(Transform3D::default()),
            components: [
                (RigidBody3dComponent::TYPE_NAME.into(), body),
                (Collider3dComponent::TYPE_NAME.into(), collider),
            ]
            .into(),
            ..EntityData::default()
        })
    }

    pub fn step(&mut self) {
        self.physics
            .step(
                &mut self.world,
                self.extractor.components(),
                Duration::from_millis(10),
            )
            .unwrap();
    }

    pub fn call(
        &mut self,
        actor: EntityId,
        name: &str,
        args: &[Value],
        context: bool,
    ) -> Result<Value, String> {
        let input = InputState::default();
        let mut board = Blackboard::default();
        let mut audio = AudioQueue::default();
        let profiles = ProfileSources::new();
        let started = BTreeSet::new();
        let mut spawned = Vec::new();
        let (world, events) = self.physics.for_scripts();
        let mut host = WorldHost::new(
            &mut self.world,
            actor,
            ScriptContext {
                input: &input,
                delta_seconds: 0.01,
                elapsed_seconds: 0.0,
            },
            &mut board,
            HostServices {
                spawning: Spawning {
                    prefabs: PrefabSources::none(),
                    started: &started,
                    spawned: &mut spawned,
                },
                profiles: &profiles,
                physics: None,
                screen_ui: None,
                aim: None,
                gestures: None,
                camera_pan: None,
                random: None,
                saves: None,
                effects: None,
                animations: None,
                scenes: None,
                tile_sets: None,
                audio: &mut audio,
            },
        )
        .with_physics3d(context.then_some(Physics3d { world, events }));
        host.call(None, &Path(vec!["Physics3d".into(), name.into()]), args)
            .map_err(|failure| format!("{failure:?}"))?
            .ok_or_else(|| "unknown call".into())
    }
}

pub(super) fn reference(actor: EntityId) -> Value {
    Value::Reference(actor.to_bits())
}
pub(super) fn near(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 0.002, "{actual} != {expected}");
    }
}
