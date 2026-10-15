//! Frame lighting: ambient fill, the directional sun and bounded point lights.

use sindri_core::{SceneComponent, World};
use sindri_render::{PointLight, WorldLighting};

use super::{SceneExtractError, SceneExtractor};
use crate::{EnvironmentComponent, LightComponent, LightError, LightKind, Sun, lights_in, sun_in};

impl SceneExtractor {
    /// Ambient fill, the first active directional sun, and up to eight active
    /// point lights. A scene without authored lighting uses renderer defaults.
    pub fn lighting(&self, world: &World) -> Result<WorldLighting, SceneExtractError> {
        let unlit = WorldLighting {
            directional_intensity: 0.0,
            ..WorldLighting::default()
        };
        let mut lighting = self
            .environment(world)?
            .map_or(unlit, EnvironmentComponent::world_lighting);
        self.apply_lights(world, &mut lighting)?;
        Ok(lighting)
    }

    /// The strongest active point lights, with stable world iteration breaking
    /// intensity ties. Validate every light before applying the renderer cap.
    fn apply_lights(
        &self,
        world: &World,
        lighting: &mut WorldLighting,
    ) -> Result<(), SceneExtractError> {
        let mut points = Vec::<PointLight>::new();
        let mut sun_found = false;
        for (entity, light) in lights_in(world) {
            let resolved = light.and_then(|light| {
                if !sun_found && let Some(sun) = sun_in(world, light, entity) {
                    lighting.directional_direction = sun.direction;
                    lighting.directional_color = sun.color;
                    lighting.directional_intensity = sun.intensity;
                    sun_found = true;
                }
                if light.kind != LightKind::Point
                    || !world.is_active(entity)
                    || light.intensity == 0.0
                {
                    return Ok(None);
                }
                let position = world.world_transform(entity).unwrap_or_default().position;
                light
                    .point_at(position)
                    .validate()
                    .map(Some)
                    .map_err(LightError::from)
            });
            match resolved {
                Ok(Some(point)) => points.push(point),
                Ok(None) => {}
                Err(error) if self.tolerant() => {
                    self.record(entity, LightComponent::TYPE_NAME, &error);
                }
                Err(error) => return Err(error.into()),
            }
        }
        points.sort_by(|a, b| b.intensity.total_cmp(&a.intensity));
        points.truncate(sindri_render::MAX_POINT_LIGHTS);
        for (slot, point) in lighting.point_lights.iter_mut().zip(points) {
            *slot = Some(point);
        }
        Ok(())
    }

    /// The light entity lighting the world, if any.
    ///
    /// Strictly, a light the engine cannot use fails the frame. Tolerantly it
    /// is recorded against its entity and passed over, so a value dragged out
    /// of range in the inspector turns the light off while it is wrong rather
    /// than freezing the view.
    pub fn sun(&self, world: &World) -> Result<Option<Sun>, SceneExtractError> {
        for (entity, light) in lights_in(world) {
            match light {
                Ok(light) => {
                    if let Some(sun) = sun_in(world, light, entity) {
                        return Ok(Some(sun));
                    }
                }
                Err(error) if self.tolerant() => {
                    self.record(entity, LightComponent::TYPE_NAME, &error);
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(None)
    }
}
