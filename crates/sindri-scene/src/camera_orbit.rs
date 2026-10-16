//! Checked world-space orbit cameras, stepped by the shared runtime.

use sindri_core::{EntityId, SceneComponent, Transform3D, World};
use sindri_physics::{PhysicsWorld3d, RaycastFilter3d};

use crate::{CameraComponent, CameraOrbitComponent};

mod control;
pub use control::{
    clear_camera_orbit, set_camera_orbit, set_camera_orbit_collision, set_camera_orbit_offset,
    set_camera_orbit_smoothing,
};

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CameraOrbitError {
    #[error(
        "invalid orbit settings: finite values, positive distance, pitch between the poles, and padding below distance are required"
    )]
    Settings,
    #[error("orbit requires a camera, a transform and a distinct active target")]
    Target,
    #[error("orbit camera has invalid component data: {0}")]
    Component(String),
    #[error("orbit cannot invert a parent with zero scale")]
    ParentScale,
    #[error("orbit would move a Z-locked camera off its layer")]
    ZLock,
    #[error("orbit produced a non-finite transform")]
    NonFinite,
    #[error("orbit obstruction query failed: {0}")]
    Query(String),
}

/// Updates active cameras using the last synchronized 3D physics poses.
/// Target transforms reflect this step's gameplay writes. Invalid cameras are
/// reported and retain their complete transform. Sensors, the target and the
/// camera's own colliders are excluded. The ray protects the sight line only.
#[must_use]
pub fn update_orbit_cameras(
    world: &mut World,
    physics: &PhysicsWorld3d,
    dt: f32,
) -> Vec<(EntityId, CameraOrbitError)> {
    if !dt.is_finite() || dt <= 0.0 {
        return Vec::new();
    }
    let cameras: Vec<_> = world
        .entities()
        .filter_map(|(entity, data)| {
            let payload = data.components.get(CameraOrbitComponent::TYPE_NAME)?;
            world.is_active(entity).then(|| (entity, payload.clone()))
        })
        .collect();
    let mut errors = Vec::new();
    for (camera, payload) in cameras {
        let result = serde_json::from_value::<CameraOrbitComponent>(payload)
            .map_err(|error| CameraOrbitError::Component(error.to_string()))
            .and_then(|orbit| candidate(world, physics, camera, &orbit, dt));
        match result {
            Ok(next) => {
                if let Some(data) = world.get_mut(camera) {
                    data.transform_3d = Some(next);
                    // Orbit replaced the old 2D shake offset. Forget it only after
                    // committing a pose, so adding/removing orbit before a step is neutral.
                    if let Some(shake) = data
                        .components
                        .get_mut(crate::CameraBehaviorComponent::TYPE_NAME)
                        .and_then(|payload| payload.get_mut("shake"))
                        .and_then(serde_json::Value::as_object_mut)
                    {
                        shake.insert("offset".to_owned(), serde_json::json!([0.0, 0.0]));
                    }
                }
            }
            Err(error) => errors.push((camera, error)),
        }
    }
    errors
}

