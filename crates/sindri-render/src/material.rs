//! Numeric standard-material factors, independent of texture ownership.

use thiserror::Error;

/// Linear factors shared by textured geometry and imported model primitives.
/// Textures, alpha mode and sidedness remain owned by their geometry resources.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialFactors {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    /// Linear self-emission, permitted above one for HDR targets.
    pub emissive: [f32; 3],
}

impl Default for MaterialFactors {
    fn default() -> Self {
        Self {
            base_color: [1.0; 4],
            metallic: 0.0,
            roughness: 1.0,
            emissive: [0.0; 3],
        }
    }
}

impl MaterialFactors {
    pub fn validate(self) -> Result<(), MaterialError> {
        let unit = |v: f32| v.is_finite() && (0.0..=1.0).contains(&v);
        if !self.base_color.into_iter().all(unit) {
            return Err(MaterialError::InvalidFactor("base_color"));
        }
        if !unit(self.metallic) {
            return Err(MaterialError::InvalidFactor("metallic"));
        }
        if !unit(self.roughness) {
            return Err(MaterialError::InvalidFactor("roughness"));
        }
        if !self.emissive.into_iter().all(|v| v.is_finite() && v >= 0.0) {
            return Err(MaterialError::InvalidFactor("emissive"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum MaterialError {
    #[error("invalid material factor `{0}`: expected finite unit factors or nonnegative emission")]
    InvalidFactor(&'static str),
}
