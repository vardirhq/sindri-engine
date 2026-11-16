//! A world that stays near its origin however far play goes
//! (`sindri.floating_origin`).
//!
//! Positions are 32-bit floats, which are fine near the origin and coarse far
//! from it: a hundred thousand units out a position is good to about a hundredth
//! of a unit, and the physics of a walk, whose steps are a twelfth of a unit, has
//! started to stutter long before that. A land that goes on (a voxel world drawn
//! as a map, scattered and walled near play) is only as wide as its precision,
//! so this keeps play near the origin instead: once the entity it follows is
//! further than `distance` from the origin, the runtime moves the whole world
//! back by whole multiples of `step`, and it is near the origin again.
//!
//! Every top-level entity of the world moves, the camera with it, and so does
//! every particle in flight, so nothing on screen changes. A land's squares are
//! counted from the land's own placing, which moved too, so the same square is
//! the same square and the generator, the scatter, the walls and what was taken
//! all agree. Screen elements stay where they are. What has moved in all is
//! kept in `moved`, so a script can say where play really is.

use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;

/// Keeps play near the origin.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct FloatingOriginComponent {
    /// The entity play follows, by its scene ID: the one the player walks.
    pub around: String,
    /// How far from the origin, on either axis, it may go before the world
    /// is moved back.
    pub distance: f32,
    /// What the world is moved by is a whole multiple of this, so a pixel
    /// grid or a sixteen-square chunk lines up as before.
    pub step: f32,
    /// How far the world has been moved in all, X and Y: where the origin
    /// now is in the world as it was authored is minus this.
    pub moved: [f64; 2],
}

impl Default for FloatingOriginComponent {
    fn default() -> Self {
        Self {
            around: String::new(),
            distance: 1024.0,
            step: 256.0,
            moved: [0.0; 2],
        }
    }
}

impl SceneComponent for FloatingOriginComponent {
    const TYPE_NAME: &'static str = "sindri.floating_origin";
}

impl FloatingOriginComponent {
    /// How far to move the world when play is at `at`: nothing while it is
    /// within `distance` of the origin, and otherwise back by the whole steps
    /// nearest to where it is.
    #[must_use]
    pub fn shift_for(&self, at: [f32; 2]) -> Option<[f32; 2]> {
        if !(self.distance > 0.0 && self.step > 0.0) {
            return None;
        }
        if at[0].abs() <= self.distance && at[1].abs() <= self.distance {
            return None;
        }
        let back = |value: f32| -(value / self.step).round() * self.step;
        Some([back(at[0]), back(at[1])])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_world_moves_back_in_whole_steps_once_play_is_far() {
        let origin = FloatingOriginComponent::default();
        assert_eq!(origin.shift_for([1000.0, -1000.0]), None, "near enough");
        assert_eq!(origin.shift_for([1100.0, 20.0]), Some([-1024.0, -0.0]));
        assert_eq!(origin.shift_for([-3000.0, 1500.0]), Some([3072.0, -1536.0]));
        let off = FloatingOriginComponent {
            step: 0.0,
            ..FloatingOriginComponent::default()
        };
        assert_eq!(
            off.shift_for([1.0e6, 0.0]),
            None,
            "a step of nothing moves nothing"
        );
    }
}
