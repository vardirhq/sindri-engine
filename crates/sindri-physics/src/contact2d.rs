//! Copied contacts, independent of the collision backend.

use sindri_core::EntityId;

/// One solid solver contact from the last fixed step, relative to a queried body.
/// Multiple points and collider pieces can name the same other entity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact2d {
    /// The other entity touching the queried body.
    pub entity: EntityId,
    /// Midpoint of the two surface anchors in world coordinates.
    pub point: [f32; 2],
    /// World unit normal pointing towards the queried body (its push direction).
    pub normal: [f32; 2],
    /// Nonnegative normal impulse from the last solve.
    pub normal_impulse: f32,
    /// Signed friction impulse along `[-normal.y, normal.x]`.
    pub tangent_impulse: f32,
    /// Total world impulse divided by the last fixed step's seconds.
    /// Sleeping contacts remain present but report zero impulses and force.
    pub force: [f32; 2],
}
