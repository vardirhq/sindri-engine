//! Renderer-owned world lighting shared by every textured 3D draw.

/// Maximum point lights shared by every 3D draw in a frame.
pub const MAX_POINT_LIGHTS: usize = 8;

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

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PointLightUniform {
    position_range: [f32; 4],
    color_intensity: [f32; 4],
}

/// Ambient, one directional light, and a bounded set of point lights.
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
    pub point_lights: [Option<PointLight>; MAX_POINT_LIGHTS],
}

impl Default for WorldLighting {
    fn default() -> Self {
        Self {
            ambient_color: [1.0; 3],
            ambient_intensity: 1.0,
            directional_direction: [0.0, -1.0, 0.0],
            directional_color: [1.0; 3],
            directional_intensity: 0.0,
            point_lights: [None; MAX_POINT_LIGHTS],
        }
    }
}

impl WorldLighting {
    pub(crate) fn point_uniforms(&self) -> [PointLightUniform; MAX_POINT_LIGHTS] {
        self.point_lights.each_ref().map(|light| {
            light.and_then(|light| light.validate().ok()).map_or_else(
                PointLightUniform::default,
                |light| PointLightUniform {
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
                },
            )
        })
    }
}
