//! The shadows things cast onto the ground: a contact ellipse under each, and a
//! silhouette laid away from every light near it.
//!
//! Both are painted over the lit ground and are not lit themselves, so they
//! darken what light reaches and change nothing in the dark. The contact shadow is
//! a feathered black ellipse drawn as a shape. The cast shadow is the thing's own
//! picture: its feet stay where they are and its head is carried away from the
//! light along the ground, so it is a card lying down and pointing away, long for
//! a tall thing and for a light a long way off, and always an area whichever side
//! the light is on. A light that is part of the thing, like a lantern it carries,
//! casts nothing from it.

use std::collections::BTreeMap;

use glam::{Mat4, Vec2, Vec3, Vec4};
use sindri_core::{EntityId, SceneComponent, World};
use sindri_render::{
    ExtractedFrame, FrameCamera, FrameCommand, FramePass, RenderLayer, RenderStage, Shape,
    ShapeBlend, ShapeInstance, SpriteInstance, TextureId, TransparentOrder, UvRect,
};

use crate::{Light2dComponent, Shadow2dComponent, SpriteComponent};

use super::camera::ResolvedCameras;
use super::camera::view::camera_distance;
use super::shape::world_model_matrix;
use super::sprite::{DrawSpace, SpriteDraw};
use super::{SceneExtractError, SceneExtractor};

/// How many lights cast a thing's shadow at once, the strongest at its feet.
const LIGHTS_PER_THING: usize = 2;

/// Light weaker than this at a thing's feet casts nothing worth drawing.
const FAINTEST: f32 = 0.02;

/// How soft a contact shadow's edge is, as a share of its size.
const FEATHER: f32 = 0.5;

/// A light that can cast a thing's shadow.
struct Caster {
    entity: EntityId,
    position: [f32; 2],
    height: f32,
    radius: f32,
    falloff: f32,
    /// How bright it is at its middle, which is what scales how dark a shadow it makes.
    brightness: f32,
}

/// What a frame needs to cast shadows: the things that cast them and the lights.
pub(super) struct CastShadows {
    things: BTreeMap<EntityId, Shadow2dComponent>,
    lights: Vec<Caster>,
}

impl SceneExtractor {
    /// The things that cast shadows and the lights they cast from, or `None` when
    /// nothing casts one or the environment has 2D lighting off.
    pub(super) fn cast_shadows(
        &self,
        world: &World,
    ) -> Result<Option<CastShadows>, SceneExtractError> {
        let mut things = BTreeMap::new();
        for (entity, shadow) in self.components.query::<Shadow2dComponent>(world)? {
            if !world.is_active(entity) {
                continue;
            }
            match shadow.validate() {
                Ok(shadow) => {
                    things.insert(entity, shadow);
                }
                Err(error) => self.refuse_shadow(entity, error)?,
            }
        }
        if things.is_empty()
            || !self
                .environment(world)?
                .is_some_and(|e| e.lighting_2d.enabled)
        {
            return Ok(None);
        }
        let mut lights = Vec::new();
        for (entity, light) in self.components.query::<Light2dComponent>(world)? {
            let Ok(light) = light.validate() else {
                continue;
            };
            if !world.is_active(entity) || !light.enabled {
                continue;
            }
            let position = world.world_transform(entity).unwrap_or_default().position;
            let [r, g, b] = light.radiance();
            lights.push(Caster {
                entity,
                position: [position[0], position[1]],
                height: light.height,
                radius: light.radius,
                falloff: light.falloff,
                brightness: 0.2126 * r + 0.7152 * g + 0.0722 * b,
            });
        }
        Ok(Some(CastShadows { things, lights }))
    }

    /// A shadow that cannot be used fails a strict frame, and is passed over by a
    /// tolerant one, as a light or an occluder is.
    fn refuse_shadow(
        &self,
        entity: EntityId,
        error: crate::Shadow2dError,
    ) -> Result<(), SceneExtractError> {
        if self.tolerant() {
            self.record(entity, Shadow2dComponent::TYPE_NAME, &error);
            Ok(())
        } else {
            Err(SceneExtractError::Shadow2d(error))
        }
    }

