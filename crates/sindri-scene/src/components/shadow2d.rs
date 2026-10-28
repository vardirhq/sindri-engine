//! What an upright thing in a 2D scene casts onto the ground.
//!
//! `sindri.light2d.occluder` decides what a light cannot see through. This is the
//! other half of a shadow, and the half a viewer actually looks at: the dark patch
//! on the ground that says a tree is standing there and which way the light is.
//! Two parts, either of which an entity may have:
//!
//! - a **contact shadow**, a soft dark ellipse under the thing, which says it is
//!   resting on the ground whatever lights are about; and
//! - a **cast shadow**, the thing's own picture laid on the ground and pointed
//!   away from each light near it, longer for a taller thing and for a light
//!   further from it, fainter where less light reaches its feet.
//!
//! Neither blocks light and neither is lit: they are painted on top of the lit
//! ground, so they darken what light reaches and change nothing in the dark.

use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;

/// A thing's shadow on the ground.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Shadow2dComponent {
    /// How wide and how deep the contact shadow is, in world units. Zero in
    /// either leaves it out.
    pub contact_size: [f32; 2],
    /// Where its middle is, from the entity's position. The entity is where the
    /// thing stands, so a shadow under a character's feet is usually at zero.
    pub contact_offset: [f32; 2],
    /// How dark it is at its middle, 0 to 1.
    pub contact_opacity: f32,
    /// Whether the thing's picture is also cast away from the lights.
    pub cast: bool,
    /// How dark a cast shadow is where a light reaches its full strength.
    pub cast_opacity: f32,
    /// How tall the thing is, in world units, which sets how long it casts.
    /// Zero takes the height its picture is drawn at.
    pub height: f32,
    /// The longest a cast shadow gets, however low or far the light is.
    pub max_length: f32,
}

impl SceneComponent for Shadow2dComponent {
    const TYPE_NAME: &'static str = "sindri.shadow2d";
}

impl Default for Shadow2dComponent {
    fn default() -> Self {
        Self {
            contact_size: [1.0, 0.4],
            contact_offset: [0.0, 0.0],
            contact_opacity: 0.35,
            cast: false,
            cast_opacity: 0.45,
            height: 0.0,
            max_length: 2.5,
        }
    }
}

/// Why a shadow was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("`{field}` {expected}")]
pub struct Shadow2dError {
    pub field: &'static str,
    pub expected: &'static str,
}

impl Shadow2dComponent {
    /// Refuses values the renderer cannot use, naming the field.
    ///
    /// # Errors
    /// A size, opacity, height or length that is not a finite number in range.
    pub fn validate(self) -> Result<Self, Shadow2dError> {
        let numbers = [
            ("contact_size", self.contact_size[0], 0.0, f32::MAX),
            ("contact_size", self.contact_size[1], 0.0, f32::MAX),
            ("contact_opacity", self.contact_opacity, 0.0, 1.0),
            ("cast_opacity", self.cast_opacity, 0.0, 1.0),
            ("height", self.height, 0.0, f32::MAX),
            ("max_length", self.max_length, 0.0, f32::MAX),
        ];
        for (field, value, low, high) in numbers {
            if !value.is_finite() || value < low || value > high {
                return Err(Shadow2dError {
                    field,
                    expected: "must be a finite number in range, at least 0 and for an opacity at most 1",
                });
            }
        }
        if !self.contact_offset.iter().all(|value| value.is_finite()) {
            return Err(Shadow2dError {
                field: "contact_offset",
                expected: "must be finite numbers",
            });
        }
        Ok(self)
    }

    /// Whether there is a contact shadow to draw.
    #[must_use]
    pub fn has_contact(self) -> bool {
        self.contact_size[0] > 0.0 && self.contact_size[1] > 0.0 && self.contact_opacity > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::Shadow2dComponent;

    #[test]
    fn a_default_shadow_has_a_contact_ellipse_and_casts_nothing() {
        let shadow = Shadow2dComponent::default()
            .validate()
            .expect("the default is valid");
        assert!(shadow.has_contact());
        assert!(!shadow.cast);
    }

    #[test]
    fn a_zero_size_or_opacity_leaves_the_contact_out() {
        let none = Shadow2dComponent {
            contact_size: [1.0, 0.0],
            ..Shadow2dComponent::default()
        };
        assert!(!none.has_contact());
        let clear = Shadow2dComponent {
            contact_opacity: 0.0,
            ..Shadow2dComponent::default()
        };
        assert!(!clear.has_contact());
    }

    #[test]
    fn a_value_out_of_range_names_its_field() {
        let error = Shadow2dComponent {
            cast_opacity: 1.5,
            ..Shadow2dComponent::default()
        }
        .validate()
        .expect_err("an opacity over 1 is refused");
        assert_eq!(error.field, "cast_opacity");
        assert!(
            Shadow2dComponent {
                height: f32::NAN,
                ..Shadow2dComponent::default()
            }
            .validate()
            .is_err()
        );
    }
}
