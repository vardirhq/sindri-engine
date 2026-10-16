//! Inspect the static model decoder without a GPU or editor.

use sindri_assets::{AssetBytes, AssetDecoder, ModelAssetDecoder, ModelCollisionLimits};
use sindri_core::AssetId;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("usage: model <path.glb> [--collision]")?;
    let collision = match args.next().as_deref() {
        None => false,
        Some("--collision") => true,
        Some(_) => return Err("usage: model <path.glb> [--collision]".into()),
    };
    if args.next().is_some() {
        return Err("usage: model <path.glb> [--collision]".into());
    }
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
    if collision {
        let mesh = model.collision_mesh(ModelCollisionLimits::default())?;
        println!(
            "collision preparation: {} vertices, {} triangles, {} primitive instances",
            mesh.positions.len(),
            mesh.triangles.len(),
            mesh.parts.len()
        );
    }
    for warning in model.warnings {
        println!("warning: {warning}");
    }
    Ok(())
}
