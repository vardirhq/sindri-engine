//! The typed physics operation catalogue.

/// What a script can do to a body, connect bodies with, and ask about what it touched.
///
/// Legacy velocity controls read axes separately and write both together.
/// New controls use typed values and preserve this existing surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PhysicsCall {
    /// The closest 2D collider hit along a world-space segment.
    Raycast,
    ContinuousCollision,
    SetContinuousCollision,
    VelocityX,
    VelocityY,
    SetVelocity,
    ApplyImpulse,
    /// Keeps two bodies no farther apart than a world-space distance while
    /// leaving their rotation and closer motion unconstrained.
    ConnectDistance,
    /// Every entity with a collider inside a circle or a box placed in the
    /// world: an area check, for a blast, an aura or a pickup radius.
    OverlapCircle,
    OverlapBox,
    /// The first collider a circle or box would touch moved along a line: a
    /// raycast with a size, for whether a body fits through a gap.
    CastCircle,
    CastBox,
    /// The mask bit a collision layer's name stands for, and the mask of
    /// several, from the names the scene's physics world gives its layers.
    Layer,
    Mask,
    /// The entities this one started touching during the last step.
    ///
    /// A query rather than a callback, because Decay now has a value that can
    /// hold several entities and a lifecycle function would be a second way for
    /// the host to enter a script. Answered for the entity the script is on:
    /// an event is about a pair, and the pair a script cares about is the one it
    /// is half of.
    CollisionStarted,
    CollisionStopped,
    /// The same, for colliders authored as sensors, which register a touch and
    /// do not push back.
    SensorEntered,
    SensorExited,
}

pub(crate) const PHYSICS_CALLS: &[(&str, PhysicsCall)] = &[
    ("raycast", PhysicsCall::Raycast),
    ("continuous_collision", PhysicsCall::ContinuousCollision),
    (
        "set_continuous_collision",
        PhysicsCall::SetContinuousCollision,
    ),
    ("velocity_x", PhysicsCall::VelocityX),
    ("velocity_y", PhysicsCall::VelocityY),
    ("set_velocity", PhysicsCall::SetVelocity),
    ("apply_impulse", PhysicsCall::ApplyImpulse),
    ("connect_distance", PhysicsCall::ConnectDistance),
    ("overlap_circle", PhysicsCall::OverlapCircle),
    ("overlap_box", PhysicsCall::OverlapBox),
    ("cast_circle", PhysicsCall::CastCircle),
    ("cast_box", PhysicsCall::CastBox),
    ("layer", PhysicsCall::Layer),
    ("mask", PhysicsCall::Mask),
    ("collision_started", PhysicsCall::CollisionStarted),
    ("collision_stopped", PhysicsCall::CollisionStopped),
    ("sensor_entered", PhysicsCall::SensorEntered),
    ("sensor_exited", PhysicsCall::SensorExited),
];

impl PhysicsCall {
    /// Whether this asks about what happened rather than acting on a body.
    pub(crate) const fn is_event(self) -> bool {
        matches!(
            self,
            Self::CollisionStarted
                | Self::CollisionStopped
                | Self::SensorEntered
                | Self::SensorExited
        )
    }
}
