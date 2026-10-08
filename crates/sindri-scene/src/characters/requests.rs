//! Runtime requests are neither component payloads nor serialized game state.
use sindri_core::EntityId;
use sindri_physics::PhysicsError;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(super) struct MotionRequest {
    pub displacement: [f32; 2],
    pub snap: bool,
}

/// One pending movement and drop-duration update per character, consumed by a fixed step.
#[derive(Clone, Default)]
pub struct CharacterRequests2d {
    pub(super) motion: BTreeMap<EntityId, MotionRequest>,
    pub(super) drop: BTreeMap<EntityId, f32>,
}

impl CharacterRequests2d {
    /// Queues world-space displacement; the last request before a step wins.
    /// `snap` permits authored snapping for this request; upward motion still disables it.
    /// Missing/inactive/non-controller requests are discarded at synchronization.
    ///
    /// # Errors
    /// Rejects nonfinite displacement or overflowing length before replacing a request.
    pub fn move_character(
        &mut self,
        entity: EntityId,
        displacement: [f32; 2],
        snap: bool,
    ) -> Result<(), PhysicsError> {
        if displacement.iter().any(|value| !value.is_finite())
            || !displacement[0].hypot(displacement[1]).is_finite()
        {
            return Err(PhysicsError::NonFinite("character_displacement"));
        }
        self.motion
            .insert(entity, MotionRequest { displacement, snap });
        Ok(())
    }

    /// Replaces the remaining simulation duration; zero cancels. A positive
    /// remainder covers a whole movement pass and expires after that fixed step.
    /// Newly spawned controllers start the timer when synchronized; pause retains it.
    ///
    /// # Errors
    /// Rejects nonfinite/negative duration before replacing a request.
    pub fn drop_through(&mut self, entity: EntityId, seconds: f32) -> Result<(), PhysicsError> {
        if !seconds.is_finite() {
            return Err(PhysicsError::NonFinite("character_drop_seconds"));
        }
        if seconds < 0.0 {
            return Err(PhysicsError::Negative("character_drop_seconds"));
        }
        self.drop.insert(entity, seconds);
        Ok(())
    }
}
