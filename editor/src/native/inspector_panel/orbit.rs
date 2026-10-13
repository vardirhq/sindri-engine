//! An authored spatial target for a newly added orbit camera.
use sindri_core::{EntityId, World};

use crate::space::{EntitySpace, declared_space};

pub(super) fn default_target(world: &World, camera: EntityId) -> Option<String> {
    world.get(camera)?.transform_3d?;
    world.entities().find_map(|(entity, data)| {
        (entity != camera
            && world.is_active(entity)
            && data.transform_3d.is_some()
            && declared_space(&data.components) != Some(EntitySpace::Ui)
            && world.check_set_parent(camera, Some(entity)).is_ok())
        .then(|| data.source_id.as_ref().map(|id| id.as_str().to_owned()))
        .flatten()
    })
}

pub(super) fn validate(
    world: &World,
    camera: EntityId,
    payload: &serde_json::Value,
) -> Result<(), &'static str> {
    let orbit: sindri_scene::CameraOrbitComponent = serde_json::from_value(payload.clone())
        .map_err(|_| "Camera Orbit fields must have valid types and a target ID")?;
    if !orbit.is_valid() {
        return Err(
            "Camera Orbit needs finite settings, positive distance, pitch between the poles, and padding below distance",
        );
    }
    let data = world.get(camera).ok_or("Orbit camera is missing")?;
    if data.transform_3d.is_none()
        || data.components.get("sindri.camera").is_none_or(|payload| {
            serde_json::from_value::<sindri_scene::CameraComponent>(payload.clone()).is_err()
        })
    {
        return Err("Camera Orbit needs a spatial Camera on this entity");
    }
    let target = world
        .entity_for_source_id(&orbit.target)
        .ok_or("Orbit target does not exist in this scene")?;
    if target == camera
        || !world.is_active(target)
        || world.world_transform(target).is_none()
        || world.check_set_parent(camera, Some(target)).is_err()
    {
        return Err(
            "Orbit target must be another active spatial entity outside the camera's children",
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "orbit_tests.rs"]
mod tests;
