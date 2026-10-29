//! 2D lights and what blocks them.
//!
//! `sindri.light2d` is a point of light in the plane: it lights world sprites,
//! tiles, flecks and painted shapes by how far they are from it, and has no
//! direction. It is a component of its own rather than a kind of
//! `sindri.light`, because a 3D light shades surfaces by the way they face and
//! a 2D one lights pictures, which face nowhere but the viewer.
//!
//! `sindri.light2d.occluder` is what a light with shadows cannot see through:
//! a box, an authored polygon, or the solid cells of the tilemap beside it.
//! Nothing is lit unless the environment turns 2D lighting on.

use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;

/// The environment's share of 2D lighting.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EnvironmentLighting2d {
    /// Whether 2D lights light anything. Off, every picture is its own color.
    pub enabled: bool,
    /// What lit content is multiplied by where no light reaches.
    pub ambient: [f32; 3],
}

impl Default for EnvironmentLighting2d {
    fn default() -> Self {
        Self {
            enabled: false,
            ambient: [0.25, 0.25, 0.3],
        }
    }
}

/// A point light in the plane.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Light2dComponent {
    pub enabled: bool,
    /// The light's color. Alpha scales its intensity, so a script can fade a
    /// light the way it fades a sprite.
    pub color: [f32; 4],
    pub intensity: f32,
    /// How far the light reaches, in world units.
    pub radius: f32,
    /// How the light fades toward its radius: 1 is linear, 2 quadratic.
    pub falloff: f32,
    /// How far above the plane the light sits, read only by normal-mapped
    /// art: low lights rake across the relief, high ones light it squarely.
    pub height: f32,
    /// Whether occluders block this light.
    pub shadows: bool,
    /// How far past an occluding edge the light still reaches, so the face of
    /// a wall the light falls on is lit.
    pub shadow_reach: f32,
    /// Size of the emitting area in world units. Zero keeps point shadows.
    pub shadow_source_radius: f32,
    /// How wide the beam is, in degrees across. 360 lights every way, a point
    /// light; less is a spot light, which lights only a cone.
    pub cone_angle: f32,
    /// Where the beam points, in degrees: 0 is right and 90 is up.
    pub cone_direction: f32,
    /// How much of the cone's width fades out at its edge, 0 to 1: 0 is a hard
    /// edge and 1 a beam that is brightest only along its middle.
    pub cone_softness: f32,
}

impl SceneComponent for Light2dComponent {
    const TYPE_NAME: &'static str = "sindri.light2d";
}

impl Default for Light2dComponent {
    fn default() -> Self {
        Self {
            enabled: true,
            color: [1.0, 0.85, 0.6, 1.0],
            intensity: 1.0,
            radius: 4.0,
            falloff: 1.5,
            height: 1.0,
            shadows: false,
            shadow_reach: 0.1,
            shadow_source_radius: 0.0,
            cone_angle: 360.0,
            cone_direction: 0.0,
            cone_softness: 0.3,
        }
    }
}

/// Why a 2D light or occluder was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum Light2dError {
    #[error("`{field}` {expected}")]
    Field {
        field: &'static str,
        expected: &'static str,
    },
    #[error("an occluder polygon needs at least two points")]
    TooFewPoints,
    #[error("a tilemap occluder needs a tilemap on the same entity")]
    NoTilemap,
}

const fn field(field: &'static str, expected: &'static str) -> Light2dError {
    Light2dError::Field { field, expected }
}

impl Light2dComponent {
    /// Refuses values the renderer cannot use, naming the field.
    pub fn validate(self) -> Result<Self, Light2dError> {
        if !self.color.iter().all(|value| value.is_finite()) {
            return Err(field("color", "must be finite numbers"));
        }
        let at_least_zero = [
            ("intensity", self.intensity),
            ("height", self.height),
            ("shadow_reach", self.shadow_reach),
            ("shadow_source_radius", self.shadow_source_radius),
        ];
        for (name, value) in at_least_zero {
            if !value.is_finite() || value < 0.0 {
                return Err(field(name, "must be a finite number of at least 0"));
            }
        }
        if !self.cone_angle.is_finite() || self.cone_angle <= 0.0 || self.cone_angle > 360.0 {
            return Err(field(
                "cone_angle",
                "must be a finite number above 0 and at most 360",
            ));
        }
        if !self.cone_direction.is_finite() {
            return Err(field("cone_direction", "must be a finite number"));
        }
        if !self.cone_softness.is_finite() || !(0.0..=1.0).contains(&self.cone_softness) {
            return Err(field("cone_softness", "must be a number from 0 to 1"));
        }
        for (name, value) in [("radius", self.radius), ("falloff", self.falloff)] {
            if !value.is_finite() || value <= 0.0 {
                return Err(field(name, "must be a finite number above 0"));
            }
        }
        Ok(self)
    }

