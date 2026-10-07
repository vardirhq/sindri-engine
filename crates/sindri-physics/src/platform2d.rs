//! Runtime support snapshots and collision-limited moving-platform carry.

use sindri_core::EntityId;

use crate::{PhysicsPose2d, SlideMotion2d};

/// Previous synchronized pose of the body that supported the character.
/// Hosts advance this snapshot once per movement pass, even if carry is blocked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlatformSupport2d {
    pub entity: EntityId,
    pub previous_pose: PhysicsPose2d,
}

/// Carry phase before the character's own grounded movement.
#[derive(Clone, Debug, PartialEq)]
pub struct PlatformCarry2d {
    pub entity: EntityId,
    /// Current synchronized body pose; use for the next snapshot only if
    /// the final ground hit still identifies this support entity.
    pub current_pose: PhysicsPose2d,
    /// Point displacement requested by the support's translation and rotation.
    /// Rotation is swept along the endpoint chord, not the full circular arc.
    pub requested_translation: [f32; 2],
    /// Actual carry and collisions against everything except the support body.
    /// The probe retains its original rotation. No platform velocity is added.
    pub motion: SlideMotion2d,
}
