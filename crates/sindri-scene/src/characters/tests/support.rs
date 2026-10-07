use crate::{Character2dComponent, SceneExtractor, ScenePhysics2d};
use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use std::time::Duration;

pub const STEP: Duration = Duration::from_nanos(16_666_667);
pub struct Fixture {
    pub world: World,
    pub physics: ScenePhysics2d,
    pub components: ComponentSchemaRegistry,
}
impl Fixture {
    pub fn new() -> Self {
        Self {
            world: World::default(),
            physics: ScenePhysics2d::top_down().unwrap(),
            components: SceneExtractor::new().unwrap().components().clone(),
        }
    }
    pub fn empty(&mut self, at: [f32; 2]) -> EntityId {
        self.world.spawn(EntityData {
            transform_3d: Some(Transform3D {
                position: [at[0], at[1], 0.0],
                ..Transform3D::default()
            }),
            ..EntityData::default()
        })
    }
    pub fn character(&mut self, at: [f32; 2]) -> EntityId {
        let entity = self.empty(at);
        let data = self.world.get_mut(entity).unwrap();
        data.components
            .insert(Character2dComponent::TYPE_NAME.into(), json!({}));
        data.components.insert("sindri.physics2d.collider".into(), json!({"shape": {"shape": "circle", "radius": 0.5}, "offset": [0.0, 0.0], "rotation": 0.0, "sensor": false, "layers": {"memberships": 1, "filter": 1}, "friction": 0.0, "restitution": 0.0}));
        entity
    }
    pub fn box_collider(&mut self, at: [f32; 2], extents: [f32; 2]) -> EntityId {
        let entity = self.character(at);
        let data = self.world.get_mut(entity).unwrap();
        data.components.remove(Character2dComponent::TYPE_NAME);
        data.components
            .get_mut("sindri.physics2d.collider")
            .unwrap()["shape"] = json!({"shape": "box", "half_extents": extents});
        entity
    }
    pub fn body(&mut self, entity: EntityId, kind: &str, velocity: [f32; 2]) {
        let mut body = self
            .components
            .default_payload("sindri.physics2d.rigid_body")
            .unwrap()
            .clone();
        body["kind"] = json!(kind);
        body["linear_velocity"] = json!(velocity);
        self.world
            .get_mut(entity)
            .unwrap()
            .components
            .insert("sindri.physics2d.rigid_body".into(), body);
    }
    pub fn set_position(&mut self, entity: EntityId, at: [f32; 2]) {
        self.world
            .get_mut(entity)
            .unwrap()
            .transform_3d
            .as_mut()
            .unwrap()
            .set_position_2d(at);
    }
    pub fn queue(&mut self, entity: EntityId, displacement: [f32; 2], snap: bool) {
        self.physics
            .character_requests()
            .move_character(entity, displacement, snap)
            .unwrap();
    }
    pub fn step(&mut self) {
        self.physics
            .step(&mut self.world, &self.components, STEP)
            .unwrap();
    }
}
pub fn position(world: &World, entity: EntityId) -> [f32; 2] {
    world.world_transform(entity).unwrap().position_2d()
}
pub fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.002, "{actual} != {expected}");
}
