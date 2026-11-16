//! What stands by a map's water, mirrored in it (`map_water.reflections`).
//!
//! A picture standing on the map (a billboard) is mirrored onto the ground in
//! front of it, upside down, as still water mirrors a tree on its bank. The
//! mirror image is cut along the map's squares and only the pieces over water
//! are drawn, so it shows in the river and not on the grass. It is drawn after
//! the water and before the banks' frayed edges, darker and bluer than the
//! picture, fading with its length and wavering a pixel to and fro by rows.

use std::sync::OnceLock;

use glam::{Mat4, Vec3, Vec4Swizzles};
use sindri_render::{SpriteInstance, SpriteShader, TextureId, TransparentOrder, UvRect};

use super::super::sprite::{DrawSpace, SpriteDraw};
use super::{CHUNK, MapWorld};

const SOURCE: &str = include_str!("voxel_map_reflections.wgsl");
/// How long a mirror image is for the picture's height: shorter, as the
/// camera looks down at the water.
const LENGTH: f32 = 0.8;
/// Reflections are drawn after the squares and before the frayed edges.
pub(super) const FIRST_REFLECTION: u32 = 1 << 22;
pub(super) const FIRST_PIECE: u32 = 1 << 23;

fn shader() -> Option<SpriteShader> {
    static SHADER: OnceLock<Option<SpriteShader>> = OnceLock::new();
    SHADER
        .get_or_init(|| SpriteShader::new("builtin:map_reflection", SOURCE).ok())
        .clone()
}

/// Where a map was last drawn, so what stands on it can be mirrored in its
/// water in the same place and order.
#[derive(Clone, Copy, Debug)]
pub(super) struct Drawn {
    pub(super) root: Mat4,
    pub(super) layer: i32,
    pub(super) distance: f32,
    pub(super) strength: f32,
}

impl MapWorld {
    /// Remembers where the map is drawn this frame, for what is mirrored in
    /// its water, and starts the frame's mirror pieces afresh.
    pub(super) fn drawn_at(&mut self, root: Mat4, layer: i32, distance: f32) {
        self.drawn = Some(Drawn {
            root,
            layer,
            distance,
            strength: self.water.as_ref().map_or(0.0, super::Water::reflections),
        });
        self.reflected = 0;
    }

    /// The pieces of a standing picture's mirror image that lie over this
    /// map's water: `model` is the picture's quad as drawn, `rect` the part
    /// of `texture` it shows.
    pub(super) fn reflections(
        &mut self,
        model: Mat4,
        texture: TextureId,
        rect: UvRect,
        out: &mut Vec<SpriteDraw>,
    ) {
        let (Some(drawn), Some(shader)) = (self.drawn, shader()) else {
            return;
        };
        if drawn.strength <= 0.0 {
            return;
        }
        let to_map = drawn.root.inverse();
        let point = |x: f32, y: f32| (to_map * model * glam::Vec4::new(x, y, 0.0, 1.0)).xyz();
        let foot = point(0.0, -0.5);
        let height = (point(0.0, 0.5) - foot).length();
        let across = point(0.5, -0.5) - point(-0.5, -0.5);
        let width = across.length();
        if height <= f32::EPSILON || width <= f32::EPSILON {
            return;
        }
        let mirrored = across.x < 0.0;

        let length = height * LENGTH;
        let (west, east) = (foot.x - width / 2.0, foot.x + width / 2.0);
        let (south, north) = (foot.y - length, foot.y);
        #[allow(clippy::cast_possible_truncation)]
        let (first_column, last_column) = (west.floor() as i32, east.floor() as i32);
        // A row's square runs from -row - 1 up to -row.
        #[allow(clippy::cast_possible_truncation)]
        let (first_row, last_row) = ((-north).floor() as i32, (-south).floor() as i32);
        for row in first_row..=last_row {
            for column in first_column..=last_column {
                if !self.is_water(column, row) {
                    continue;
                }
                #[allow(clippy::cast_precision_loss)]
                let (x0, x1) = ((column as f32).max(west), (column as f32 + 1.0).min(east));
                #[allow(clippy::cast_precision_loss)]
                let (y0, y1) = ((-(row as f32) - 1.0).max(south), (-(row as f32)).min(north));
                if x1 - x0 <= 1e-4 || y1 - y0 <= 1e-4 {
                    continue;
                }
                // How far along the mirror image this piece is, from the foot.
                let along = |y: f32| ((north - y) / length).clamp(0.0, 1.0);
                let (near, far) = (along(y1), along(y0));
                let across = |x: f32| ((x - west) / width).clamp(0.0, 1.0);
                let (mut u0, mut u1) = (across(x0), across(x1));
                if mirrored {
                    (u0, u1) = (1.0 - u1, 1.0 - u0);
                }
                // Upside down: the picture's foot is nearest the bank.
                let x = (rect.x() + u0 * rect.width()).clamp(0.0, 1.0);
                let y = (rect.y() + (1.0 - far) * rect.height()).clamp(0.0, 1.0);
                let Ok(part) = UvRect::new(
                    x,
                    y,
                    ((u1 - u0) * rect.width()).min(1.0 - x),
                    ((far - near) * rect.height()).min(1.0 - y),
                ) else {
                    continue;
                };
                let flip = if mirrored { -1.0 } else { 1.0 };
                let local = Mat4::from_translation(Vec3::new(
                    f32::midpoint(x0, x1),
                    f32::midpoint(y0, y1),
                    0.0,
                )) * Mat4::from_scale(Vec3::new(flip * (x1 - x0), -(y1 - y0), 1.0));
                let index = FIRST_REFLECTION.saturating_add(self.reflected);
                self.reflected = self
                    .reflected
                    .saturating_add(1)
                    .min(FIRST_PIECE - FIRST_REFLECTION - 1);
                let Ok(order) = TransparentOrder::new(drawn.layer, drawn.distance, index) else {
                    continue;
                };
                out.push(SpriteDraw {
                    mask: None,
                    shader: Some(shader.clone()),
                    normal_map: None,
                    clip: None,
                    space: DrawSpace::World,
                    texture,
                    order,
                    sprite: SpriteInstance::new(drawn.root * local, [1.0; 4])
                        .with_uv_rect(part)
                        .lit(true)
                        .with_shader_params([far, near, drawn.strength, 0.0]),
                });
            }
        }
    }

    /// Whether a square is water, as far as the chunks worked out so far say.
    fn is_water(&self, column: i32, row: i32) -> bool {
        self.chunks
            .get(&(column.div_euclid(CHUNK), row.div_euclid(CHUNK)))
            .is_some_and(|chunk| chunk.water.contains_key(&(column, row)))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_reflection_shader_compiles() {
        assert!(
            super::shader().is_some(),
            "{:?}",
            sindri_render::SpriteShader::new("reflection", super::SOURCE).err()
        );
    }
}
