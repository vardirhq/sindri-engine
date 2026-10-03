//! Scene-authored configuration for an engine-owned voxel world.

use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;

use super::voxel_terrain::NaturalTerrainDocument;

/// How a scene deterministically supplies untouched voxel data.
///
/// Both built-in sources are portable: the editor constructs them without
/// loading game-specific Rust code. `layered_terrain` is a modest rolling
/// field of three layers; `natural_terrain` is a whole world of continents,
/// ranges, rivers, biomes, caves and trees. Games may still own richer
/// `VoxelSource` implementations.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VoxelGeneratorDocument {
    LayeredTerrain {
        #[serde(default)]
        seed: u64,
        #[serde(default = "default_base_height")]
        base_height: i32,
        #[serde(default = "default_height_variation")]
        height_variation: u32,
        #[serde(default = "surface_voxel")]
        surface_voxel: VoxelBlock,
        #[serde(default = "subsurface_voxel")]
        subsurface_voxel: VoxelBlock,
        #[serde(default = "deep_voxel")]
        deep_voxel: VoxelBlock,
        #[serde(default = "default_subsurface_depth")]
        subsurface_depth: u32,
    },
    NaturalTerrain(NaturalTerrainDocument),
}

impl VoxelGeneratorDocument {
    /// Every spelling of `kind`, in the order a picker offers them.
    pub const KINDS: [&'static str; 2] = ["layered_terrain", "natural_terrain"];
}

impl Default for VoxelGeneratorDocument {
    fn default() -> Self {
        Self::LayeredTerrain {
            seed: 0,
            base_height: default_base_height(),
            height_variation: default_height_variation(),
            surface_voxel: surface_voxel(),
            subsurface_voxel: subsurface_voxel(),
            deep_voxel: deep_voxel(),
            subsurface_depth: default_subsurface_depth(),
        }
    }
}

/// A block a generator lays down.
///
/// Named, in a world that names a block set: `"grass"` is the block called
/// grass in that set, and the number the engine stores it as is the engine's
/// business. A number is a material from the world's own `materials` list,
/// which is how every world was written before block sets, and still loads.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum VoxelBlock {
    Material(u16),
    Named(String),
}

impl From<u16> for VoxelBlock {
    fn from(voxel: u16) -> Self {
        Self::Material(voxel)
    }
}

impl From<&str> for VoxelBlock {
    fn from(name: &str) -> Self {
        Self::Named(name.to_owned())
    }
}

impl std::fmt::Display for VoxelBlock {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Material(voxel) => write!(formatter, "material {voxel}"),
            Self::Named(name) => write!(formatter, "block `{name}`"),
        }
    }
}

/// Renderer-facing appearance for one semantic voxel identity.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct VoxelMaterialDocument {
    pub voxel: u16,
    pub top: String,
    pub side: String,
    pub bottom: String,
}

/// A persistent, section-streamed voxel world rendered through the block
/// mesher and GPU section cache.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct VoxelWorldComponent {
    #[serde(default)]
    pub generator: VoxelGeneratorDocument,
    /// The tile set whose blocks this world is built from, such as
    /// `builtin:blocks` or a project's `terrain.tileset`.
    ///
    /// Set, the generator names blocks from it and `materials` is not used.
    /// Left out, the generator names materials by number, as before block
    /// sets existed.
    #[serde(default)]
    pub blocks: Option<String>,
    #[serde(default = "default_materials")]
    pub materials: Vec<VoxelMaterialDocument>,
    /// Section coordinate around which residency is maintained.
    #[serde(default)]
    pub focus: [i32; 3],
    #[serde(default = "default_render_radius")]
    pub render_radius: u32,
    #[serde(default = "default_vertical_radius")]
    pub vertical_radius: u32,
    #[serde(default)]
    pub layer: i32,
    /// Keep the resident window under the world camera rather than at
    /// `focus`.
    ///
    /// `focus` still says which height the window is centred on: the camera
    /// says where across the world, and terrain that rises and falls is what
    /// `vertical_radius` is for.
    #[serde(default)]
    pub follow_camera: bool,
    /// Which arrangement of its blocks' variants this world wears: the same
    /// seed lays the same look on the same cell every time.
    #[serde(default)]
    pub variant_seed: u64,
    /// What has been changed from what the generator lays down, in the order
    /// it was changed.
    ///
    /// Only the changes are stored: the world itself is the generator's
    /// answer, which costs nothing to keep. A later edit of a cell replaces an
    /// earlier one, and an edit back to what was generated is dropped.
    #[serde(default)]
    pub edits: Vec<VoxelEdit>,
}

/// One cell of a voxel world set to something other than what was generated.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VoxelEdit {
    /// The voxel, as world X, up, and Z.
    pub at: [i32; 3],
    /// What is there now: a block or material, or `""` for air.
    pub block: VoxelBlock,
}

impl VoxelBlock {
    /// Air, which is how a removed block is written.
    #[must_use]
    pub fn air() -> Self {
        Self::Named(String::new())
    }

    #[must_use]
    pub fn is_air(&self) -> bool {
        matches!(self, Self::Named(name) if name.is_empty()) || matches!(self, Self::Material(0))
    }
}

impl SceneComponent for VoxelWorldComponent {
    const TYPE_NAME: &'static str = "sindri.voxel_world";
}

const fn default_base_height() -> i32 {
    3
}

const fn default_height_variation() -> u32 {
    6
}

const fn default_subsurface_depth() -> u32 {
    3
}

const fn default_render_radius() -> u32 {
    2
}

const fn default_vertical_radius() -> u32 {
    1
}

const fn surface_voxel() -> VoxelBlock {
    VoxelBlock::Material(1)
}

const fn subsurface_voxel() -> VoxelBlock {
    VoxelBlock::Material(2)
}

const fn deep_voxel() -> VoxelBlock {
    VoxelBlock::Material(3)
}

fn default_materials() -> Vec<VoxelMaterialDocument> {
    (1..=3)
        .map(|voxel| VoxelMaterialDocument {
            voxel,
            top: "procedural:checkerboard".to_owned(),
            side: "procedural:checkerboard".to_owned(),
            bottom: "procedural:checkerboard".to_owned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_minimal_world_has_bounded_visible_defaults() {
        let component: VoxelWorldComponent = serde_json::from_str("{}").unwrap();
        assert_eq!(component.focus, [0, 0, 0]);
        assert_eq!(component.render_radius, 2);
        assert_eq!(component.vertical_radius, 1);
        assert_eq!(component.generator, VoxelGeneratorDocument::default());
        assert_eq!(component.materials.len(), 3);
    }
}
