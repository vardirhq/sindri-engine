//! Renderer-owned world lighting shared by every textured 3D draw.

/// Maximum point and spot lights shared by every 3D draw in a frame.
pub const MAX_LOCAL_LIGHTS: usize = 8;

/// A world-space, unshadowed point light. Range is in world units and ignores
/// the carrying entity's scale. Colors and intensity are linear, nonnegative.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLight {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
}

/// Why a point light cannot be sent to the GPU.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PointLightError {
    #[error("point light position must be finite")]
    Position,
    #[error("point light color must be finite and nonnegative")]
    Color,
    #[error("point light intensity must be finite and nonnegative")]
    Intensity,
    #[error("point light range must be finite and greater than zero")]
    Range,
    #[error("spotlight direction must be finite and normalizable")]
    Direction,
    #[error("spotlight half-angles must satisfy 0 <= inner < outer <= pi/2 with distinct cosines")]
    Cone,
}

impl PointLight {
    /// Checks authored or low-level renderer input before it reaches shading.
    pub fn validate(self) -> Result<Self, PointLightError> {
        if !self.position.iter().all(|value| value.is_finite()) {
            return Err(PointLightError::Position);
        }
        if !self
            .color
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
        {
            return Err(PointLightError::Color);
        }
        if !self.intensity.is_finite() || self.intensity < 0.0 {
            return Err(PointLightError::Intensity);
        }
        if !self.range.is_finite() || self.range <= 0.0 {
            return Err(PointLightError::Range);
        }
        Ok(self)
    }
}

/// Angular shape of an unshadowed spotlight. Angles are half-angles in radians;
/// direction is the finite, nonzero world-space direction the light travels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpotCone {
    pub direction: [f32; 3],
    pub inner_angle: f32,
    pub outer_angle: f32,
}

impl SpotCone {
    /// Checks the direction and a nonempty feather interval within a hemisphere.
    pub fn validate(self) -> Result<Self, PointLightError> {
        let direction = glam::Vec3::from_array(self.direction);
        if !direction.is_finite() || direction.normalize_or_zero() == glam::Vec3::ZERO {
            return Err(PointLightError::Direction);
        }
        if !self.inner_angle.is_finite()
            || !self.outer_angle.is_finite()
            || self.inner_angle < 0.0
            || self.inner_angle >= self.outer_angle
            || self.outer_angle > std::f32::consts::FRAC_PI_2
            || self.inner_angle.cos() <= self.outer_angle.cos()
        {
            return Err(PointLightError::Cone);
        }
        Ok(self)
    }
}

/// A point source, optionally restricted to a spotlight cone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalLight {
    pub point: PointLight,
    pub spot: Option<SpotCone>,
}

impl From<PointLight> for LocalLight {
    fn from(point: PointLight) -> Self {
        Self { point, spot: None }
    }
}

impl LocalLight {
    /// Validates both the source and its optional angular restriction.
    pub fn validate(self) -> Result<Self, PointLightError> {
        self.point.validate()?;
        if let Some(spot) = self.spot {
            spot.validate()?;
        }
        Ok(self)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct LocalLightUniform {
    position_range: [f32; 4],
    color_intensity: [f32; 4],
    direction_outer: [f32; 4],
    inner_spot: [f32; 4],
}

/// Ambient, one directional light, and a bounded set of point and spot lights.
///
/// Direction is the direction the light travels, from the source toward the
/// world. The default preserves the historical unlit appearance: white ambient
/// at full strength and no directional contribution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldLighting {
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
    pub directional_direction: [f32; 3],
    pub directional_color: [f32; 3],
    pub directional_intensity: f32,
    /// Unused slots are `None`. Invalid low-level inputs are packed as disabled
    /// lights; scene extraction reports authored errors before reaching here.
    pub local_lights: [Option<LocalLight>; MAX_LOCAL_LIGHTS],
}

impl Default for WorldLighting {
    fn default() -> Self {
        Self {
            ambient_color: [1.0; 3],
            ambient_intensity: 1.0,
            directional_direction: [0.0, -1.0, 0.0],
            directional_color: [1.0; 3],
            directional_intensity: 0.0,
            local_lights: [None; MAX_LOCAL_LIGHTS],
        }
    }
}

impl WorldLighting {
    pub(crate) fn local_uniforms(&self) -> [LocalLightUniform; MAX_LOCAL_LIGHTS] {
        self.local_lights.each_ref().map(|light| {
            light.and_then(|light| light.validate().ok()).map_or_else(
                LocalLightUniform::default,
                |local| {
                    let light = local.point;
                    let spot = local.spot;
                    let direction = spot.map_or(glam::Vec3::ZERO, |s| {
                        glam::Vec3::from_array(s.direction).normalize()
                    });
                    LocalLightUniform {
                        position_range: [
                            light.position[0],
                            light.position[1],
                            light.position[2],
                            light.range,
                        ],
                        color_intensity: [
                            light.color[0],
                            light.color[1],
                            light.color[2],
                            light.intensity,
                        ],
                        direction_outer: [
                            direction.x,
                            direction.y,
                            direction.z,
                            spot.map_or(0.0, |s| s.outer_angle.cos()),
                        ],
                        inner_spot: [
                            spot.map_or(0.0, |s| s.inner_angle.cos()),
                            f32::from(u8::from(spot.is_some())),
                            0.0,
                            0.0,
                        ],
                    }
                },
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cones_require_a_representable_cosine_feather_interval() {
        let cone = SpotCone {
            direction: [0.0, 0.0, -2.0],
            inner_angle: 0.0,
            outer_angle: 0.5,
        };
        assert!(cone.validate().is_ok());
        for (inner_angle, outer_angle) in [
            (0.0, 0.00001),
            (0.5, 0.5),
            (-0.1, 0.5),
            (0.5, 2.0),
            (0.0, f32::NAN),
        ] {
            assert_eq!(
                SpotCone {
                    inner_angle,
                    outer_angle,
                    ..cone
                }
                .validate(),
                Err(PointLightError::Cone)
            );
        }
    }
}
