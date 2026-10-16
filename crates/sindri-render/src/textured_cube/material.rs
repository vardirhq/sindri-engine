use super::TexturedCubeRenderer;
use crate::{MaterialError, MaterialFactors};

impl TexturedCubeRenderer {
    /// Selects face visibility for subsequent world color and shadow draws.
    pub const fn set_sidedness(&mut self, sidedness: crate::MaterialSidedness) {
        self.sidedness = sidedness;
    }

    /// Selects standard factors for subsequent draws; `None` uses legacy mesh
    /// lighting or imported primitive factors. Frame preparation validates them.
    pub fn set_material(&mut self, material: Option<MaterialFactors>) -> Result<(), MaterialError> {
        if let Some(factors) = material {
            factors.validate()?;
        }
        self.material = material;
        Ok(())
    }
}
