//! Backend-independent configuration and results for swept sliding.

use crate::validate::positive;
use crate::{PhysicsError, ShapeHit2d};

/// Geometric sweep/slide settings, independent of gameplay speed or gravity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlideOptions2d {
    /// Separation maintained from solid surfaces, in world units; must be positive.
    pub skin: f32,
    /// Maximum sweep/slide iterations per call, from 1 through 32.
    pub max_iterations: u8,
}

impl Default for SlideOptions2d {
    fn default() -> Self {
        Self {
            skin: 0.01,
            max_iterations: 8,
        }
    }
}

impl SlideOptions2d {
    /// # Errors
    /// Rejects non-positive/non-finite skin and an iteration budget outside 1..=32.
    pub fn validate(self) -> Result<(), PhysicsError> {
        positive("slide_skin", self.skin)?;
        if !(1..=32).contains(&self.max_iterations) {
            return Err(PhysicsError::InvalidSlideIterations);
        }
        Ok(())
    }
}

/// Proposed movement against the current world; computing it changes no body.
#[derive(Clone, Debug, PartialEq)]
pub struct SlideMotion2d {
    /// Displacement the caller may apply to the probe's starting pose.
    pub translation: [f32; 2],
    /// Remaining sliding displacement discarded when initially penetrating or
    /// when the iteration budget is exhausted. Blocked normal motion is removed.
    pub remaining: [f32; 2],
    /// Hits in sweep order; distances are relative to each sweep's starting pose.
    pub collisions: Vec<ShapeHit2d>,
    /// Initial penetration blocks the whole move rather than guessing a recovery.
    pub started_penetrating: bool,
    /// Unapplied sliding movement remains after the configured iteration budget.
    pub iteration_limit_reached: bool,
}