    /// The beam as (direction, angle, softness) in degrees and a share, or
    /// `None` for a light that shines every way.
    #[must_use]
    pub fn cone(self) -> Option<(f32, f32, f32)> {
        (self.cone_angle < 360.0).then_some((
            self.cone_direction,
            self.cone_angle,
            self.cone_softness,
        ))
    }

    /// The light's color in linear terms, scaled by intensity and alpha.
    #[must_use]
    pub fn radiance(self) -> [f32; 3] {
        let scale = self.intensity * self.color[3];
        [
            self.color[0] * scale,
            self.color[1] * scale,
            self.color[2] * scale,
        ]
    }
}

/// What an occluder's outline is.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OccluderShape {
    /// A rectangle `size` across, centred on the entity.
    #[default]
    Box,
    /// The authored `points`, in the entity's own space, closed.
    Polygon,
    /// The solid cells of the tilemap on the same entity: the boxes its
    /// tilemap collider would make, passable sprites included if it has one.
    Tilemap,
}

impl OccluderShape {
    /// Every spelling, in the order a picker offers them.
    pub const NAMES: [&'static str; 3] = ["box", "polygon", "tilemap"];
}

/// Blocks 2D lights that cast shadows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Light2dOccluderComponent {
    pub enabled: bool,
    pub shape: OccluderShape,
    pub size: [f32; 2],
    pub points: Vec<[f32; 2]>,
}

impl SceneComponent for Light2dOccluderComponent {
    const TYPE_NAME: &'static str = "sindri.light2d.occluder";
}

impl Default for Light2dOccluderComponent {
    fn default() -> Self {
        Self {
            enabled: true,
            shape: OccluderShape::Box,
            size: [1.0, 1.0],
            points: Vec::new(),
        }
    }
}

impl Light2dOccluderComponent {
    /// The outline's edges in the entity's own space, for a box or polygon.
    /// A tilemap occluder's edges come from its tilemap instead.
    pub fn local_edges(&self) -> Result<Vec<[[f32; 2]; 2]>, Light2dError> {
        let corners = match self.shape {
            OccluderShape::Box => {
                if !self
                    .size
                    .iter()
                    .all(|value| value.is_finite() && *value > 0.0)
                {
                    return Err(field("size", "must be finite numbers above 0"));
                }
                let [x, y] = self.size.map(|value| value / 2.0);
                vec![[-x, -y], [x, -y], [x, y], [-x, y]]
            }
            OccluderShape::Polygon => {
                if self.points.len() < 2 {
                    return Err(Light2dError::TooFewPoints);
                }
                if !self.points.iter().flatten().all(|value| value.is_finite()) {
                    return Err(field("points", "must be finite numbers"));
                }
                self.points.clone()
            }
            OccluderShape::Tilemap => return Ok(Vec::new()),
        };
        Ok(outline(&corners))
    }
}

/// The closed outline through `corners`: each to the next, and the last back
/// to the first. Two points are one edge, not two.
#[must_use]
pub fn outline(corners: &[[f32; 2]]) -> Vec<[[f32; 2]; 2]> {
    match corners.len() {
        0 | 1 => Vec::new(),
        2 => vec![[corners[0], corners[1]]],
        count => (0..count)
            .map(|index| [corners[index], corners[(index + 1) % count]])
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_is_four_edges_round_its_centre() {
        let edges = Light2dOccluderComponent {
            size: [2.0, 4.0],
            ..Light2dOccluderComponent::default()
        }
        .local_edges()
        .unwrap();
        assert_eq!(edges.len(), 4);
        assert_eq!(edges[0], [[-1.0, -2.0], [1.0, -2.0]]);
    }

    #[test]
    fn a_light_names_the_field_it_cannot_use() {
        let light = Light2dComponent {
            radius: 0.0,
            ..Light2dComponent::default()
        };
        assert_eq!(
            light.validate().unwrap_err().to_string(),
            "`radius` must be a finite number above 0"
        );
    }

    #[test]
    fn alpha_and_intensity_both_scale_the_light() {
        let light = Light2dComponent {
            color: [1.0, 0.5, 0.0, 0.5],
            intensity: 2.0,
            ..Light2dComponent::default()
        };
        assert_eq!(
            light.radiance().map(f32::to_bits),
            [1.0_f32, 0.5, 0.0].map(f32::to_bits)
        );
    }
}
