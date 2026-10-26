//! Runs the same emitter and pool as a Sindri scene.
use sindri_core::{EntityData, EntityId, SceneComponent, Transform3D, World};
use sindri_scene::{Effects2d, ParticlesComponent, SceneExtractor};
use std::time::Duration;

pub struct Preview {
    pub effects: Effects2d,
    world: World,
    entity: EntityId,
    scene: SceneExtractor,
}
impl Preview {
    pub fn new(component: &ParticlesComponent) -> Result<Self, String> {
        let mut world = World::default();
        let entity = world.spawn(EntityData {
            transform_3d: Some(Transform3D::default()),
            components: [(
                ParticlesComponent::TYPE_NAME.to_owned(),
                serde_json::to_value(component).map_err(|e| e.to_string())?,
            )]
            .into_iter()
            .collect(),
            ..EntityData::default()
        });
        Ok(Self {
            effects: Effects2d::default(),
            world,
            entity,
            scene: SceneExtractor::new().map_err(|e| e.to_string())?,
        })
    }
    pub fn step(&mut self, component: &ParticlesComponent, delta: Duration) -> Result<(), String> {
        component.validate().map_err(|e| e.to_string())?;
        let data = self
            .world
            .get_mut(self.entity)
            .ok_or("preview entity missing")?;
        data.components.insert(
            ParticlesComponent::TYPE_NAME.to_owned(),
            serde_json::to_value(component).map_err(|e| e.to_string())?,
        );
        self.effects.advance(delta);
        let problems = self
            .effects
            .emit(&self.world, self.scene.components(), delta)
            .map_err(|e| e.to_string())?;
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.join("; "))
        }
    }
    pub fn burst(&mut self, component: &ParticlesComponent, count: u32) -> Result<(), String> {
        component.validate().map_err(|e| e.to_string())?;
        self.effects
            .emit_now(self.entity, component, [0.0, 0.0], count);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets;

    #[test]
    fn every_preset_is_valid_and_emits_with_the_real_engine() {
        for index in 0..presets::NAMES.len() {
            let component = presets::preset(index);
            component.validate().unwrap();
            let mut preview = Preview::new(&component).unwrap();
            for _ in 0..60 {
                preview.step(&component, Duration::from_millis(16)).unwrap();
            }
            assert!(preview.effects.live() > 0, "{}", presets::NAMES[index]);
        }
    }

    #[test]
    fn stop_keeps_live_particles_and_burst_respects_cap() {
        let mut component = presets::preset(0);
        component.max = 20;
        let mut preview = Preview::new(&component).unwrap();
        preview.burst(&component, 200).unwrap();
        assert_eq!(preview.effects.live(), 20);
        component.playing = false;
        preview.step(&component, Duration::from_millis(16)).unwrap();
        assert_eq!(preview.effects.live(), 20);
        preview.step(&component, Duration::from_secs(10)).unwrap();
        assert_eq!(preview.effects.live(), 0);
    }

    #[test]
    fn saved_emitter_round_trips_without_losing_fields() {
        let component = presets::preset(4);
        let json = serde_json::to_string(&component).unwrap();
        let restored: ParticlesComponent = serde_json::from_str(&json).unwrap();
        assert_eq!(component, restored);
    }
    #[test]
    fn identical_previews_follow_identical_particle_paths() {
        let component = presets::preset(2);
        let mut first = Preview::new(&component).unwrap();
        let mut second = Preview::new(&component).unwrap();
        for _ in 0..60 {
            first.step(&component, Duration::from_millis(16)).unwrap();
            second.step(&component, Duration::from_millis(16)).unwrap();
        }
        let paths = |preview: &Preview| {
            preview
                .effects
                .flecks()
                .iter()
                .map(|fleck| (fleck.position, fleck.velocity))
                .collect::<Vec<_>>()
        };
        assert_eq!(paths(&first), paths(&second));
    }
}
