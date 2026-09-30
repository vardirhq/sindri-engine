//! Backend-independent 2D raycast data.

use sindri_core::EntityId;

/// Which collider pieces a ray may hit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RaycastFilter2d {
    /// Bit mask matched against each collider's layer memberships.
    pub mask: u32,
    pub include_sensors: bool,
    /// Excludes every collider piece belonging to this entity.
    pub exclude: Option<EntityId>,
}

impl Default for RaycastFilter2d {
    fn default() -> Self {
        Self { mask: u32::MAX, include_sensors: false, exclude: None }
    }
}

/// A snapshot of the closest hit, in world coordinates and world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit2d {
    pub entity: EntityId,
    pub point: [f32; 2],
    /// Surface normal; zero for a zero-distance hit inside/on a collider.
    pub normal: [f32; 2],
    pub distance: f32,
}