    /// The contact shadows under every thing that has one, under its sprite: a
    /// layer below the sprite's own, or the lowest if it has none.
    pub(super) fn push_contact_shadows(
        &self,
        world: &World,
        cameras: &ResolvedCameras,
        frame: &mut ExtractedFrame,
    ) -> Result<(), SceneExtractError> {
        let shadows = self.components.query::<Shadow2dComponent>(world)?;
        if shadows.is_empty() {
            return Ok(());
        }
        let camera = cameras.world.ok_or(SceneExtractError::MissingWorldCamera)?;
        let layers: BTreeMap<EntityId, i32> = self
            .components
            .query::<SpriteComponent>(world)?
            .into_iter()
            .map(|(entity, sprite)| (entity, sprite.layer))
            .collect();
        let mut batches: BTreeMap<i32, Vec<ShapeInstance>> = BTreeMap::new();
        for (entity, shadow) in shadows {
            if !world.is_active(entity) {
                continue;
            }
            let shadow = match shadow.validate() {
                Ok(shadow) if shadow.has_contact() => shadow,
                Ok(_) => continue,
                Err(error) => {
                    self.refuse_shadow(entity, error)?;
                    continue;
                }
            };
            let at = world_model_matrix(world, entity).w_axis.truncate();
            let model = Mat4::from_translation(Vec3::new(
                at.x + shadow.contact_offset[0],
                at.y + shadow.contact_offset[1],
                at.z - 0.001,
            )) * Mat4::from_scale(Vec3::new(
                shadow.contact_size[0],
                shadow.contact_size[1],
                1.0,
            ));
            let layer = layers.get(&entity).map_or(-1, |layer| layer - 1);
            batches.entry(layer).or_default().push(
                ShapeInstance::filled(
                    model,
                    Shape::Ellipse,
                    [0.0, 0.0, 0.0, shadow.contact_opacity],
                )
                .feathered(FEATHER)
                .lit(false),
            );
        }
        for (layer, instances) in batches {
            frame.push(FramePass::new(
                RenderStage::Transparent2d,
                RenderLayer(layer),
                FrameCamera {
                    view_projection: camera.view_projection,
                    position: Vec3::ZERO,
                },
                FrameCommand::Shapes {
                    blend: ShapeBlend::Over,
                    instances,
                },
            ));
        }
        Ok(())
    }
}

impl CastShadows {
    /// Whether `entity` is lit from where it stands.
    pub(super) fn stands(&self, entity: EntityId) -> bool {
        self.things
            .get(&entity)
            .is_some_and(|shadow| shadow.lit_from_base)
    }

    /// The silhouettes `entity` casts, if it casts any: its picture, `model`
    /// placing it and `rect` the part of `texture` it shows, laid away from each
    /// of the lights that reach its feet.
    pub(super) fn draws(
        &self,
        world: &World,
        entity: EntityId,
        model: Mat4,
        picture: (TextureId, UvRect),
        layer: i32,
        camera_view: Mat4,
    ) -> Vec<SpriteDraw> {
        let Some(shadow) = self.things.get(&entity).filter(|shadow| shadow.cast) else {
            return Vec::new();
        };
        let (texture, rect) = picture;
        let feet = model.transform_point3(Vec3::new(0.0, -0.5, 0.0));
        let width = model.x_axis.truncate().length();
        let drawn_height = model.y_axis.truncate().length();
        let flip = if model.x_axis.x < 0.0 { -1.0 } else { 1.0 };
        let tall = if shadow.height > 0.0 {
            shadow.height
        } else {
            drawn_height
        };
        let mut reaching: Vec<(f32, &Caster)> = self
            .lights
            .iter()
            .filter(|light| !is_part_of(world, light.entity, entity))
            .filter_map(|light| {
                let reach = (light.position[0] - feet.x).hypot(light.position[1] - feet.y);
                let strength =
                    (1.0 - reach / light.radius).max(0.0).powf(light.falloff) * light.brightness;
                (strength > FAINTEST).then_some((strength.min(1.0), light))
            })
            .collect();
        reaching.sort_by(|a, b| b.0.total_cmp(&a.0));
        reaching.truncate(LIGHTS_PER_THING);

        reaching
            .into_iter()
            .filter_map(|(strength, light)| {
                let away = Vec2::new(feet.x - light.position[0], feet.y - light.position[1]);
                let distance = away.length();
                if distance < 0.05 {
                    return None;
                }
                let along = away / distance;
                let length = (tall * distance / light.height.max(0.25)).min(shadow.max_length);
                let across = Vec2::new(-along.y, along.x) * (width * flip);
                let lying = Mat4::from_cols(
                    Vec4::new(across.x, across.y, 0.0, 0.0),
                    Vec4::new(along.x * length, along.y * length, 0.0, 0.0),
                    model.z_axis,
                    Vec4::new(
                        feet.x + along.x * length * 0.5,
                        feet.y + along.y * length * 0.5,
                        feet.z - 0.001,
                        1.0,
                    ),
                );
                let order = TransparentOrder::new(
                    layer - 1,
                    camera_distance(camera_view, lying.w_axis.truncate()),
                    entity.index(),
                )
                .ok()?;
                Some(SpriteDraw {
                    mask: None,
                    clip: None,
                    space: DrawSpace::World,
                    texture,
                    shader: None,
                    normal_map: None,
                    order,
                    sprite: SpriteInstance::new(
                        lying,
                        [0.0, 0.0, 0.0, shadow.cast_opacity * strength],
                    )
                    .lit(false)
                    .with_uv_rect(rect),
                })
            })
            .collect()
    }
}

/// Whether `light` is `thing` or carried by it, anywhere below it in the hierarchy.
fn is_part_of(world: &World, light: EntityId, thing: EntityId) -> bool {
    let mut cursor = Some(light);
    while let Some(entity) = cursor {
        if entity == thing {
            return true;
        }
        cursor = world.get(entity).and_then(|data| data.parent);
    }
    false
}
