//! Pending displacement is runtime state, never an authored component field.
use sindri_core::EntityId;
use sindri_physics::PhysicsError;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(super) struct MotionRequest {
    pub displacement: [f32; 3],
    pub snap: bool,
}

/// One pending world-space movement per character, consumed by a fixed step.
#[derive(Clone, Default)]
pub struct CharacterRequests3d {
    pub(super) motion: BTreeMap<EntityId, MotionRequest>,
}

impl CharacterRequests3d {
    /// Queues displacement for the next fixed step; the last valid request wins.
    /// `snap` permits authored snapping; upward motion still suppresses it.
    /// Missing, inactive and non-controller requests are discarded at synchronization.
    ///
    /// # Errors
    /// Rejects nonfinite displacement or overflowing length before replacing input.
    pub fn move_character(
        &mut self,
        entity: EntityId,
        displacement: [f32; 3],
        snap: bool,
    ) -> Result<(), PhysicsError> {
        let length = displacement[0]
            .hypot(displacement[1])
            .hypot(displacement[2]);
        // The backend also uses a squared f32 length; reject overflow there now.
        let squared = displacement.iter().map(|axis| axis * axis).sum::<f32>();
        if !length.is_finite() || !squared.is_finite() {
            return Err(PhysicsError::NonFinite("character_displacement"));
        }
        self.motion
            .insert(entity, MotionRequest { displacement, snap });
        Ok(())
    }
}
