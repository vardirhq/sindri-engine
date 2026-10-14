//! One-dimensional polar shadow maps, built on the CPU.
//!
//! For each of [`SHADOW_BINS_2D`] directions round a light, the distance to
//! the nearest occluding edge that way. A pixel further than that, in that
//! direction, is in shadow. Built from edges rather than rendered, because an
//! occluder is a handful of segments and a tilemap's merged boxes a few dozen,
//! and arithmetic is the same on every host.

use std::f32::consts::{PI, TAU};

/// Directions per light. 256 keeps a shadow's edge within a degree and a half.
pub const SHADOW_BINS_2D: usize = 256;

/// What a direction with nothing in it holds: further than any light reaches.
const OPEN: f32 = 1.0e9;

/// The bin a direction falls in, matching the shader's own arithmetic.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub(super) fn bin_of(direction: [f32; 2]) -> usize {
    let turn = (direction[1].atan2(direction[0]) + PI) / TAU;
    ((turn * SHADOW_BINS_2D as f32).floor() as usize).min(SHADOW_BINS_2D - 1)
}

/// The shadow map of a light at `origin` reaching `radius`, blocked by
/// `edges`, each a segment between two points in the plane.
///
/// Each bin holds the distance along the bin's middle ray to the nearest edge
/// it crosses. Edges entirely outside the radius are skipped.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn shadow_map_2d(origin: [f32; 2], radius: f32, edges: &[[[f32; 2]; 2]]) -> Vec<f32> {
    let mut map = vec![OPEN; SHADOW_BINS_2D];
    let near: Vec<_> = edges
        .iter()
        .filter(|edge| distance_to_segment(origin, **edge) < radius)
        .collect();
    if near.is_empty() {
        return map;
    }
    for (bin, nearest) in map.iter_mut().enumerate() {
        let angle = (bin as f32 + 0.5) / SHADOW_BINS_2D as f32 * TAU - PI;
        let ray = [angle.cos(), angle.sin()];
        for edge in &near {
            if let Some(distance) = ray_hits(origin, ray, **edge) {
                *nearest = nearest.min(distance);
            }
        }
    }
    map
}

/// How far along `ray` from `origin` it crosses `edge`, if it does.
fn ray_hits(origin: [f32; 2], ray: [f32; 2], [a, b]: [[f32; 2]; 2]) -> Option<f32> {
    let along = [b[0] - a[0], b[1] - a[1]];
    let denominator = cross(ray, along);
    if denominator.abs() < 1.0e-9 {
        return None;
    }
    let to_a = [a[0] - origin[0], a[1] - origin[1]];
    let distance = cross(to_a, along) / denominator;
    let across = cross(to_a, ray) / denominator;
    ((0.0..=1.0).contains(&across) && distance >= 0.0).then_some(distance)
}

const fn cross(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}

fn distance_to_segment(point: [f32; 2], [a, b]: [[f32; 2]; 2]) -> f32 {
    let along = [b[0] - a[0], b[1] - a[1]];
    let length = along[0] * along[0] + along[1] * along[1];
    let t = if length <= f32::EPSILON {
        0.0
    } else {
        (((point[0] - a[0]) * along[0] + (point[1] - a[1]) * along[1]) / length).clamp(0.0, 1.0)
    };
    let closest = [a[0] + along[0] * t, a[1] + along[1] * t];
    (point[0] - closest[0]).hypot(point[1] - closest[1])
}
