use std::{collections::BTreeSet, time::Duration};

use decay_ir::Path;
use decay_runtime::{Host, Value};
use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent, Transform3D, World};
use sindri_decay::{
    AudioQueue, Blackboard, Characters3d, HostServices, Physics3d, PrefabSources, ProfileSources,
    ScriptComponent, ScriptContext, ScriptFrame, ScriptReport, ScriptSources, Scripts, Spawning,
    WorldHost,
};
use sindri_platform::InputState;
use sindri_scene::{Character3dComponent, SceneExtractor, ScenePhysics3d};

pub const STEP: Duration = Duration::from_nanos(16_666_667);

pub struct Fixture {
    pub world: World,
    pub physics: ScenePhysics3d,
    pub extractor: SceneExtractor,
    pub sources: ScriptSources,
    pub scripts: Scripts,
}
impl Fixture {
    pub fn new() -> Self {
        let mut extractor = SceneExtractor::new().unwrap();
        extractor.register::<ScriptComponent>("Script").unwrap();
        Self {
            world: World::default(),
            physics: ScenePhysics3d::new([0.0; 3]).unwrap(),
            extractor,
            sources: ScriptSources::new(),
            scripts: Scripts::new(),
        }
    }
    pub fn actor(&mut self, at: [f32; 3]) -> EntityId {
        let actor = self.solid("Actor", at, [0.5; 3]);
        self.world.get_mut(actor).unwrap().components.insert(
            Character3dComponent::TYPE_NAME.into(),
            json!({"snap_distance": 0.2}),
        );
        actor
    }
    pub fn solid(&mut self, name: &str, at: [f32; 3], size: [f32; 3]) -> EntityId {
        let mut collider = self
            .extractor
            .components()
            .default_payload("sindri.physics3d.collider")
            .unwrap()
            .clone();
        collider["pieces"][0]["shape"]["half_extents"] = json!(size);
        self.world.spawn(EntityData {
            name: Some(name.into()),
            transform_3d: Some(Transform3D {
                position: at,
                ..Transform3D::default()
            }),
            components: [("sindri.physics3d.collider".into(), collider)]
                .into_iter()
                .collect(),
            ..EntityData::default()
        })
    }
    pub fn script(&mut self, entity: EntityId, name: &str, source: &str) {
        self.sources.insert("controller.decay", source);
        self.world.get_mut(entity).unwrap().components.insert(
            ScriptComponent::TYPE_NAME.into(),
            json!({"source": "controller.decay", "script": name}),
        );
    }
    pub fn advance(&mut self) -> ScriptReport {
        let input = InputState::default();
        let (world, events, requests, motions) = self.physics.for_scripts_with_characters();
        self.scripts.advance(
            &mut self.world,
            self.extractor.components(),
            ScriptFrame::new(&self.sources, &input, 1.0 / 60.0)
                .with_physics3d(Physics3d { world, events })
                .with_characters3d(Characters3d { requests, motions }),
        )
    }
    pub fn step(&mut self) {
        self.physics
            .step(&mut self.world, self.extractor.components(), STEP)
            .unwrap();
    }
    pub fn call(
        &mut self,
        entity: EntityId,
        name: &str,
        args: &[Value],
        characters: bool,
    ) -> Result<Value, String> {
        let input = InputState::default();
        let mut board = Blackboard::default();
        let mut audio = AudioQueue::default();
        let profiles = ProfileSources::new();
        let started = BTreeSet::new();
        let mut spawned = Vec::new();
        let (world, events, requests, motions) = self.physics.for_scripts_with_characters();
        let mut host = WorldHost::new(
            &mut self.world,
            entity,
            ScriptContext {
                input: &input,
                delta_seconds: 1.0 / 60.0,
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
                gestures: None,
                camera_pan: None,
                aim: None,
                random: None,
                saves: None,
                effects: None,
                animations: None,
                tile_sets: None,
                audio: &mut audio,
                scenes: None,
            },
        )
        .with_physics3d(Some(Physics3d { world, events }))
        .with_characters3d(characters.then_some(Characters3d { requests, motions }));
        host.call(None, &Path(vec!["Physics3d".into(), name.into()]), args)
            .map_err(|failure| format!("{failure:?}"))?
            .ok_or_else(|| "unknown call".into())
    }
    pub fn field(&self, entity: EntityId, name: &str) -> &Value {
        self.scripts.field(entity, name).unwrap()
    }
}
pub fn reference(entity: EntityId) -> Value {
    Value::Reference(entity.to_bits())
}
pub fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.002, "{actual} != {expected}");
}
pub fn number(value: &Value) -> f32 {
    let Value::Number(value) = value else {
        panic!("expected number, got {value:?}")
    };
    // Test observations originate in backend f32 values.
    #[allow(clippy::cast_possible_truncation)]
    {
        *value as f32
    }
}
