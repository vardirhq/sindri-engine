//! Exact queries over current body/local poses, independent of solver caches.

use rapier3d::parry::{query::Ray, shape::Shape};
use sindri_core::EntityId;

use super::{PhysicsWorld3d, r3, validation::finite3};
use crate::{PhysicsError, RayHit3d, RaycastFilter3d, validate::non_negative};

impl PhysicsWorld3d {
    /// Casts a finite segment; direction is normalized and distance is inclusive.
    /// Inside/on hits have zero distance/normal. Exact ties prefer entity then
    /// authored piece order. Geometry reflects insertion/teleport before stepping.
    ///
    /// # Errors
    /// Rejects non-finite input/endpoints, negative distance and zero direction.
    pub fn raycast(
        &self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_distance: f32,
        filter: RaycastFilter3d,
    ) -> Result<Option<RayHit3d>, PhysicsError> {
        self.raycast_where(origin, direction, max_distance, filter, |_| true)
    }

    /// Also excludes whole entities rejected by a stable host predicate.
    ///
    /// # Errors
    /// Has the validation contract of [`Self::raycast`].
    pub fn raycast_where(
        &self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_distance: f32,
        filter: RaycastFilter3d,
        mut include: impl FnMut(EntityId) -> bool,
    ) -> Result<Option<RayHit3d>, PhysicsError> {
        let ray = checked_ray(origin, direction, max_distance)?;
        let mut closest: Option<RayHit3d> = None;
        self.each_piece(filter, &mut include, |entity, piece, pose| {
            let Some(hit) = piece.cast_ray_and_get_normal(&pose, &ray, max_distance, true) else {
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
            closest = Some(RayHit3d {
                entity,
                point: ray.point_at(distance).to_array(),
                normal: if distance <= 0.0 {
                    [0.0; 3]
                } else {
                    hit.normal.to_array()
                },
                distance,
            });
        });
        Ok(closest)
    }

    pub(super) fn each_piece(
        &self,
        filter: RaycastFilter3d,
        include: &mut impl FnMut(EntityId) -> bool,
        mut visit: impl FnMut(EntityId, &dyn Shape, r3::Pose),
    ) {
        // Establish exact semantics first; a query-only index is a later slice.
        let mut entities: Vec<_> = self.bodies.keys().copied().collect();
        entities.sort_unstable();
        for entity in entities {
            if filter.exclude == Some(entity) || !include(entity) {
                continue;
            }
            let record = &self.bodies[&entity];
            let body = &self.backend.bodies[record.body];
            for &handle in &record.colliders {
                let collider = &self.backend.colliders[handle];
                if !collider.is_enabled()
                    || (!filter.include_sensors && collider.is_sensor())
                    || collider.collision_groups().memberships.bits() & filter.mask == 0
                {
                    continue;
                }
                let pose = collider
                    .position_wrt_parent()
                    .map_or(*collider.position(), |local| *body.position() * *local);
                visit(entity, collider.shape(), pose);
            }
        }
    }
}

pub(super) fn checked_ray(
    origin: [f32; 3],
    direction: [f32; 3],
    distance: f32,
) -> Result<Ray, PhysicsError> {
    finite3("query_origin", origin)?;
    finite3("query_direction", direction)?;
    non_negative("query_distance", distance)?;
    let [x, y, z] = direction.map(f64::from);
    let length = x.hypot(y).hypot(z);
    if length <= 0.0 {
        return Err(PhysicsError::NonPositive("query_direction_length"));
    }
    // Normalized components lie in [-1, 1], including extreme finite inputs.
    #[allow(clippy::cast_possible_truncation)]
    let direction = direction.map(|part| (f64::from(part) / length) as f32);
    let ray = Ray::new(
        r3::Vector::from_array(origin),
        r3::Vector::from_array(direction),
    );
    finite3("query_endpoint", ray.point_at(distance).to_array())?;
    Ok(ray)
}
