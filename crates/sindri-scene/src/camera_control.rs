//! Runtime control of the authored world camera.
//!
//! Camera math remains in the scene camera system. These functions only mutate
//! the same authored component state that system consumes on the next frame.

use serde_json::Value;
use sindri_core::{EntityId, SceneComponent, World};

use crate::{CameraBehaviorComponent, CameraBounds, CameraComponent, CameraFollow};

fn camera_behavior_payload(world: &mut World) -> Option<&mut Value> {
    let cameras: Vec<_> = world
        .entities()
        .filter_map(|(entity, data)| {
            (data.components.contains_key(CameraComponent::TYPE_NAME)
                && data
                    .components
                    .contains_key(CameraBehaviorComponent::TYPE_NAME))
            .then_some(entity)
        })
        .collect();
    let [camera] = cameras.as_slice() else {
        return None;
    };
    world
        .get_mut(*camera)?
        .components
        .get_mut(CameraBehaviorComponent::TYPE_NAME)
}

fn edit_behavior(
    world: &mut World,
    edit: impl FnOnce(&mut CameraBehaviorComponent) -> bool,
) -> bool {
    let Some(payload) = camera_behavior_payload(world) else {
        return false;
    };
    let Ok(mut behavior) = serde_json::from_value::<CameraBehaviorComponent>(payload.clone())
    else {
        return false;
    };
    if !edit(&mut behavior) {
        return false;
    }
    let Ok(value) = serde_json::to_value(behavior) else {
        return false;
    };
    *payload = value;
    true
}

/// Makes the authored world camera follow `target` while preserving its other
/// follow settings. The target must be an authored entity with a scene ID.
pub fn set_camera_follow_target(world: &mut World, target: EntityId) -> bool {
    let Some(target) = world.get(target).and_then(|data| data.source_id.clone()) else {
        return false;
    };
    edit_behavior(world, |behavior| {
        if let Some(follow) = &mut behavior.follow {
            follow.target = target;
        } else {
            behavior.follow = Some(CameraFollow {
                target,
                offset: [0.0; 3],
                dead_zone: [0.0; 2],
                smoothing: 8.0,
                max_speed: 0.0,
            });
        }
        true
    })
}

/// Stops engine-owned follow without changing the camera transform.
pub fn clear_camera_follow(world: &mut World) -> bool {
    edit_behavior(world, |behavior| {
        behavior.follow = None;
        true
    })
}

/// Changes the engine-owned follow offset.
pub fn set_camera_follow_offset(world: &mut World, offset: [f32; 3]) -> bool {
    edit_behavior(world, |behavior| {
        let Some(follow) = &mut behavior.follow else {
            return false;
        };
        follow.offset = offset;
        true
    })
}

/// Changes the full width and height of the rectangular follow dead zone.
pub fn set_camera_dead_zone(world: &mut World, dead_zone: [f32; 2]) -> bool {
    if dead_zone
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return false;
    }
    edit_behavior(world, |behavior| {
        let Some(follow) = &mut behavior.follow else {
            return false;
        };
        follow.dead_zone = dead_zone;
        true
    })
}

/// Changes follow smoothing. Zero means no smoothing movement.
pub fn set_camera_smoothing(world: &mut World, smoothing: f32) -> bool {
    if !smoothing.is_finite() || smoothing < 0.0 {
        return false;
    }
    edit_behavior(world, |behavior| {
        let Some(follow) = &mut behavior.follow else {
            return false;
        };
        follow.smoothing = smoothing;
        true
    })
}

/// Changes the follow speed cap. Zero means uncapped.
pub fn set_camera_max_speed(world: &mut World, max_speed: f32) -> bool {
    if !max_speed.is_finite() || max_speed < 0.0 {
        return false;
    }
    edit_behavior(world, |behavior| {
        let Some(follow) = &mut behavior.follow else {
            return false;
        };
        follow.max_speed = max_speed;
        true
    })
}

/// Replaces the authored 2D confinement rectangle.
pub fn set_camera_bounds(world: &mut World, min: [f32; 2], max: [f32; 2]) -> bool {
    if min.into_iter().chain(max).any(|value| !value.is_finite())
        || min[0] > max[0]
        || min[1] > max[1]
    {
        return false;
    }
    edit_behavior(world, |behavior| {
        behavior.confine = Some(CameraBounds { min, max });
        true
    })
}

/// Removes camera confinement.
pub fn clear_camera_bounds(world: &mut World) -> bool {
    edit_behavior(world, |behavior| {
        behavior.confine = None;
        true
    })
}

/// Raises the authored camera's trauma to at least `amount`, up to 1.
///
/// An impact rather than an addition: a small hit while a bigger one is still
/// shaking the camera changes nothing, so a burst of small hits in one frame
/// does not add up to the biggest shake there is.
pub fn raise_camera_trauma(world: &mut World, amount: f32) -> bool {
    if !amount.is_finite() || amount < 0.0 {
        return false;
    }
    edit_behavior(world, |behavior| {
        behavior.shake.trauma = behavior.shake.trauma.max(amount.min(1.0));
        true
    })
}

/// Changes shake strength, decay and frequency without changing current trauma.
pub fn set_camera_shake(world: &mut World, strength: f32, decay: f32, frequency: f32) -> bool {
    if [strength, decay, frequency]
        .into_iter()
        .any(|value| !value.is_finite() || value < 0.0)
    {
        return false;
    }
    edit_behavior(world, |behavior| {
        behavior.shake.strength = strength;
        behavior.shake.decay = decay;
        behavior.shake.frequency = frequency;
        true
    })
}
