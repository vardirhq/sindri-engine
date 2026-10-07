//! Exact 3D overlaps and fixed-orientation sweeps.

use rapier3d::parry::bounding_volume::{Aabb, BoundingVolume};
use rapier3d::parry::query::{self, ShapeCastOptions};
use rapier3d::parry::shape::Shape;
use sindri_core::EntityId;

use super::{PhysicsWorld3d, build, query::checked_ray, r3, validation::validate_pose};
use crate::validate::{non_negative, positive};
use crate::{ColliderShape3d, PhysicsError, PhysicsPose3d, RaycastFilter3d, ShapeHit3d};

impl PhysicsWorld3d {
    /// Returns overlapping entity handles once each, sorted by handle.
    ///
    /// # Errors
    /// Rejects invalid pose/quaternion or shape dimensions.
    pub fn overlap(
        &self,
        shape: ColliderShape3d,
        pose: PhysicsPose3d,
        filter: RaycastFilter3d,
    ) -> Result<Vec<EntityId>, PhysicsError> {
        self.overlap_where(shape, pose, filter, |_| true)
    }

    /// Also excludes whole entities rejected by a stable host predicate.
    /// Only spatial candidates are visited, once per entity.
    ///
    /// # Errors
    /// Has the validation contract of [`Self::overlap`].
    pub fn overlap_where(
        &self,
        shape: ColliderShape3d,
        pose: PhysicsPose3d,
        filter: RaycastFilter3d,
        mut include: impl FnMut(EntityId) -> bool,
    ) -> Result<Vec<EntityId>, PhysicsError> {
        let probe = query_shape(shape)?;
        validate_pose(pose)?;
        let at = build::pose(pose);
        let mut found = Vec::new();
        self.visit_candidates(
            self.spatial.area(probe.compute_aabb(&at)),
            filter,
            &mut include,
            |entity, piece, piece_pose| {
                if found.last() != Some(&entity)
                    && query::intersection_test(&at, probe.as_ref(), &piece_pose, piece)
                        .is_ok_and(|intersection| intersection.intersecting)
                {
                    found.push(entity);
                }
            },
        );
        Ok(found)
    }

    /// Sweeps without rotation; reports the closest piece in world units.
    /// Inside/on hits have zero distance/normal and the probe origin as point.
    /// Exact ties prefer entity handle then authored piece order.
    ///
    /// # Errors
    /// Rejects invalid shape/pose, non-finite input/endpoints, negative distance
    /// and zero direction. A query capsule may have zero half-height.
    pub fn shape_cast(
        &self,
        shape: ColliderShape3d,
        pose: PhysicsPose3d,
        direction: [f32; 3],
        max_distance: f32,
        filter: RaycastFilter3d,
    ) -> Result<Option<ShapeHit3d>, PhysicsError> {
        self.shape_cast_where(shape, pose, direction, max_distance, filter, |_| true)
    }

    /// Also excludes whole entities rejected by a stable host predicate.
    /// Only spatial candidates are visited, once per entity.
    ///
    /// # Errors
    /// Has the validation contract of [`Self::shape_cast`].
    pub fn shape_cast_where(
        &self,
        shape: ColliderShape3d,
        pose: PhysicsPose3d,
        direction: [f32; 3],
        max_distance: f32,
        filter: RaycastFilter3d,
        mut include: impl FnMut(EntityId) -> bool,
    ) -> Result<Option<ShapeHit3d>, PhysicsError> {
        let probe = query_shape(shape)?;
        validate_pose(pose)?;
        let at = build::pose(pose);
        let ray = checked_ray(pose.position, direction, max_distance)?;
        let options = ShapeCastOptions {
            max_time_of_impact: max_distance,
            stop_at_penetration: true,
            compute_impact_geometry_on_penetration: false,
            ..ShapeCastOptions::default()
        };
        let mut closest: Option<ShapeHit3d> = None;
        self.visit_candidates(
            self.spatial
                .area(swept_bounds(probe.as_ref(), at, ray.dir * max_distance)),
            filter,
            &mut include,
            |entity, piece, piece_pose| {
                let Ok(Some(hit)) = query::cast_shapes(
                    &at,
                    ray.dir,
                    probe.as_ref(),
                    &piece_pose,
                    r3::Vector::ZERO,
                    piece,
                    options,
                ) else {
                    return;
                };
                let distance = hit.time_of_impact;
                if closest.as_ref().is_some_and(|old| {
                    distance
                        .total_cmp(&old.distance)
                        .then(entity.cmp(&old.entity))
                        .is_ge()
                }) {
                    return;
                }
                let (point, normal) = if distance <= 0.0 {
                    (at.translation, r3::Vector::ZERO)
                } else {
                    (
                        piece_pose.transform_point(hit.witness2),
                        piece_pose.rotation * hit.normal2,
                    )
                };
                closest = Some(ShapeHit3d {
                    entity,
                    point: point.to_array(),
                    normal: normal.to_array(),
                    distance,
                });
            },
        );
        Ok(closest)
    }
}

fn query_shape(shape: ColliderShape3d) -> Result<r3::SharedShape, PhysicsError> {
    Ok(match shape {
        ColliderShape3d::Box {
            half_extents: [x, y, z],
        } => {
            positive("query_half_extent_x", x)?;
            positive("query_half_extent_y", y)?;
            positive("query_half_extent_z", z)?;
            r3::SharedShape::cuboid(x, y, z)
        }
        ColliderShape3d::Sphere { radius } => {
            positive("query_radius", radius)?;
            r3::SharedShape::ball(radius)
        }
        ColliderShape3d::Capsule {
            half_height,
            radius,
        } => {
            non_negative("query_half_height", half_height)?;
            positive("query_radius", radius)?;
            r3::SharedShape::capsule_y(half_height, radius)
        }
    })
}

fn swept_bounds(shape: &dyn Shape, start: r3::Pose, displacement: r3::Vector) -> Aabb {
    let mut end = start;
    end.translation += displacement;
    shape.compute_aabb(&start).merged(&shape.compute_aabb(&end))
}
