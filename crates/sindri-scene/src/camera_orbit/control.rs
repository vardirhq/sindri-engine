//! Explicit-entity controls preserve unknown orbit payload fields.

use serde_json::Value;
use sindri_core::{EntityId, SceneComponent, World};

use crate::{CameraComponent, CameraOrbitComponent};

fn edit(world: &mut World, camera: EntityId, change: impl FnOnce(&mut Value)) -> bool {
    let Some(data) = world.get(camera) else {
        return false;
    };
    if data.transform_3d.is_none()
        || !data
            .components
            .get(CameraComponent::TYPE_NAME)
            .is_some_and(|payload| {
                serde_json::from_value::<CameraComponent>(payload.clone()).is_ok()
            })
    {
        return false;
    }
    let Some(payload) = data.components.get(CameraOrbitComponent::TYPE_NAME) else {
        return false;
    };
    let mut next = payload.clone();
    if !next.is_object() {
        return false;
    }
    change(&mut next);
    if !serde_json::from_value::<CameraOrbitComponent>(next.clone())
        .is_ok_and(|orbit| orbit.is_valid())
    {
        return false;
    }
    let Some(data) = world.get_mut(camera) else {
        return false;
    };
    data.components
        .insert(CameraOrbitComponent::TYPE_NAME.to_owned(), next);
    true
}

/// Creates or retargets an explicit camera's orbit, keeping its other settings.
/// Target IDs must be authored scene IDs. Failed changes leave both payload and pose intact.
pub fn set_camera_orbit(
    world: &mut World,
    camera: EntityId,
    target: EntityId,
    yaw: f32,
    pitch: f32,
    distance: f32,
) -> bool {
    let Some(target_id) = world.get(target).and_then(|data| data.source_id.clone()) else {
        return false;
    };
    let Some(data) = world.get(camera) else {
        return false;
    };
    if camera == target
        || world.check_set_parent(camera, Some(target)).is_err()
        || !world.is_active(target)
        || data.transform_3d.is_none()
        || world.world_transform(target).is_none()
        || !data
            .components
            .get(CameraComponent::TYPE_NAME)
            .is_some_and(|payload| {
                serde_json::from_value::<CameraComponent>(payload.clone()).is_ok()
            })
    {
        return false;
    }
    let mut payload = if let Some(payload) = data.components.get(CameraOrbitComponent::TYPE_NAME) {
        payload.clone()
    } else {
        let Ok(payload) = serde_json::to_value(CameraOrbitComponent::new(target_id.clone())) else {
            return false;
        };
        payload
    };
    if !payload.is_object() {
        return false;
    }
    payload["target"] = Value::from(target_id.as_str());
    payload["yaw"] = Value::from(yaw);
    payload["pitch"] = Value::from(pitch);
    payload["distance"] = Value::from(distance);
    if !serde_json::from_value::<CameraOrbitComponent>(payload.clone())
        .is_ok_and(|orbit| orbit.is_valid())
    {
        return false;
    }
    let Some(data) = world.get_mut(camera) else {
        return false;
    };
    data.components
        .insert(CameraOrbitComponent::TYPE_NAME.to_owned(), payload);
    true
}

/// Stops orbit without changing the camera's last pose. Existing 2D behavior resumes.
pub fn clear_camera_orbit(world: &mut World, camera: EntityId) -> bool {
    let Some(data) = world.get_mut(camera) else {
        return false;
    };
    if !data.components.contains_key(CameraComponent::TYPE_NAME) {
        return false;
    }
    data.components.remove(CameraOrbitComponent::TYPE_NAME);
    true
}

/// Sets a world-space offset from the target's world position.
pub fn set_camera_orbit_offset(world: &mut World, camera: EntityId, offset: [f32; 3]) -> bool {
    edit(world, camera, |payload| {
        payload["offset"] = serde_json::json!(offset);
    })
}

/// Sets position smoothing per second; zero snaps, including on the first step.
pub fn set_camera_orbit_smoothing(world: &mut World, camera: EntityId, smoothing: f32) -> bool {
    edit(world, camera, |payload| {
        payload["smoothing"] = Value::from(smoothing);
    })
}

/// Sets obstruction layers and pull-in padding; mask zero disables collision.
pub fn set_camera_orbit_collision(
    world: &mut World,
    camera: EntityId,
    mask: u32,
    padding: f32,
) -> bool {
    edit(world, camera, |payload| {
        payload["collision_mask"] = Value::from(mask);
        payload["collision_padding"] = Value::from(padding);
    })
}
