//! Inspect the static model decoder without a GPU or editor.

use sindri_assets::{AssetBytes, AssetDecoder, ModelAssetDecoder};
use sindri_core::AssetId;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: model <path.glb>")?;
    let model = ModelAssetDecoder.decode(AssetBytes::new(
        AssetId::new("models/inspection.glb")?,
        std::fs::read(path)?,
    ))?;
    println!(
        "{} nodes, {} meshes, {} primitives, {} materials, {} textures",
        model.nodes.len(),
        model.meshes.len(),
        model
            .meshes
            .iter()
            .map(|mesh| mesh.primitives.len())
            .sum::<usize>(),
        model.materials.len(),
        model.textures.len(),
    );
    for warning in model.warnings {
        println!("warning: {warning}");
    }
    Ok(())
}
