//! Classified support and opt-in solved platform carry around Rapier movement.
use sindri_core::EntityId;

use crate::{CharacterMotion3d, CharacterOptions3d, GroundProbe3d, PhysicsPose3d};

/// Previous synchronized support. Hosts must invalidate on rebuild/teleport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlatformSupport3d {
    pub entity: EntityId,
    pub previous_pose: PhysicsPose3d,
}

/// Movement geometry policy; gravity and jump velocity remain gameplay-owned.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GroundedCharacterOptions3d {
    pub movement: CharacterOptions3d,
    /// Opt-in. Do not also add support motion through parenting or input.
    pub platform_support: Option<PlatformSupport3d>,
}

/// Verified support motion, swept before the character's requested movement.
#[derive(Clone, Debug, PartialEq)]
pub struct PlatformCarry3d {
    pub entity: EntityId,
    pub current_pose: PhysicsPose3d,
    /// Motion of the probe origin through the previous/current support poses.
    pub requested: [f32; 3],
    /// Collision-limited carry, separate from the character's movement hits.
    pub motion: CharacterMotion3d,
}

/// Read-only proposal combining carry, movement and classified final support.
#[derive(Clone, Debug, PartialEq)]
pub struct GroundedCharacterMotion3d {
    /// Total displacement, including actual carry exactly once.
    pub translation: [f32; 3],
    /// Rapier movement from the carried pose; excludes carry translation/hits.
    pub movement: CharacterMotion3d,
    /// Zero-travel classified support at the proposed endpoint.
    pub ground: GroundProbe3d,
    pub platform: Option<PlatformCarry3d>,
    /// Walkable final support without penetration or an upward input request.
    /// Independent of the raw Rapier `movement.grounded` flag.
    pub grounded: bool,
}
