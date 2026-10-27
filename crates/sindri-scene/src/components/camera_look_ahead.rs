//! Smooth portrait camera look-ahead based on target movement.

use sindri_core::{EntityId, World};

use super::CameraLookAhead;

pub(super) fn advance(
    world: &World,
    camera: EntityId,
    look: &mut CameraLookAhead,
    target_x: f32,
    viewport: Option<(f32, f32)>,
    dt: f32,
) -> f32 {
    const TURN_DISTANCE: f32 = 0.25;
    if let Some(previous) = look.previous_x {
        let delta = target_x - previous;
        if delta.is_finite() {
            look.travel = (look.travel + delta).clamp(-TURN_DISTANCE, TURN_DISTANCE);
            if look.travel >= TURN_DISTANCE {
                look.direction = 1.0;
            } else if look.travel <= -TURN_DISTANCE {
                look.direction = -1.0;
            }
        }
    }
    look.previous_x = Some(target_x);
    let width = viewport
        .filter(|(width, height)| width < height)
        .and_then(|size| super::camera_confine::view_half_size(world, camera, Some(size)))
        .map_or(0.0, |half| half[0]);
    let fraction = if look.fraction.is_finite() {
        look.fraction.clamp(0.0, 0.8)
    } else {
        0.0
    };
    let desired = look.direction * width * fraction;
    look.offset += (desired - look.offset) * (1.0 - (-3.0 * dt).exp());
    look.offset
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sindri_core::{EntityData, SceneComponent};

    #[test]
    fn portrait_look_ahead_holds_and_eases_when_direction_changes() {
        let mut world = World::default();
        let mut data = EntityData::default();
        data.components.insert(
            super::super::CameraComponent::TYPE_NAME.into(),
            json!({"projection": "orthographic", "vertical_size": 11.25,
                "near": 0.1, "far": 100.0}),
        );
        let camera = world.spawn(data);
        let mut look: CameraLookAhead = serde_json::from_value(json!({"fraction": 0.65})).unwrap();
        let portrait = Some((390.0, 844.0));
        advance(&world, camera, &mut look, 10.0, portrait, 0.1);
        for _ in 0..30 {
            advance(&world, camera, &mut look, 11.0, portrait, 0.1);
        }
        assert!(look.offset > 1.5);
        let before = look.offset;
        advance(&world, camera, &mut look, 10.9, portrait, 0.1);
        assert!(look.offset >= before - 0.01, "short reversals keep direction");
        for _ in 0..30 {
            advance(&world, camera, &mut look, 10.0, portrait, 0.1);
        }
        assert!(look.offset < -1.5, "sustained reverse travel looks left");
        for _ in 0..30 {
            advance(&world, camera, &mut look, 10.0, Some((960.0, 540.0)), 0.1);
        }
        assert!(look.offset.abs() < 0.01, "landscape recenters");
    }
}
