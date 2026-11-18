//! 3D collider pieces as wireframes: a box's twelve edges, a sphere's three
//! great circles, a capsule's rings, sides and caps.
//!
//! Placed as physics places them: the piece's offset and turn inside the
//! entity, then the entity's whole world transform, scale included, since a
//! 3D collider's dimensions are local units scaled by the composed transform.

use std::f32::consts::{PI, TAU};

use glam::{Mat4, Quat, Vec3};
use sindri_capture::ProjectPlayer;
use sindri_physics::ColliderShape3d;
use sindri_scene::Collider3dComponent;

/// Each 3D collider piece on an active entity, as polylines in world units,
/// with whether it is a sensor.
pub(super) fn wireframes(player: &ProjectPlayer) -> Vec<(bool, Vec<Vec<Vec3>>)> {
    let registry = player.scene.components();
    let mut found = Vec::new();
    for (entity, _) in player.world.entities() {
        let Ok(parts) = registry.get_parts::<Collider3dComponent>(&player.world, entity) else {
            continue;
        };
        let Some(place) = player.world.world_transform(entity) else {
            continue;
        };
        let body = Mat4::from_scale_rotation_translation(
            Vec3::from(place.scale),
            Quat::from_array(place.rotation),
            Vec3::from(place.position),
        );
        for (_, Collider3dComponent(pieces)) in parts {
            for piece in pieces {
                let local = body
                    * Mat4::from_rotation_translation(
                        Quat::from_array(piece.rotation),
                        Vec3::from(piece.offset),
                    );
                let lines: Vec<Vec<Vec3>> = shape(&piece.shape)
                    .into_iter()
                    .map(|line| {
                        line.into_iter()
                            .map(|point| local.transform_point3(point))
                            .collect()
                    })
                    .collect();
                found.push((piece.sensor, lines));
            }
        }
    }
    found
}

/// A shape's wireframe around its own middle, as polylines.
fn shape(shape: &ColliderShape3d) -> Vec<Vec<Vec3>> {
    const ROUND: usize = 32;
    match *shape {
        ColliderShape3d::Box {
            half_extents: [x, y, z],
        } => {
            let corner = |sx: f32, sy: f32, sz: f32| Vec3::new(sx * x, sy * y, sz * z);
            let mut edges = Vec::new();
            for (a, b) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                // Along x, along y and along z, from each of four corners.
                edges.push(vec![corner(-1.0, a, b), corner(1.0, a, b)]);
                edges.push(vec![corner(a, -1.0, b), corner(a, 1.0, b)]);
                edges.push(vec![corner(a, b, -1.0), corner(a, b, 1.0)]);
            }
            edges
        }
        ColliderShape3d::Sphere { radius } => vec![
            ring(Vec3::ZERO, radius, Plane::Xy, 0.0, TAU, ROUND),
            ring(Vec3::ZERO, radius, Plane::Xz, 0.0, TAU, ROUND),
            ring(Vec3::ZERO, radius, Plane::Yz, 0.0, TAU, ROUND),
        ],
        // Upright, as physics makes one.
        ColliderShape3d::Capsule {
            half_height,
            radius,
        } => {
            let (top, bottom) = (Vec3::Y * half_height, Vec3::NEG_Y * half_height);
            let mut lines = vec![
                ring(top, radius, Plane::Xz, 0.0, TAU, ROUND),
                ring(bottom, radius, Plane::Xz, 0.0, TAU, ROUND),
                ring(top, radius, Plane::Xy, 0.0, PI, ROUND / 2),
                ring(bottom, radius, Plane::Xy, PI, TAU, ROUND / 2),
                ring(top, radius, Plane::Yz, 0.0, PI, ROUND / 2),
                ring(bottom, radius, Plane::Yz, PI, TAU, ROUND / 2),
            ];
            for side in [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z] {
                lines.push(vec![top + side * radius, bottom + side * radius]);
            }
            lines
        }
    }
}

#[derive(Clone, Copy)]
enum Plane {
    Xy,
    Xz,
    Yz,
}

/// An arc of a circle in `plane`, from `from` to `to` radians.
fn ring(middle: Vec3, radius: f32, plane: Plane, from: f32, to: f32, steps: usize) -> Vec<Vec3> {
    (0..=steps)
        .map(|step| {
            #[allow(clippy::cast_precision_loss)]
            let angle = from + (to - from) * step as f32 / steps as f32;
            let (along, up) = (radius * angle.cos(), radius * angle.sin());
            middle
                + match plane {
                    // In the Yz plane the first axis is z, so a cap's arc
                    // rises in y as the others do.
                    Plane::Xy => Vec3::new(along, up, 0.0),
                    Plane::Xz => Vec3::new(along, 0.0, up),
                    Plane::Yz => Vec3::new(0.0, up, along),
                }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wireframes_have_their_shape() {
        let cube = shape(&ColliderShape3d::Box {
            half_extents: [1.0, 2.0, 3.0],
        });
        assert_eq!(cube.len(), 12, "a box has twelve edges");
        for edge in &cube {
            for point in edge {
                assert!((point.x.abs() - 1.0).abs() < 1e-6 && (point.y.abs() - 2.0).abs() < 1e-6);
            }
        }
        let ball = shape(&ColliderShape3d::Sphere { radius: 2.0 });
        assert!(
            ball.iter()
                .flatten()
                .all(|point| (point.length() - 2.0).abs() < 1e-4)
        );
        let capsule = shape(&ColliderShape3d::Capsule {
            half_height: 1.0,
            radius: 0.5,
        });
        let highest = capsule
            .iter()
            .flatten()
            .map(|point| point.y)
            .fold(f32::MIN, f32::max);
        let lowest = capsule
            .iter()
            .flatten()
            .map(|point| point.y)
            .fold(f32::MAX, f32::min);
        assert!(
            (highest - 1.5).abs() < 1e-4 && (lowest + 1.5).abs() < 1e-4,
            "{highest} {lowest}"
        );
    }
}
