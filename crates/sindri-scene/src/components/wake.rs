//! What a body pushes aside as it passes (`sindri.wake`).
//!
//! Swaying pictures (a sprite with `sway`, grass, a bush) within `radius` of
//! an entity with a wake lean away from it, most at its feet and less further
//! out, and stand back up as it goes on. Only how they look changes.

use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;

/// How a body parts the plants it walks through.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct WakeComponent {
    /// How far round it plants lean, in world units.
    pub radius: f32,
    /// How far a plant at its feet leans, as a share of the plant's width.
    pub strength: f32,
}

impl Default for WakeComponent {
    fn default() -> Self {
        Self {
            radius: 1.2,
            strength: 0.25,
        }
    }
}

impl SceneComponent for WakeComponent {
    const TYPE_NAME: &'static str = "sindri.wake";
}

impl WakeComponent {
    /// How far a plant whose foot is `away` from the body's leans along the
    /// world's x, as a share of its width: away from the body, more the
    /// nearer it is.
    #[must_use]
    pub fn lean(self, away: [f32; 2]) -> f32 {
        if !(self.radius > 0.0 && self.strength.is_finite()) {
            return 0.0;
        }
        let distance = away[0].hypot(away[1]);
        if distance >= self.radius {
            return 0.0;
        }
        let near = 1.0 - distance / self.radius;
        let side = if away[0].abs() < 1e-3 {
            0.0
        } else {
            away[0].signum()
        };
        self.strength * near * near * side
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plant_leans_away_more_the_nearer_it_is() {
        let wake = WakeComponent::default();
        assert!(wake.lean([0.3, 0.0]) > wake.lean([0.8, 0.0]));
        assert!(wake.lean([0.3, 0.0]) > 0.0, "to the east of it, east");
        assert!(wake.lean([-0.3, 0.1]) < 0.0, "to the west, west");
        assert!(
            wake.lean([2.0, 0.0]).abs() < f32::EPSILON,
            "out of reach, upright"
        );
    }
}
