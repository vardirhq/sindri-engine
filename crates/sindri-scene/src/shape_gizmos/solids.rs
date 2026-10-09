//! A 3D collider's pieces, drawn as wireframes where the 3D physics world
//! puts them: a box's twelve edges, a sphere's three great circles, a
//! capsule's two end rings, its sides and its rounded ends.

use glam::{Quat, Vec3};
use sindri_core::{ComponentSchemaRegistry, World};
use sindri_physics::{Collider3d, ColliderShape3d, PhysicsPose3d};

use super::{GizmoKind, GizmoStroke, ShapeGizmo, arc, circle};
use crate::physics_sync3d::pose_of;
use crate::{Collider3dComponent, RigidBody3dComponent};

pub(super) fn colliders(world: &World, components: &ComponentSchemaRegistry) -> Vec<ShapeGizmo> {
    components
        .query::<Collider3dComponent>(world)
        .unwrap_or_default()
        .into_iter()
        .map(|(entity, collider)| {
            let body = components
                .get::<RigidBody3dComponent>(world, entity)
                .ok()
                .flatten()
                .map(|authored| authored.0);
            let pose = pose_of(world, entity, body);
            ShapeGizmo {
                entity,
                kind: GizmoKind::Collider3d,
                strokes: collider
                    .0
                    .iter()
                    .flat_map(|piece| wireframe(pose, piece))
                    .collect(),
                marks: Vec::new(),
            }
        })
        .collect()
}

/// One piece's lines, in the world.
pub(super) fn wireframe(pose: PhysicsPose3d, piece: &Collider3d) -> Vec<GizmoStroke> {
    let body = Quat::from_array(pose.rotation).normalize();
    let turn = body * Quat::from_array(piece.rotation).normalize();
    let centre = Vec3::from_array(pose.position) + body * Vec3::from_array(piece.offset);
    let (x, y, z) = (turn * Vec3::X, turn * Vec3::Y, turn * Vec3::Z);
    match piece.shape {
        ColliderShape3d::Box { half_extents } => {
            let [hx, hy, hz] = half_extents;
            let corner = |sx: f32, sy: f32, sz: f32| {
                (centre + x * (hx * sx) + y * (hy * sy) + z * (hz * sz)).to_array()
            };
            let face = |sz: f32| {
                GizmoStroke::closed(vec![
                    corner(-1.0, -1.0, sz),
                    corner(1.0, -1.0, sz),
                    corner(1.0, 1.0, sz),
                    corner(-1.0, 1.0, sz),
                ])
            };
            let mut strokes = vec![face(-1.0), face(1.0)];
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                strokes.push(GizmoStroke::open(vec![
                    corner(sx, sy, -1.0),
                    corner(sx, sy, 1.0),
                ]));
            }
            strokes
        }
        ColliderShape3d::Sphere { radius } => vec![
            circle(centre, x, y, radius),
            circle(centre, y, z, radius),
            circle(centre, z, x, radius),
        ],
        ColliderShape3d::Capsule {
            half_height,
            radius,
        } => {
            // Upright along the piece's own Y, as physics builds it.
            let top = centre + y * half_height;
            let bottom = centre - y * half_height;
            let half = std::f32::consts::PI;
            let mut strokes = vec![circle(top, x, z, radius), circle(bottom, x, z, radius)];
            for side in [x, -x, z, -z] {
                strokes.push(GizmoStroke::open(vec![
                    (top + side * radius).to_array(),
                    (bottom + side * radius).to_array(),
                ]));
            }
            for across in [x, z] {
                strokes.push(GizmoStroke::open(arc(top, across, y, radius, 0.0, half)));
                strokes.push(GizmoStroke::open(arc(
                    bottom, across, -y, radius, 0.0, half,
                )));
            }
            strokes
        }
    }
}
