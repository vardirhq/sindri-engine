//! What a light can see, exactly, built on the CPU.
//!
//! The region a point light sees is the plane out to the nearest blocking edge
//! in every direction. Round the light that is a list of angular intervals, each
//! with the one edge nearest the light across it, so a shadow's boundary is the
//! true angle of an edge's end and a corner casts a corner. Each interval holds
//! the line of its edge as a unit normal `n` and the distance `d` of the line
//! from the light, so the distance along a ray `u` through it is `d / (n . u)`.
//!
//! Built from edges rather than rendered, because an occluder is a handful of
//! segments and a tilemap's merged boxes a few dozen, and arithmetic is the same
//! on every host: the shader searches the same intervals with the same formula,
//! and [`super::Lighting2d::at`] answers a question about a point from them
//! without a GPU.

use std::f32::consts::PI;

/// Angular intervals one source can hold. A scene of a few dozen boxes needs
/// about a hundred; past this the narrowest intervals are folded into their
/// neighbors, which loses slivers of shadow rather than any whole edge.
pub const SHADOW_INTERVALS_2D: usize = 256;

/// Floats in one interval: its start angle, the edge's unit normal (two), and
/// the line's distance from the light, negative where nothing blocks.
pub const SHADOW_INTERVAL_FLOATS_2D: usize = 4;

/// Floats one source's map holds.
pub const SHADOW_MAP_LEN_2D: usize = SHADOW_INTERVALS_2D * SHADOW_INTERVAL_FLOATS_2D;

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

/// A start angle no direction reaches, which pads the unused intervals so a
/// search never lands on one.
const PAST_THE_END: f32 = 1.0e9;

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
    let mut maps = Vec::with_capacity(SHADOW_MAP_LEN_2D * count);
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

/// Whether the light's samples see `point`: the share of them, 0 to 1, that
/// have no blocking edge between them and it. Uses the same intervals and
/// formula as the shader.
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
        let seen = shadow
            .map
            .get(sample * SHADOW_MAP_LEN_2D..(sample + 1) * SHADOW_MAP_LEN_2D)
            .is_none_or(|map| sees(map, away, shadow.reach));
        if seen {
            visible += 1.0;
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let divisor = count as f32;
    visible / divisor
}

/// Whether a source with this map sees the point `away` from it, allowing the
/// light `reach` further than the blocking edge.
#[must_use]
pub fn sees(map: &[f32], away: [f32; 2], reach: f32) -> bool {
    let distance = away[0].hypot(away[1]);
    let angle = away[1].atan2(away[0]);
    // The last interval that starts at or before this direction.
    let (mut low, mut high) = (0, SHADOW_INTERVALS_2D - 1);
    while low < high {
        let middle = (low + high).div_ceil(2);
        if map[middle * SHADOW_INTERVAL_FLOATS_2D] <= angle {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let at = low * SHADOW_INTERVAL_FLOATS_2D;
    let line = map[at + 3];
    if line < 0.0 {
        return true;
    }
    let facing = (map[at + 1] * away[0] + map[at + 2] * away[1]) / distance.max(1.0e-9);
    distance <= line / facing.max(1.0e-6) + reach
}

/// The visibility map of a light at `origin` reaching `radius`, blocked by
/// `edges`, each a segment between two points in the plane.
///
/// An angular sweep: the direction of every edge end cuts the circle into
/// intervals within which the nearest edge cannot change, and the edge nearest
/// the light along each interval's middle ray is the one that blocks it.
/// Neighboring intervals behind the same edge are one.
/// Edges entirely outside the radius, and edges the light stands on, are skipped.
#[must_use]
pub fn shadow_map_2d(origin: [f32; 2], radius: f32, edges: &[[[f32; 2]; 2]]) -> Vec<f32> {
    let near: Vec<([[f32; 2]; 2], [f32; 3])> = edges
        .iter()
        .filter(|edge| distance_to_segment(origin, **edge) < radius)
        .filter_map(|edge| line_of(origin, *edge).map(|line| (*edge, line)))
        .collect();
    let mut starts = vec![-PI];
    for (edge, _) in &near {
        for end in edge {
            starts.push((end[1] - origin[1]).atan2(end[0] - origin[0]));
        }
    }
    starts.sort_by(f32::total_cmp);
    starts.dedup_by(|a, b| (*a - *b).abs() < 1.0e-7);

    // (start angle, the edge that blocks from there, if any)
    let mut intervals: Vec<(f32, Option<usize>)> = Vec::new();
    for (index, start) in starts.iter().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(PI);
        let middle = (start + end) * 0.5;
        let ray = [middle.cos(), middle.sin()];
        let blocked_by = near
            .iter()
            .enumerate()
            .filter_map(|(edge, (segment, _))| {
                ray_hits(origin, ray, *segment).map(|distance| (distance, edge))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, edge)| edge);
        if intervals.last().is_some_and(|last| last.1 == blocked_by) {
            continue;
        }
        intervals.push((*start, blocked_by));
    }
    fold_narrowest(&mut intervals);

    let mut map = Vec::with_capacity(SHADOW_MAP_LEN_2D);
    for (start, edge) in &intervals {
        let [nx, ny, d] = edge.map_or([0.0, 0.0, -1.0], |edge| near[edge].1);
        map.extend([*start, nx, ny, d]);
    }
    for _ in intervals.len()..SHADOW_INTERVALS_2D {
        map.extend([PAST_THE_END, 0.0, 0.0, -1.0]);
    }
    map
}

/// Keeps a map within [`SHADOW_INTERVALS_2D`] by giving the narrowest interval
/// to the one before it, until it fits. Never removes the first, which starts the
/// circle.
fn fold_narrowest(intervals: &mut Vec<(f32, Option<usize>)>) {
    while intervals.len() > SHADOW_INTERVALS_2D {
        let width = |at: usize| intervals.get(at + 1).map_or(PI, |next| next.0) - intervals[at].0;
        let narrowest = (1..intervals.len())
            .min_by(|a, b| width(*a).total_cmp(&width(*b)))
            .unwrap_or(1);
        intervals.remove(narrowest);
    }
}

/// The line of an edge as seen from `origin`: its unit normal, turned to face
/// away from the light, and the line's distance from it. `None` when the light
/// is on the edge's line, where no ray crosses it, or the edge has no length.
fn line_of(origin: [f32; 2], [a, b]: [[f32; 2]; 2]) -> Option<[f32; 3]> {
    let along = [b[0] - a[0], b[1] - a[1]];
    let length = along[0].hypot(along[1]);
    if length <= f32::EPSILON {
        return None;
    }
    let mut normal = [-along[1] / length, along[0] / length];
    let mut distance = normal[0] * (a[0] - origin[0]) + normal[1] * (a[1] - origin[1]);
    if distance < 0.0 {
        normal = [-normal[0], -normal[1]];
        distance = -distance;
    }
    (distance > 1.0e-6).then_some([normal[0], normal[1], distance])
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
