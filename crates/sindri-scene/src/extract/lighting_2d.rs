//! The 2D lights a frame is drawn with, and the shadows they cast.
//!
//! Gathered once per frame: the environment says whether 2D lighting is on and
//! what the dark is, every active `sindri.light2d` is a light, and for lights
//! that cast shadows every active occluder's edges, in world space, become one
//! polar shadow map per light. The frame carries at most
//! [`MAX_LIGHTS_2D`] lights, the ones nearest the camera.

use std::sync::Arc;

use glam::{Mat4, Vec3};
use sindri_core::{ComponentSchemaRegistry, EntityId, SceneComponent, World};
use sindri_render::{Light2d, Light2dShadow, Lighting2d, MAX_LIGHTS_2D, shadow_maps_2d};

use super::camera::ResolvedCameras;
use super::{SceneExtractError, SceneExtractor, transform_matrix};
use crate::{
    Light2dComponent, Light2dError, Light2dOccluderComponent, OccluderShape,
    TilemapCollider2dComponent, TilemapComponent, components::outline,
};

impl SceneExtractor {
    /// The frame's 2D lighting, or `None` when the environment has it off.
    pub(super) fn lighting_2d(
        &self,
        world: &World,
        cameras: &ResolvedCameras,
    ) -> Result<Option<Lighting2d>, SceneExtractError> {
        let Some(environment) = self.environment(world)? else {
            return Ok(None);
        };
        if !environment.lighting_2d.enabled {
            return Ok(None);
        }
        let mut lights = self.lights_2d(world)?;
        // The camera's own position, from the inverse of where it looks from.
        let eye = cameras
            .world
            .map_or(Vec3::ZERO, |camera| camera.view.inverse().w_axis.truncate());
        lights.sort_by(|a, b| {
            let near = |light: &Light2d| {
                (light.position[0] - eye.x).hypot(light.position[1] - eye.y) - light.radius
            };
            near(a).total_cmp(&near(b))
        });
        lights.truncate(MAX_LIGHTS_2D);
        if lights.iter().any(|light| light.shadow.is_some()) {
            let edges = self.occluder_edges(world)?;
            for light in &mut lights {
                if let Some(shadow) = &mut light.shadow {
                    shadow.map = Arc::from(shadow_maps_2d(
                        light.position,
                        light.radius,
                        shadow.source_radius,
                        &edges,
                    ));
                }
            }
        }
        Ok(Some(Lighting2d {
            ambient: environment.lighting_2d.ambient,
            lights,
        }))
    }

    /// Every active, enabled light in the world, shadows not yet cast.
    fn lights_2d(&self, world: &World) -> Result<Vec<Light2d>, SceneExtractError> {
        let (lights, refused) = gather_lights(&self.components, world)?;
        for (entity, error) in refused {
            self.refuse(entity, Light2dComponent::TYPE_NAME, error)?;
        }
        Ok(lights)
    }

    /// Every active occluder's edges, in world space.
    fn occluder_edges(&self, world: &World) -> Result<Vec<[[f32; 2]; 2]>, SceneExtractError> {
        let (edges, refused) = gather_edges(&self.components, world)?;
        for (entity, error) in refused {
            self.refuse(entity, Light2dOccluderComponent::TYPE_NAME, error)?;
        }
        Ok(edges)
    }

    /// Strictly, a component the engine cannot use fails the frame; tolerantly
    /// it is recorded against its entity and passed over.
    fn refuse(
        &self,
        entity: EntityId,
        type_name: &'static str,
        error: Light2dError,
    ) -> Result<(), SceneExtractError> {
        if self.tolerant() {
            self.record(entity, type_name, &error);
            Ok(())
        } else {
            Err(SceneExtractError::Light2d(error))
        }
    }
}

/// What was gathered, and the entities whose component could not be used.
pub(crate) type Gathered<T> = (Vec<T>, Vec<(EntityId, Light2dError)>);

