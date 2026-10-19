//! Resolve reusable authored values before constructing renderer draw state.

use sindri_core::{EntityId, SceneComponent, World};
use sindri_render::FramePass;

use super::{SceneExtractError, SceneExtractor};
use crate::{MaterialComponent, RenderMaterialSources, ResolvedRenderMaterial};

impl SceneExtractor {
    /// Replaces already validated, asynchronously loaded material profiles.
    /// Frame packets and retained geometry remain independent of this source set.
    pub fn set_materials(&mut self, materials: RenderMaterialSources) {
        self.materials = materials;
    }

    pub(super) fn resolved_material(
        &self,
        world: &World,
        entity: EntityId,
    ) -> Result<Option<ResolvedRenderMaterial>, SceneExtractError> {
        self.components
            .get::<MaterialComponent>(world, entity)?
            .as_ref()
            .map(|component| self.materials.resolve(component))
            .transpose()
            .map_err(Into::into)
    }

    pub(super) fn apply_material(
        &self,
        world: &World,
        entity: EntityId,
        pass: &mut FramePass,
    ) -> Result<bool, SceneExtractError> {
        match self.resolved_material(world, entity) {
            Ok(Some(material)) => {
                pass.material = Some(material.factors);
                pass.sidedness = material.sidedness;
            }
            Ok(None) => {}
            Err(error) if self.tolerant() => {
                self.record(entity, MaterialComponent::TYPE_NAME, &error);
                return Ok(false);
            }
            Err(error) => return Err(error),
        }
        Ok(true)
    }
}
