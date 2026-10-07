use std::{collections::BTreeSet, time::Duration};

use decay_ir::Path;
use decay_runtime::{Host, Value};
use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent, Transform3D, World};
use sindri_decay::{
    AudioQueue, Blackboard, Characters2d, HostServices, Physics2d, PrefabSources, ProfileSources,
    ScriptComponent, ScriptContext, ScriptFrame, ScriptReport, ScriptSources, Scripts, Spawning,
    WorldHost,
};
use sindri_platform::InputState;
use sindri_scene::{Character2dComponent, SceneExtractor, ScenePhysics2d};

pub const STEP: Duration = Duration::from_nanos(16_666_667);

pub struct Fixture {
    pub world: World,
    pub physics: ScenePhysics2d,
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
            physics: ScenePhysics2d::top_down().unwrap(),
            extractor,
            sources: ScriptSources::new(),
            scripts: Scripts::new(),
        }
    }
    pub fn actor(&mut self, at: [f32; 2]) -> EntityId {
        let actor = self.solid("Actor", at, [0.5; 2]);
        self.world.get_mut(actor).unwrap().components.insert(
            Character2dComponent::TYPE_NAME.into(),
            json!({"snap_distance": 0.2}),
        );
        actor
    }
    pub fn solid(&mut self, name: &str, at: [f32; 2], size: [f32; 2]) -> EntityId {
        self.world.spawn(EntityData {
            name: Some(name.into()),
            transform_3d: Some(Transform3D {
                position: [at[0], at[1], 0.0],
                ..Transform3D::default()
            }),
            components: [("sindri.physics2d.collider".into(), json!({"shape": {"shape": "box", "half_extents": size}, "offset": [0.0, 0.0], "rotation": 0.0, "sensor": false, "layers": {"memberships": 1, "filter": 1}, "friction": 0.0, "restitution": 0.0}))].into_iter().collect(),
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
                .with_physics(Physics2d { world, events })
                .with_characters(Characters2d { requests, motions }),
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
                physics: Some(Physics2d { world, events }),
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
        .with_characters(characters.then_some(Characters2d { requests, motions }));
        host.call(None, &Path(vec!["Physics".into(), name.into()]), args)
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
