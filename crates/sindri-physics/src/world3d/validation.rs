//! Validate the complete request before any backend mutation.

use crate::validate::{finite, non_negative, normalized, positive};
use crate::{Collider3d, ColliderShape3d, PhysicsError, PhysicsPose3d, RigidBody3d};

pub(super) fn finite3(name: &'static str, value: [f32; 3]) -> Result<(), PhysicsError> {
    if value.into_iter().all(f32::is_finite) {
        Ok(())
    } else {
        Err(PhysicsError::NonFinite(name))
    }
}

fn rotation(name: &'static str, value: [f32; 4]) -> Result<(), PhysicsError> {
    if !value.into_iter().all(f32::is_finite) {
        return Err(PhysicsError::NonFinite(name));
    }
    let norm = value
        .into_iter()
        .map(|part| f64::from(part).powi(2))
        .sum::<f64>();
    if (norm - 1.0).abs() > 0.0001 {
        return Err(PhysicsError::InvalidQuaternion(name));
    }
    Ok(())
}

pub(super) fn validate_pose(value: PhysicsPose3d) -> Result<(), PhysicsError> {
    finite3("position", value.position)?;
    rotation("rotation", value.rotation)
}

pub(super) fn validate_body(value: RigidBody3d) -> Result<(), PhysicsError> {
    validate_pose(PhysicsPose3d {
        position: value.position,
        rotation: value.rotation,
    })?;
    finite3("linear_velocity", value.linear_velocity)?;
    finite3("angular_velocity", value.angular_velocity)?;
    finite("gravity_scale", value.gravity_scale)?;
    non_negative("linear_damping", value.linear_damping)?;
    non_negative("angular_damping", value.angular_damping)
}

pub(super) fn validate_colliders(values: &[Collider3d]) -> Result<(), PhysicsError> {
    for (index, &value) in values.iter().enumerate() {
        validate_collider(value).map_err(|reason| PhysicsError::ColliderPiece {
            index,
            reason: Box::new(reason),
        })?;
    }
    Ok(())
}

fn validate_collider(value: Collider3d) -> Result<(), PhysicsError> {
    finite3("collider_offset", value.offset)?;
    rotation("collider_rotation", value.rotation)?;
    non_negative("friction", value.friction)?;
    normalized("restitution", value.restitution)?;
    match value.shape {
        ColliderShape3d::Box { half_extents } => {
            for (name, extent) in [
                "box_half_extent_x",
                "box_half_extent_y",
                "box_half_extent_z",
            ]
            .into_iter()
            .zip(half_extents)
            {
                positive(name, extent)?;
            }
        }
        ColliderShape3d::Sphere { radius } => positive("sphere_radius", radius)?,
        ColliderShape3d::Capsule {
            half_height,
            radius,
        } => {
            positive("capsule_half_height", half_height)?;
            positive("capsule_radius", radius)?;
        }
    }
    Ok(())
}
