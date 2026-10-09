//! The engine's own tile sets, bound before anything a project names.
//!
//! A `builtin:` reference is not a file: the exporter leaves it out of a
//! site's manifest because the engine provides it, so no loader will fetch
//! it, and a host that does not bind it has a voxel world naming
//! `builtin:blocks` that fails to step. Their textures need a GPU, which this
//! crate does not have, so a host binds those itself from
//! `sindri_assets::builtin_textures`.

use sindri_assets::builtin_tile_sets;
use sindri_scene::TileSetBindings;

use crate::RuntimeError;

/// Binds every tile set the engine ships.
pub fn bind_builtin_tile_sets(tile_sets: &mut TileSetBindings) -> Result<(), RuntimeError> {
    for built_in in builtin_tile_sets() {
        let (reference, tile_set) = built_in?;
        tile_sets.bind(reference, tile_set)?;
    }
    Ok(())
}
