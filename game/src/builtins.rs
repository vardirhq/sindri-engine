//! The engine's own assets, bound before anything a project names.
//!
//! A `builtin:` reference is not a file: the exporter leaves it out of a
//! site's manifest because the engine provides it (`sindri-export`'s
//! `engine_provided`), so no loader will fetch it and a host that does not bind
//! it has a voxel world that names `builtin:blocks` and fails to draw or step.
//! The editor has always bound them; every host here does too now, the same
//! way, so a project that plays in the editor plays in a build.

use sindri_assets::{BuiltinError, builtin_textures, builtin_tile_sets};
use sindri_render::{Texture2D, TextureRegistry};
use sindri_scene::{TextureBindings, TileSetBindings};

use crate::CausewayError;

/// Binds every tile set the engine ships.
pub fn bind_builtin_tile_sets(tile_sets: &mut TileSetBindings) -> Result<(), CausewayError> {
    for built_in in builtin_tile_sets() {
        let (reference, tile_set) = built_in?;
        tile_sets.bind(reference, tile_set)?;
    }
    Ok(())
}

/// Uploads every texture the engine ships and binds it, with its sheet.
pub fn bind_builtin_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    registry: &mut TextureRegistry,
    bindings: &mut TextureBindings,
) -> Result<(), CausewayError> {
    for texture in builtin_textures() {
        let asset = texture.decode()?;
        let uploaded = Texture2D::from_rgba8(
            device,
            queue,
            texture.reference,
            asset.width(),
            asset.height(),
            asset.rgba8(),
        )?;
        bindings.bind(texture.reference, registry.insert(uploaded));
        if let Some(sheet) = texture.sheet() {
            bindings.bind_sheet(texture.reference, &sheet?)?;
        }
    }
    Ok(())
}

impl From<BuiltinError> for CausewayError {
    fn from(error: BuiltinError) -> Self {
        Self::Builtin(Box::new(error))
    }
}
