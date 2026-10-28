//! 2D lights: points in the plane that light world sprites, tiles and shapes.
//!
//! A frame carries at most [`MAX_LIGHTS_2D`] of them, uploaded once per frame
//! to a uniform both the sprite and the shape pipelines read. A light that
//! casts shadows carries a one-dimensional polar shadow map built on the CPU
//! (see [`shadow_map_2d`]), so where a shadow falls is plain arithmetic a test
//! can check without a GPU.

mod gpu;
mod shadow;

pub(crate) use gpu::Lights2dGpu;
pub use shadow::{SHADOW_BINS_2D, SHADOW_SAMPLES_2D, shadow_map_2d, shadow_maps_2d};

use std::sync::Arc;

/// How many lights one frame can carry. The extractor keeps the ones nearest
/// the camera.
pub const MAX_LIGHTS_2D: usize = 16;

/// The WGSL both pipelines append to their own source.
pub(crate) const LIGHT_2D_WGSL: &str = include_str!("light2d.wgsl");

/// One point light in the plane.
#[derive(Clone, Debug, PartialEq)]
pub struct Light2d {
    pub position: [f32; 2],
    /// How far above the plane the light sits. Only normal-mapped art reads
    /// it: the higher, the more squarely it lights a flat surface.
    pub height: f32,
    /// Where the light reaches nothing.
    pub radius: f32,
    /// How fast it fades toward the radius: 1 is linear, 2 quadratic.
    pub falloff: f32,
    /// Linear color, already multiplied by the light's intensity.
    pub color: [f32; 3],
    pub shadow: Option<Light2dShadow>,
}

/// Where a light is blocked, by direction.
#[derive(Clone, Debug, PartialEq)]
pub struct Light2dShadow {
    /// How far past an occluding edge light still reaches, so the face of a
    /// wall the light falls on is lit rather than starting the dark.
    pub reach: f32,
    /// Radius of the light-emitting area, in world units; zero is a point.
    pub source_radius: f32,
    /// One or five [`SHADOW_BINS_2D`]-wide maps from [`shadow_maps_2d`].
    pub map: Arc<[f32]>,
}

/// Everything the frame's lit 2D content is lit by.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lighting2d {
    /// Linear color multiplied in where no light reaches.
    pub ambient: [f32; 3],
    /// At most [`MAX_LIGHTS_2D`]; any more are ignored.
    pub lights: Vec<Light2d>,
}

impl Lighting2d {
    /// The light reaching `point` on a surface facing the viewer, as the
    /// shader computes it without a normal map or shadows. For tests and for
    /// tools that want to know whether something is in the dark.
    #[must_use]
    pub fn at(&self, point: [f32; 2]) -> [f32; 3] {
        let mut total = self.ambient;
        for light in self.lights.iter().take(MAX_LIGHTS_2D) {
            let offset = [light.position[0] - point[0], light.position[1] - point[1]];
            let distance = offset[0].hypot(offset[1]);
            if distance >= light.radius {
                continue;
            }
            let mut amount = (1.0 - distance / light.radius).max(0.0).powf(light.falloff);
            if let Some(shadow) = &light.shadow {
                amount *= shadow::visibility(light.position, point, shadow);
            }
            for (channel, color) in total.iter_mut().zip(light.color) {
                *channel += color * amount;
            }
        }
        total
    }
}

#[cfg(test)]
mod tests;
