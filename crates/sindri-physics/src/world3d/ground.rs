//! Deterministic support queries over the same current-pose per-piece index.
use rapier3d::parry::bounding_volume::BoundingVolume;
use rapier3d::parry::query::{self, ShapeCastOptions};
use sindri_core::EntityId;

use super::{
    PhysicsWorld3d, build, r3,
    sweep::query_shape,
    validation::{finite3, validate_pose},
};
use crate::{
    ColliderShape3d, GroundOptions3d, GroundProbe3d, PhysicsError, PhysicsPose3d, RaycastFilter3d,
    ShapeHit3d,
};

impl PhysicsWorld3d {
    /// Finds the nearest downward surface and classifies its normal against up.
    /// Zero travel supports touching/inside-skin contacts. Initial penetration
    /// blocks classification; steep support is returned rather than skipped.
    /// Does not snap, carry, change velocity or mutate any body.
    ///
    /// # Errors
    /// Rejects invalid input, overflowing bounds and nonfinite hit geometry.
    pub fn probe_ground(
        &self,
        shape: ColliderShape3d,
        pose: PhysicsPose3d,
        options: GroundOptions3d,
        filter: RaycastFilter3d,
    ) -> Result<GroundProbe3d, PhysicsError> {
        self.probe_ground_where(shape, pose, options, filter, |_| true)
    }

    /// [`Self::probe_ground`] with a stable whole-entity predicate.
    /// Spatial candidates are visited once per entity; all pieces use its answer.
    ///
    /// # Errors
    /// Has the validation contract of [`Self::probe_ground`].
    pub fn probe_ground_where(
        &self,
        shape: ColliderShape3d,
        pose: PhysicsPose3d,
        options: GroundOptions3d,
        filter: RaycastFilter3d,
        mut include: impl FnMut(EntityId) -> bool,
    ) -> Result<GroundProbe3d, PhysicsError> {
        let shape = query_shape(shape)?;
        validate_pose(pose)?;
        options.validate()?;
        // Character movement also refuses unbounded spatial pieces rather than
        // silently missing them. Ordinary ray/overlap fallback stays unchanged.
        self.spatial.character_tree()?;
        let at = build::pose(pose);
        let up = r3::Vector::from_array(options.up).normalize();
        let mut end = at;
        end.translation -= up * options.max_distance;
        finite3("ground_destination", end.translation.to_array())?;
        let prediction = options.skin + (options.skin * 0.01 + f32::EPSILON);
        let bounds = shape
            .compute_aabb(&at)
            .merged(&shape.compute_aabb(&end))
            .loosened(prediction);
        finite3("ground_bounds", bounds.mins.to_array())?;
        finite3("ground_bounds", bounds.maxs.to_array())?;
        if !shape.compute_local_aabb().extents().length().is_finite() {
            return Err(PhysicsError::NonFinite("ground_probe_extent"));
        }
        let cast = ShapeCastOptions {
            max_time_of_impact: options.max_distance,
            target_distance: options.skin,
            stop_at_penetration: false,
            compute_impact_geometry_on_penetration: true,
        };
        let mut first = None;
        let mut penetration = None;
        self.visit_candidates(
            self.spatial.area(bounds),
            filter,
            &mut include,
            |entity, piece, placed| {
                let contact = query::contact(&at, shape.as_ref(), &placed, piece, prediction)
                    .ok()
                    .flatten();
                if contact
                    .as_ref()
                    .is_some_and(|contact| contact.dist < -f32::EPSILON)
                {
                    penetration.get_or_insert(ShapeHit3d {
                        entity,
                        point: at.translation.to_array(),
                        normal: [0.0; 3],
                        distance: 0.0,
                    });
                    return;
                }
                let hit = if let Some(contact) =
                    contact.filter(|contact| contact.normal2.dot(up) > f32::EPSILON)
                {
                    Some(ShapeHit3d {
                        entity,
                        point: contact.point2.to_array(),
                        normal: contact.normal2.to_array(),
                        distance: 0.0,
                    })
                } else {
                    query::cast_shapes(
                        &at,
                        -up,
                        shape.as_ref(),
                        &placed,
                        r3::Vector::ZERO,
                        piece,
                        cast,
                    )
                    .ok()
                    .flatten()
                    .map(|hit| ShapeHit3d {
                        entity,
                        point: placed.transform_point(hit.witness2).to_array(),
                        normal: (placed.rotation * hit.normal2).to_array(),
                        distance: hit.time_of_impact,
                    })
                };
                if let Some(hit) = hit {
                    select(&mut first, hit);
                }
            },
        );
        classify(first, penetration, up, options.max_slope_angle)
    }
}

fn select(first: &mut Option<ShapeHit3d>, hit: ShapeHit3d) {
    if first.as_ref().is_none_or(|old| {
        hit.distance
            .total_cmp(&old.distance)
            .then(hit.entity.cmp(&old.entity))
            .is_lt()
    }) {
        *first = Some(hit);
    }
}

fn classify(
    first: Option<ShapeHit3d>,
    penetration: Option<ShapeHit3d>,
    up: r3::Vector,
    max_slope_angle: f32,
) -> Result<GroundProbe3d, PhysicsError> {
    let hit = penetration.or(first);
    if let Some(hit) = hit {
        finite3("ground_hit_point", hit.point)?;
        finite3("ground_hit_normal", hit.normal)?;
        if !r3::Vector::from_array(hit.normal).length().is_finite() {
            return Err(PhysicsError::NonFinite("ground_hit_normal_length"));
        }
        crate::validate::non_negative("ground_hit_distance", hit.distance)?;
    }
    let walkable = penetration.is_none()
        && hit.is_some_and(|hit| {
            let normal = r3::Vector::from_array(hit.normal);
            let length = normal.length();
            length > f32::EPSILON && {
                let alignment = (normal / length).dot(up);
                alignment > f32::EPSILON && alignment + 1.0e-6 >= max_slope_angle.cos()
            }
        });
    Ok(GroundProbe3d {
        hit,
        walkable,
        started_penetrating: penetration.is_some(),
    })
}
