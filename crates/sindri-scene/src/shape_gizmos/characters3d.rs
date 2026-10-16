//! Selected 3D footing guides from the same scaled probe the scene simulates.
#[cfg(test)]
mod tests;

use glam::{Quat, Vec3};
use sindri_core::{ComponentSchemaRegistry, World};
use sindri_physics::ColliderShape3d;

use super::{GizmoKind, GizmoStroke, ShapeGizmo, circle};
use crate::characters3d::SceneCharacters3d;

pub(super) fn characters(world: &World, components: &ComponentSchemaRegistry) -> Vec<ShapeGizmo> {
    // Controller geometry is validated as one batch by the runtime too. An
    // invalid controller leaves these guides absent rather than misleading.
    SceneCharacters3d::plan(world, components)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(entity, character)| {
            let transform = world.world_transform(entity)?;
            let body = Quat::from_array(transform.rotation).normalize();
            let turn = body * Quat::from_array(character.probe.rotation).normalize();
            let centre = Vec3::from_array(transform.position)
                + body * Vec3::from_array(character.probe.offset);
            let options = character.settings.movement;
            let up = Vec3::from_array(options.up).normalize();
            let side = up.any_orthonormal_vector();
            let across = up.cross(side);
            let extent = |axis: Vec3| extent(character.probe.shape, turn, axis);
            let foot = centre - up * extent(up);
            let radius = extent(side).max(extent(across)).max(0.1);
            let (sin, cos) = options.max_slope_angle.sin_cos();
            let reach = radius * 1.5;
            let mut strokes = Vec::new();
            // Four slope rays and their ring make the same guide legible from
            // any camera angle; they do not predict actual support or step success.
            for direction in [side, -side, across, -across] {
                strokes.push(GizmoStroke::open(vec![
                    foot.to_array(),
                    (foot + (direction * cos + up * sin) * reach).to_array(),
                ]));
            }
            strokes.push(circle(foot + up * sin * reach, side, across, cos * reach));
            if options.step_height > 0.0 {
                strokes.push(circle(
                    foot + up * options.step_height,
                    side,
                    across,
                    radius,
                ));
            }
            if options.snap_distance > 0.0 {
                strokes.push(circle(
                    foot - up * options.snap_distance,
                    side,
                    across,
                    radius,
                ));
            }
            if !foot.is_finite()
                || strokes
                    .iter()
                    .flat_map(|stroke| &stroke.points)
                    .flatten()
                    .any(|value| !value.is_finite())
            {
                return None;
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

/// Exact projection half-extent of the supported primitive, not a sampled ring.
fn extent(shape: ColliderShape3d, turn: Quat, axis: Vec3) -> f32 {
    let local = turn.inverse() * axis;
    match shape {
        ColliderShape3d::Box { half_extents } => Vec3::from_array(half_extents).dot(local.abs()),
        ColliderShape3d::Sphere { radius } => radius,
        ColliderShape3d::Capsule {
            half_height,
            radius,
        } => half_height * local.y.abs() + radius,
    }
}
