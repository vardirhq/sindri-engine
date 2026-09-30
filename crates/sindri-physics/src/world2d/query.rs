//! Closest collider hits, expressed only in Sindri types.

use rapier2d::parry::query::Ray;

use super::{PhysicsWorld2d, r2};
use crate::validate::{finite2, non_negative};
use crate::{PhysicsError, RayHit2d, RaycastFilter2d};
use sindri_core::EntityId;

impl PhysicsWorld2d {
    /// Casts a finite world-space segment against registered collider pieces.
    ///
    /// Direction is normalized; distance is in world units. The mask selects
    /// collider memberships, independently of their physical interaction filter.
    /// Starting inside gives a zero-distance hit with a zero normal. Exact ties
    /// prefer the smaller entity handle, then authored piece order.
    ///
    /// # Errors
    /// Rejects non-finite values, negative distance and a zero direction.
    pub fn raycast(
        &self,
        origin: [f32; 2],
        direction: [f32; 2],
        max_distance: f32,
        filter: RaycastFilter2d,
    ) -> Result<Option<RayHit2d>, PhysicsError> {
        self.raycast_where(origin, direction, max_distance, filter, |_| true)
    }

    /// Casts while also excluding entities rejected by the caller.
    ///
    /// Scene hosts use this to omit entities despawned or disabled since their
    /// last physics synchronization. Geometry comes from current body poses;
    /// newly authored colliders appear at the next synchronization.
    ///
    /// # Errors
    /// Has the same validation contract as [`Self::raycast`].
    pub fn raycast_where(
        &self,
        origin: [f32; 2],
        direction: [f32; 2],
        max_distance: f32,
        filter: RaycastFilter2d,
        mut include: impl FnMut(EntityId) -> bool,
    ) -> Result<Option<RayHit2d>, PhysicsError> {
        let ray = checked_ray(origin, direction, max_distance)?;
        let mut closest: Option<RayHit2d> = None;
        // A direct scan also works before the first step, without a stale
        // broad-phase index. Compound pieces retain their authored order.
        for (&entity, record) in &self.bodies {
            if filter.exclude == Some(entity) || !include(entity) {
                continue;
            }
            let body = &self.backend.bodies[record.body];
            for &handle in &record.colliders {
                let collider = &self.backend.colliders[handle];
                if !collider.is_enabled()
                    || (!filter.include_sensors && collider.is_sensor())
                    || collider.collision_groups().memberships.bits() & filter.mask == 0
                {
                    continue;
                }
                let pose = collider.position_wrt_parent().map_or(*collider.position(), |local| {
                    *body.position() * *local
                });
                let Some(hit) = collider.shape().cast_ray_and_get_normal(&pose, &ray, max_distance, true) else {
                    continue;
                };
                let distance = hit.time_of_impact;
                if closest.as_ref().is_some_and(|old| {
                    distance.total_cmp(&old.distance).then(entity.cmp(&old.entity)).is_ge()
                }) {
                    continue;
                }
                let point = ray.point_at(distance);
                let normal = if distance <= 0.0 { r2::Vector::ZERO } else { hit.normal };
                closest = Some(RayHit2d {
                    entity,
                    point: [point.x, point.y],
                    normal: [normal.x, normal.y],
                    distance,
                });
            }
        }
        Ok(closest)
    }
}

fn checked_ray(origin: [f32; 2], direction: [f32; 2], distance: f32) -> Result<Ray, PhysicsError> {
    finite2("ray_origin", origin)?;
    finite2("ray_direction", direction)?;
    non_negative("ray_distance", distance)?;
    // Normalize in f64 so even the smallest/largest finite f32 directions work.
    let length = f64::from(direction[0]).hypot(f64::from(direction[1]));
    if length <= 0.0 {
        return Err(PhysicsError::NonPositive("ray_direction_length"));
    }
    #[allow(clippy::cast_possible_truncation)]
    let direction = [
        (f64::from(direction[0]) / length) as f32,
        (f64::from(direction[1]) / length) as f32,
    ];
    let ray = Ray::new(r2::Vector::new(origin[0], origin[1]), r2::Vector::new(direction[0], direction[1]));
    let end = ray.point_at(distance);
    finite2("ray_endpoint", [end.x, end.y])?;
    Ok(ray)
}
