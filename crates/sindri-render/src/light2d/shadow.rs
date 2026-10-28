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

/// The center and four offsets used to approximate an area light.
pub const SHADOW_SAMPLES_2D: usize = 5;

/// Sample positions relative to the center and radius of a light source.
pub const SHADOW_SAMPLE_OFFSETS_2D: [[f32; 2]; SHADOW_SAMPLES_2D] = [
    [0.0, 0.0],
    [-0.75, 0.0],
    [0.75, 0.0],
    [0.0, -0.75],
    [0.0, 0.75],
];

/// Build one map for a point light, or five maps for an area light.
#[must_use]
pub fn shadow_maps_2d(
    origin: [f32; 2],
    radius: f32,
    source_radius: f32,
    edges: &[[[f32; 2]; 2]],
) -> Vec<f32> {
    let count = if source_radius > 0.0 {
        SHADOW_SAMPLES_2D
    } else {
        1
    };
    let mut maps = Vec::with_capacity(SHADOW_BINS_2D * count);
    for offset in SHADOW_SAMPLE_OFFSETS_2D.iter().take(count) {
        let source = [
            origin[0] + offset[0] * source_radius,
            origin[1] + offset[1] * source_radius,
        ];
        // A displaced source can see an occluder just beyond the main radius.
        maps.extend(shadow_map_2d(source, radius + source_radius, edges));
    }
    maps
}

/// Visibility uses the same area samples and three-bin filter as the shader.
pub(super) fn visibility(origin: [f32; 2], point: [f32; 2], shadow: &super::Light2dShadow) -> f32 {
    let count = if shadow.source_radius > 0.0 {
        SHADOW_SAMPLES_2D
    } else {
        1
    };
    let mut visible = 0.0;
    for (sample, offset) in SHADOW_SAMPLE_OFFSETS_2D.iter().take(count).enumerate() {
        let away = [
            point[0] - origin[0] - offset[0] * shadow.source_radius,
            point[1] - origin[1] - offset[1] * shadow.source_radius,
        ];
        let distance = away[0].hypot(away[1]);
        let center = bin_of(away);
        for bin in [
            (center + SHADOW_BINS_2D - 1) % SHADOW_BINS_2D,
            center,
            (center + 1) % SHADOW_BINS_2D,
        ] {
            let edge = shadow
                .map
                .get(sample * SHADOW_BINS_2D + bin)
                .copied()
                .unwrap_or(OPEN);
            if distance <= edge + shadow.reach {
                visible += 1.0;
            }
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let divisor = (count * 3) as f32;
    visible / divisor
}

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
