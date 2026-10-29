//! What a viewer in a 2D scene can see.
//!
//! A light decides what is lit; this decides what is seen. `sindri.sight2d` on an
//! entity makes it the viewer: everything the entity cannot see in a straight line,
//! past the edges of `sindri.light2d.occluder`s, is dimmed. The picture is not hidden,
//! the light is not changed, and a lantern across the field still shows where it
//! is: it is the viewer's own eyes that stop at a tree. Only the first enabled
//! viewer in the scene counts. Nothing is dimmed unless the environment turns 2D
//! lighting on.

use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;

use super::light2d::Light2dError;

/// A viewer: what it sees and how dim the rest goes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Sight2dComponent {
    pub enabled: bool,
    /// How far it sees, in world units, fading over the last quarter. Zero sees
    /// as far as there is.
    pub radius: f32,
    /// What lit content out of sight is multiplied by, 0 to 1: 1 changes nothing
    /// and 0 is black.
    pub unseen: f32,
    /// How far past an occluding edge it still sees, so the face of a wall it
    /// looks at is seen.
    pub reach: f32,
}

impl SceneComponent for Sight2dComponent {
    const TYPE_NAME: &'static str = "sindri.sight2d";
}

impl Default for Sight2dComponent {
    fn default() -> Self {
        Self {
            enabled: true,
            radius: 0.0,
            unseen: 0.3,
            reach: 0.3,
        }
    }
}

impl Sight2dComponent {
    /// Refuses values the renderer cannot use, naming the field.
    ///
    /// # Errors
    /// A radius or reach that is not a finite number of at least 0, or an
    /// `unseen` outside 0 to 1.
    pub fn validate(self) -> Result<Self, Light2dError> {
        for (name, value) in [("radius", self.radius), ("reach", self.reach)] {
            if !value.is_finite() || value < 0.0 {
                return Err(Light2dError::Field {
                    field: name,
                    expected: "must be a finite number of at least 0",
                });
            }
        }
        if !self.unseen.is_finite() || !(0.0..=1.0).contains(&self.unseen) {
            return Err(Light2dError::Field {
                field: "unseen",
                expected: "must be a number from 0 to 1",
            });
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::Sight2dComponent;

    #[test]
    fn a_default_viewer_is_valid_and_dims_but_does_not_black_out() {
        let sight = Sight2dComponent::default()
            .validate()
            .expect("the default is valid");
        assert!(sight.unseen > 0.0 && sight.unseen < 1.0);
    }

    #[test]
    fn a_value_out_of_range_names_its_field() {
        for (sight, field) in [
            (
                Sight2dComponent {
                    unseen: 1.5,
                    ..Sight2dComponent::default()
                },
                "unseen",
            ),
            (
                Sight2dComponent {
                    radius: -1.0,
                    ..Sight2dComponent::default()
                },
                "radius",
            ),
            (
                Sight2dComponent {
                    reach: f32::NAN,
                    ..Sight2dComponent::default()
                },
                "reach",
            ),
        ] {
            let error = sight.validate().expect_err("refused");
            assert!(error.to_string().contains(field), "{field}: {error}");
        }
    }
}
