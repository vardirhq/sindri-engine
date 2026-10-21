//! Frame lighting: ambient fill, the directional sun and bounded local lights.

use sindri_core::{SceneComponent, World};
use sindri_render::{LocalLight, WorldLighting};

use super::{SceneExtractError, SceneExtractor};
use crate::{EnvironmentComponent, LightComponent, LightError, LightKind, Sun, lights_in, sun_in};

impl SceneExtractor {
    /// Ambient fill, the first active directional sun, and up to eight active
    /// point/spot lights. A scene without authored lighting uses renderer defaults.
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

    /// The strongest active local lights, with stable world iteration breaking
    /// intensity ties. Validate every light before applying the renderer cap.
    fn apply_lights(
        &self,
        world: &World,
        lighting: &mut WorldLighting,
    ) -> Result<(), SceneExtractError> {
        let mut points = Vec::<LocalLight>::new();
        let mut sun_found = false;
        for (entity, light) in lights_in(world) {
            let resolved = light.and_then(|light| {
                if !sun_found && let Some(sun) = sun_in(world, light, entity) {
                    lighting.directional_direction = sun.direction;
                    lighting.directional_color = sun.color;
                    lighting.directional_intensity = sun.intensity;
                    sun_found = true;
                }
                if light.kind == LightKind::Directional
                    || !light.enabled
                    || !world.is_active(entity)
                    || light.intensity == 0.0
                {
                    return Ok(None);
                }
                let transform = world.world_transform(entity).unwrap_or_default();
                LocalLight {
                    point: light.point_at(transform.position),
                    spot: (light.kind == LightKind::Spot)
                        .then(|| light.spot_cone(crate::light_direction(transform).to_array())),
                }
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
        points.sort_by(|a, b| b.point.intensity.total_cmp(&a.point.intensity));
        points.truncate(sindri_render::MAX_LOCAL_LIGHTS);
        for (slot, point) in lighting.local_lights.iter_mut().zip(points) {
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

impl SceneExtractor {
    /// Everything the scene's environment asks of the 3D renderer for one
    /// frame -- its light, shadows, fog, ambient occlusion and the
    /// post-processing the frame is finished with -- for
    /// [`sindri_render::TexturedCubeRenderer::configure`].
    ///
    /// One answer for every host, so a shipped game is lit the way the editor
    /// shows it. Hosts used to set the light and shadows themselves and stop
    /// there, which left an authored fog, occlusion strength or grade visible
    /// in the editor and gone from the game.
    ///
    /// # Errors
    /// An invalid environment or light, as [`Self::lighting`].
    pub fn world_settings(
        &self,
        world: &World,
    ) -> Result<sindri_render::WorldSettings, SceneExtractError> {
        let lighting = self.lighting(world)?;
        let environment = self.environment(world)?;
        Ok(sindri_render::WorldSettings {
            lighting,
            shadows: environment
                .map(EnvironmentComponent::shadow_settings)
                .unwrap_or_default(),
            fog: environment
                .map(EnvironmentComponent::fog_settings)
                .unwrap_or_default(),
            ambient_occlusion: environment.map_or(0.0, |environment| {
                if environment.ambient_occlusion.enabled {
                    environment.ambient_occlusion.strength
                } else {
                    0.0
                }
            }),
            post_process: environment
                .map(EnvironmentComponent::post_process_settings)
                .unwrap_or_default(),
        })
    }
}
