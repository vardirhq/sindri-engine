//! What a character stands on and how far an effect throws, worked out from
//! the numbers the simulation runs on.

use glam::Vec3;
use sindri_core::{ComponentSchemaRegistry, World};

use super::{GizmoKind, GizmoStroke, ShapeGizmo, circle};
use crate::{Character2dComponent, EffectBurstComponent, collision_shapes};

/// How far a burst's fastest fleck gets before it dies, in world units.
///
/// A fleck keeps `1 - drag * dt` of its speed each step, which at small steps
/// is `e^(-drag t)`: integrated over its life, `v (1 - e^(-drag L)) / drag`.
/// A step moves a fleck before slowing it, so at sixty steps a second a
/// fleck can land a percent or so past this; the ring is a guide to the
/// spray's size, not a wall.
#[must_use]
pub fn effect_reach(speed: f32, lifetime: f32, drag: f32) -> f32 {
    let lifetime = lifetime.max(0.0);
    if drag <= f32::EPSILON {
        return speed * lifetime;
    }
    speed * (1.0 - (-drag * lifetime).exp()) / drag
}

pub(super) fn effects(world: &World, components: &ComponentSchemaRegistry) -> Vec<ShapeGizmo> {
    components
        .query::<EffectBurstComponent>(world)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(entity, burst)| {
            let at = Vec3::from_array(world.world_transform(entity)?.position);
            // Every fleck leaves at `speed (1 ± spread)`: the slowest and the
            // fastest bound the ring the spray lands in.
            let spread = burst.spread.max(0.0);
            let reach = |scale: f32| effect_reach(burst.speed * scale, burst.lifetime, burst.drag);
            let mut strokes = vec![circle(at, Vec3::X, Vec3::Y, reach(1.0 + spread))];
            let nearest = reach((1.0 - spread).max(0.0));
            if spread > 0.0 && nearest > 0.0 {
                strokes.push(circle(at, Vec3::X, Vec3::Y, nearest));
            }
            Some(ShapeGizmo {
                entity,
                kind: GizmoKind::EffectReach,
                strokes,
                marks: Vec::new(),
            })
        })
        .collect()
}

pub(super) fn characters(world: &World, components: &ComponentSchemaRegistry) -> Vec<ShapeGizmo> {
    let characters = components
        .query::<Character2dComponent>(world)
        .unwrap_or_default();
    if characters.is_empty() {
        return Vec::new();
    }
    let shapes = collision_shapes(world, components).unwrap_or_default();
    characters
        .into_iter()
        .filter_map(|(entity, character)| {
            let shape = shapes.iter().find(|shape| shape.entity == entity)?;
            let depth = world
                .world_transform(entity)
                .map_or(0.0, |transform| transform.position[2]);
            let up = Vec3::new(character.up[0], character.up[1], 0.0).normalize_or(Vec3::Y);
            let side = Vec3::new(up.y, -up.x, 0.0);
            let centre = Vec3::new(shape.pose.position[0], shape.pose.position[1], depth);
            // The collider's extent along up and across it, measured from its
            // own outline: the foot is its lowest point.
            let mut outline = shape.outlines().flatten().peekable();
            outline.peek()?;
            let (mut low, mut left, mut right) = (f32::MAX, f32::MAX, f32::MIN);
            for [x, y] in outline {
                let offset = Vec3::new(x, y, depth) - centre;
                low = low.min(offset.dot(up));
                left = left.min(offset.dot(side));
                right = right.max(offset.dot(side));
            }
            let foot = centre + up * low + side * f32::midpoint(left, right);
            let half_width = ((right - left) / 2.0).max(0.1);
            let across = |height: f32| {
                let at = foot + up * height;
                GizmoStroke::open(vec![
                    (at - side * half_width).to_array(),
                    (at + side * half_width).to_array(),
                ])
            };
            let (sin, cos) = character.max_slope_angle.sin_cos();
            let mut strokes = Vec::new();
            // The steepest ground it still stands on, rising either way.
            for way in [side, -side] {
                let slope = way * cos + up * sin;
                strokes.push(GizmoStroke::open(vec![
                    foot.to_array(),
                    (foot + slope * half_width * 1.5).to_array(),
                ]));
            }
            if character.step_height > 0.0 {
                strokes.push(across(character.step_height));
            }
            if character.snap_distance > 0.0 {
                strokes.push(across(-character.snap_distance));
            }
            Some(ShapeGizmo {
                entity,
                kind: GizmoKind::Character,
                strokes,
                marks: vec![foot.to_array()],
            })
        })
        .collect()
}
