//! The studio's little world: a sprite, a light and an environment, as a
//! scene would hold them.
//!
//! The lit preview is not an imitation of the game's lighting. The surface is
//! a `sindri.sprite` naming the picture and its map, the light is a
//! `sindri.light2d`, the dark is a `sindri.environment`'s `lighting_2d`, and
//! the frame is extracted from this world by a `SceneExtractor`, as every
//! game's frame is. CPU-side, so what reaches the renderer is tested here
//! without a GPU.

use glam::{Vec2 as GlamVec2, Vec3, Vec4};
use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent, Transform3D, World};
use sindri_scene::{
    CameraView, EnvironmentComponent, Light2dComponent, SceneExtractor, SpriteComponent,
    ViewCamera, WorldProjection,
};

use super::super::scene_io::scene_extractor;
use crate::normal_studio::{NormalStudio, StudioMode};

/// What the surface's sprite names. Not paths: these are bound by the
/// studio, never loaded, and a colon cannot appear in an asset ID.
pub(super) const SOURCE: &str = "studio:source";
pub(super) const NORMAL: &str = "studio:normal";
pub(super) const CHANNEL: &str = "studio:channel";

/// The half-height the viewer camera frames at a distance scale of one: its
/// base offset's length times the tangent of half its 45° field of view, as
/// `resolve_viewer_cameras` places it. Dividing by this frames a surface.
const BASE_HALF_HEIGHT: f32 = 2.230_6;

/// Which picture of the surface a frame is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Look {
    /// Lit, reading the map: what the game draws with `normal_map` set.
    Mapped,
    /// Lit by distance alone: what the game draws with no map.
    Flat,
    /// The picture as it is, no light at all.
    Unlit,
    /// The map, or a channel of it, as a picture.
    Channel,
}

pub(super) struct StudioScene {
    pub(super) extractor: SceneExtractor,
    pub(super) world: World,
    surface: EntityId,
    light: EntityId,
    environment: EntityId,
}

fn spawn(world: &mut World, name: &str) -> EntityId {
    world.spawn(EntityData {
        name: Some(name.to_owned()),
        transform_3d: Some(Transform3D::default()),
        ..EntityData::default()
    })
}

impl StudioScene {
    pub(super) fn new() -> Self {
        let mut world = World::default();
        let environment = spawn(&mut world, "Environment");
        let surface = spawn(&mut world, "Surface");
        let light = spawn(&mut world, "Light");
        Self {
            extractor: scene_extractor(),
            world,
            surface,
            light,
            environment,
        }
    }

    fn set(&mut self, entity: EntityId, change: impl FnOnce(&mut EntityData)) {
        if let Some(data) = self.world.get_mut(entity) {
            change(data);
        }
    }

    /// Sizes the surface and places the light and the dark as the studio
    /// says.
    pub(super) fn follow(&mut self, studio: &NormalStudio) {
        let [width, height] = studio.surface_size();
        self.set(self.surface, |data| {
            data.transform_3d = Some(Transform3D {
                scale: [width, height, 1.0],
                ..Transform3D::default()
            });
        });
        let light = studio.light.clone();
        self.set(self.light, |data| {
            data.transform_3d = Some(Transform3D {
                position: [light.position[0], light.position[1], 0.0],
                ..Transform3D::default()
            });
            data.components.insert(
                Light2dComponent::TYPE_NAME.to_owned(),
                serde_json::to_value(light.light).unwrap_or_default(),
            );
        });
        self.set(self.environment, |data| {
            data.components.insert(
                EnvironmentComponent::TYPE_NAME.to_owned(),
                json!({
                    "background": [0.055, 0.06, 0.075, 1.0],
                    "lighting_2d": { "enabled": true, "ambient": light.ambient }
                }),
            );
        });
    }

    /// Dresses the surface as `look`.
    pub(super) fn wear(&mut self, look: Look) {
        let (texture, normal_map, unlit) = match look {
            Look::Mapped => (SOURCE, NORMAL, false),
            Look::Flat => (SOURCE, "", false),
            Look::Unlit => (SOURCE, "", true),
            Look::Channel => (CHANNEL, "", true),
        };
        self.set(self.surface, |data| {
            data.components.insert(
                SpriteComponent::TYPE_NAME.to_owned(),
                json!({ "texture": texture, "normal_map": normal_map, "unlit": unlit }),
            );
        });
    }

    /// Where the frames are seen from, for a viewport of this shape.
    pub(super) fn view_camera(&self, aspect: f32, camera: CameraView) -> Option<ViewCamera> {
        self.extractor
            .world_camera_for_viewport(&self.world, aspect, camera)
            .ok()
            .flatten()
    }
}

/// How the surface is looked at: flat and straight on for every mode but 3D,
/// which orbits it. Framed so the surface fills most of a canvas of this
/// aspect, before the view's own zoom.
pub(super) fn camera_for(studio: &NormalStudio, aspect: f32) -> CameraView {
    let [width, height] = studio.surface_size();
    let half = 0.55 * height.max(width / aspect.max(f32::EPSILON));
    let framed = (half / BASE_HALF_HEIGHT).max(1.0e-4);
    if studio.view.mode == StudioMode::Inspect {
        CameraView {
            orbit: GlamVec2::from_array(studio.view.orbit.angles),
            distance_scale: framed * 1.2 / studio.view.orbit.zoom,
            pan: GlamVec2::ZERO,
            projection: WorldProjection::Perspective,
        }
    } else {
        CameraView {
            orbit: GlamVec2::ZERO,
            distance_scale: framed / studio.view.zoom,
            pan: GlamVec2::from_array(studio.view.pan),
            projection: WorldProjection::Flat,
        }
    }
}

/// Where a point in a view of `rect` lands on the surface's plane, z = 0.
pub(super) fn onto_plane(
    camera: &ViewCamera,
    rect: eframe::egui::Rect,
    point: eframe::egui::Pos2,
) -> Option<[f32; 2]> {
    let inverse = camera.view_projection.inverse();
    let x = (point.x - rect.min.x) / rect.width() * 2.0 - 1.0;
    let y = 1.0 - (point.y - rect.min.y) / rect.height() * 2.0;
    let unproject = |depth: f32| {
        let clip = inverse * Vec4::new(x, y, depth, 1.0);
        (clip.w.abs() > f32::EPSILON).then(|| clip.truncate() / clip.w)
    };
    let (near, far): (Vec3, Vec3) = (unproject(0.0)?, unproject(1.0)?);
    let along = far - near;
    if along.z.abs() < f32::EPSILON {
        return None;
    }
    let hit = near + along * (-near.z / along.z);
    hit.is_finite().then_some([hit.x, hit.y])
}

#[cfg(test)]
mod tests;