fn candidate(
    world: &World,
    physics: &PhysicsWorld3d,
    camera: EntityId,
    orbit: &CameraOrbitComponent,
    dt: f32,
) -> Result<Transform3D, CameraOrbitError> {
    if !orbit.is_valid() {
        return Err(CameraOrbitError::Settings);
    }
    let target = world
        .entity_for_source_id(&orbit.target)
        .filter(|target| *target != camera && world.is_active(*target))
        .ok_or(CameraOrbitError::Target)?;
    if world.check_set_parent(camera, Some(target)).is_err() {
        return Err(CameraOrbitError::Target);
    }
    let data = world.get(camera).ok_or(CameraOrbitError::Target)?;
    let local = data.transform_3d.ok_or(CameraOrbitError::Target)?;
    let valid_camera = data
        .components
        .get(CameraComponent::TYPE_NAME)
        .is_some_and(|payload| serde_json::from_value::<CameraComponent>(payload.clone()).is_ok());
    if !valid_camera {
        return Err(CameraOrbitError::Target);
    }
    if world
        .parent_space(camera)
        .scale
        .iter()
        .any(|part| part.abs() <= f32::EPSILON)
    {
        return Err(CameraOrbitError::ParentScale);
    }
    let focus = world
        .world_transform(target)
        .ok_or(CameraOrbitError::Target)?
        .position;
    let focus = std::array::from_fn(|i| narrow(f64::from(focus[i]) + f64::from(orbit.offset[i])));
    let mut placed = world
        .world_transform(camera)
        .ok_or(CameraOrbitError::Target)?;
    let (yaw_sin, yaw_cos) = orbit.yaw.sin_cos();
    let (pitch_sin, pitch_cos) = orbit.pitch.sin_cos();
    let direction = [yaw_sin * pitch_cos, pitch_sin, yaw_cos * pitch_cos];
    let desired: [f32; 3] = std::array::from_fn(|i| {
        narrow(f64::from(focus[i]) + f64::from(direction[i]) * f64::from(orbit.distance))
    });
    let alpha = if orbit.smoothing <= 0.0 {
        1.0
    } else {
        1.0 - (-f64::from(orbit.smoothing) * f64::from(dt)).exp()
    };
    placed.position = std::array::from_fn(|i| {
        narrow(f64::from(placed.position[i]) * (1.0 - alpha) + f64::from(desired[i]) * alpha)
    });
    // A camera initially at the focus needs a direction even when a solid starts there.
    if placed
        .position
        .into_iter()
        .zip(focus)
        .all(|(a, b)| (a - b).abs() <= 0.0)
    {
        placed.position = desired;
    }
    placed
        .look_at(focus)
        .map_err(|_| CameraOrbitError::NonFinite)?;
    pull_in(world, physics, camera, target, focus, orbit, &mut placed)?;
    let mut next = world.local_for_world(camera, placed);
    next.scale = local.scale;
    if !next
        .position
        .into_iter()
        .chain(next.rotation)
        .all(f32::is_finite)
    {
        return Err(CameraOrbitError::NonFinite);
    }
    if local.z_lock_rejects(Some(next)) {
        return Err(CameraOrbitError::ZLock);
    }
    Ok(next)
}

fn pull_in(
    world: &World,
    physics: &PhysicsWorld3d,
    camera: EntityId,
    target: EntityId,
    focus: [f32; 3],
    orbit: &CameraOrbitComponent,
    placed: &mut Transform3D,
) -> Result<(), CameraOrbitError> {
    if orbit.collision_mask == 0 {
        return Ok(());
    }
    let delta: [f64; 3] =
        std::array::from_fn(|i| f64::from(placed.position[i]) - f64::from(focus[i]));
    let distance = delta
        .into_iter()
        .map(|part| part * part)
        .sum::<f64>()
        .sqrt();
    if distance <= 0.0 || !distance.is_finite() || distance > f64::from(f32::MAX) {
        return Err(CameraOrbitError::NonFinite);
    }
    let direction = delta.map(|part| narrow(part / distance));
    let filter = RaycastFilter3d {
        mask: orbit.collision_mask,
        exclude: Some(target),
        ..RaycastFilter3d::default()
    };
    if let Some(hit) = physics
        .raycast_where(focus, direction, narrow(distance), filter, |entity| {
            entity != camera && world.is_active(entity)
        })
        .map_err(|error| CameraOrbitError::Query(error.to_string()))?
    {
        let allowed = (hit.distance - orbit.collision_padding).max(0.0);
        placed.position = std::array::from_fn(|i| focus[i] + direction[i] * allowed);
    }
    Ok(())
}

// Positions are checked for overflow before committing; normalized components fit f32.
#[allow(clippy::cast_possible_truncation)]
fn narrow(value: f64) -> f32 {
    value as f32
}
