//! Typed material references are dependency edges even without a script export.

use crate::ExportError;
use sindri_assets::AssetKind;
use sindri_core::{ComponentSchemaRegistry, World};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(crate) struct ProfileReferences {
    physics: BTreeSet<String>,
    render: BTreeSet<String>,
}

impl ProfileReferences {
    pub(crate) fn collect(
        &mut self,
        world: &World,
        components: &ComponentSchemaRegistry,
        wanted: &mut BTreeMap<String, AssetKind>,
    ) -> Result<(), ExportError> {
        let physics = sindri_scene::referenced_physics_materials(world, components)
            .map_err(|error| ExportError::Project(error.to_string()))?;
        let render = sindri_scene::referenced_render_materials(world, components)
            .map_err(|error| ExportError::Project(error.to_string()))?;
        for id in physics {
            wanted.insert(id.clone(), AssetKind::Profile);
            self.physics.insert(id);
        }
        for id in render {
            wanted.insert(id.clone(), AssetKind::Profile);
            self.render.insert(id);
        }
        Ok(())
    }

    pub(crate) fn validate(&self, id: &str, bytes: &[u8]) -> Result<(), ExportError> {
        let bad = |error: String| ExportError::Project(format!("{id}: {error}"));
        let text = std::str::from_utf8(bytes).map_err(|error| bad(error.to_string()))?;
        let profile = sindri_core::ProfileDocument::from_json(text)
            .map_err(|error| bad(error.to_string()))?;
        let physics = sindri_scene::physics_material_profile(id, &profile)
            .map_err(|error| ExportError::Project(error.to_string()))?;
        let render = sindri_scene::render_material_profile(id, &profile)
            .map_err(|error| ExportError::Project(error.to_string()))?;
        if self.physics.contains(id) && physics.is_none() {
            return Err(bad(
                "physics material reference needs type physics_material".into(),
            ));
        }
        if self.render.contains(id) && render.is_none() {
            return Err(bad(
                "render material reference needs type render_material".into()
            ));
        }
        Ok(())
    }
}
