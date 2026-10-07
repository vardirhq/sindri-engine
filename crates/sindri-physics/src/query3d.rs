//! Backend-independent 3D query values.

use sindri_core::EntityId;

/// Piece filtering shared by 3D rays, overlaps and shape casts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RaycastFilter3d {
    /// Matches collider memberships independently of physical interaction filters.
    pub mask: u32,
    pub include_sensors: bool,
    /// Excludes every piece of this entity.
    pub exclude: Option<EntityId>,
}

impl Default for RaycastFilter3d {
    fn default() -> Self {
        Self {
            mask: u32::MAX,
            include_sensors: false,
            exclude: None,
        }
    }
}

/// Copied closest ray hit in world coordinates and world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit3d {
    pub entity: EntityId,
    pub point: [f32; 3],
    /// Outward surface normal; zero when starting inside/on the piece.
    pub normal: [f32; 3],
    pub distance: f32,
}

/// Copied first contact along a fixed-orientation shape sweep.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeHit3d {
    pub entity: EntityId,
    /// World contact point; probe origin for an initial overlap.
    pub point: [f32; 3],
    /// Outward surface normal; zero for an initial overlap.
    pub normal: [f32; 3],
    pub distance: f32,
}