/// Every active, enabled light, shadows not yet cast, and the lights that
/// could not be used.
pub(crate) fn gather_lights(
    components: &ComponentSchemaRegistry,
    world: &World,
) -> Result<Gathered<Light2d>, SceneExtractError> {
    let mut lights = Vec::new();
    let mut refused = Vec::new();
    for (entity, light) in components.query::<Light2dComponent>(world)? {
        if !world.is_active(entity) || !light.enabled {
            continue;
        }
        let light = match light.validate() {
            Ok(light) => light,
            Err(error) => {
                refused.push((entity, error));
                continue;
            }
        };
        let position = world.world_transform(entity).unwrap_or_default().position;
        lights.push(Light2d {
            position: [position[0], position[1]],
            height: light.height,
            radius: light.radius,
            falloff: light.falloff,
            color: light.radiance(),
            shadow: light.shadows.then(|| Light2dShadow {
                reach: light.shadow_reach,
                source_radius: light.shadow_source_radius,
                map: Arc::from([]),
            }),
        });
    }
    Ok((lights, refused))
}

/// Every active occluder's edges, in world space, and the occluders that
/// could not be used.
pub(crate) fn gather_edges(
    components: &ComponentSchemaRegistry,
    world: &World,
) -> Result<Gathered<[[f32; 2]; 2]>, SceneExtractError> {
    let mut edges = Vec::new();
    let mut refused = Vec::new();
    for (entity, occluder) in components.query::<Light2dOccluderComponent>(world)? {
        if !world.is_active(entity) || !occluder.enabled {
            continue;
        }
        let local = match local_occluder_edges(world, entity, &occluder) {
            Ok(local) => local,
            Err(error) => {
                refused.push((entity, error));
                continue;
            }
        };
        let matrix = transform_matrix(world.world_transform(entity).unwrap_or_default());
        edges.extend(
            local
                .into_iter()
                .map(|edge| edge.map(|point| placed(matrix, point))),
        );
    }
    Ok((edges, refused))
}

fn placed(matrix: Mat4, [x, y]: [f32; 2]) -> [f32; 2] {
    let point = matrix.transform_point3(Vec3::new(x, y, 0.0));
    [point.x, point.y]
}

/// An occluder's edges in its entity's own space.
fn local_occluder_edges(
    world: &World,
    entity: EntityId,
    occluder: &Light2dOccluderComponent,
) -> Result<Vec<[[f32; 2]; 2]>, Light2dError> {
    if occluder.shape != OccluderShape::Tilemap {
        return occluder.local_edges();
    }
    let data = world.get(entity).ok_or(Light2dError::NoTilemap)?;
    let tilemap = data
        .components
        .get(TilemapComponent::TYPE_NAME)
        .and_then(|payload| serde_json::from_value::<TilemapComponent>(payload.clone()).ok())
        .ok_or(Light2dError::NoTilemap)?;
    // The collider's own passable list when there is one, so what a
    // character walks through a light shines through too.
    let collider = data
        .components
        .get(TilemapCollider2dComponent::TYPE_NAME)
        .and_then(|payload| {
            serde_json::from_value::<TilemapCollider2dComponent>(payload.clone()).ok()
        })
        .unwrap_or_else(|| TilemapCollider2dComponent {
            passable: Vec::new(),
            layers: sindri_physics::CollisionLayers::default(),
            friction: 0.0,
            restitution: 0.0,
        });
    let Ok(pieces) = collider.pieces(&tilemap, [1.0, 1.0]) else {
        // An isometric map is a floor seen at an angle; it blocks nothing.
        return Ok(Vec::new());
    };
    Ok(pieces
        .iter()
        .filter_map(|piece| match piece.shape {
            sindri_physics::ColliderShape2d::Box { half_extents } => {
                let [x, y] = half_extents;
                let [cx, cy] = piece.offset;
                Some(outline(&[
                    [cx - x, cy - y],
                    [cx + x, cy - y],
                    [cx + x, cy + y],
                    [cx - x, cy + y],
                ]))
            }
            _ => None,
        })
        .flatten()
        .collect())
}
