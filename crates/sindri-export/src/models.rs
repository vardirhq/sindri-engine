//! Imported models stay external and ship as their original verified bytes.

use crate::{ExportError, GatheredAsset};
use sindri_assets::{AssetBytes, AssetDecoder, AssetKind, ModelAssetDecoder};
use sindri_core::{AssetId, World};
use std::collections::BTreeMap;

pub(crate) fn references(world: &World, wanted: &mut BTreeMap<String, AssetKind>) {
    for id in sindri_scene::referenced_models(world) {
        wanted.insert(id, AssetKind::Model);
    }
}

pub(crate) fn validate(assets: &[GatheredAsset]) -> Result<(), ExportError> {
    for asset in assets.iter().filter(|asset| asset.kind == AssetKind::Model) {
        let id = AssetId::new(asset.id.clone())
            .map_err(|error| ExportError::Project(error.to_string()))?;
        ModelAssetDecoder
            .decode(AssetBytes::new(id, asset.bytes.clone()))
            .map_err(|error| ExportError::Project(error.to_string()))?;
    }
    Ok(())
}
