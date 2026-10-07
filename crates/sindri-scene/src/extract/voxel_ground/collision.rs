//! A voxel world as solid geometry: which blocks a 3D body collides with, the
//! box each one fills, and whole resident sections with their edits applied.
//!
//! Collision is its own answer rather than `supports` or `walkable` read again.
//! Those are what a walker stands on; this is what a falling crate hits.

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::PoisonError;

use sindri_voxel::{
    LocalVoxelCoord, SECTION_EDGE, SectionCoord, VoxelId, VoxelSection, VoxelShape, VoxelSource,
};

use super::super::voxel_appearance::Appearance;
use super::{GeneratedTerrain, VoxelGround};

/// Every block's collision box, by stored voxel. A block set's tile says
/// whether it collides and fills its own bounds; a world of numbered
/// materials is whole cubes, the way it draws.
pub(super) fn shapes(appearance: &Appearance) -> BTreeMap<u16, Option<VoxelShape>> {
    match appearance {
        Appearance::Blocks(blocks) => blocks
            .iter()
            .map(|(voxel, _, definition)| {
                let bounds = definition.bounds();
                let shape = definition
                    .collides
                    .then(|| VoxelShape::from_fractions(bounds.min, bounds.max));
                (*voxel, shape)
            })
            .collect(),
        Appearance::Materials(materials) => materials
            .iter()
            .map(|material| (material.voxel, Some(VoxelShape::FULL)))
            .collect(),
    }
}

impl GeneratedTerrain {
    /// One whole generated section, remembered like a single voxel's.
    fn section(&self, coord: SectionCoord) -> VoxelSection {
        let mut sections = self.sections.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(generated) = sections.get(&coord) {
            return generated.clone();
        }
        let generated = self.key.source.generate_section(coord);
        if sections.len() >= super::REMEMBERED_SECTIONS {
            sections.clear();
        }
        sections.insert(coord, generated.clone());
        generated
    }
}

impl VoxelGround {
    /// The box a block fills for a 3D body, or `None` for air and for a block
    /// nothing collides with.
    #[must_use]
    pub fn collision_shape(&self, voxel: VoxelId) -> Option<VoxelShape> {
        if voxel.is_air() {
            return None;
        }
        self.collision.get(&voxel.value()).copied().flatten()
    }

    /// Changes whenever any block's collision answer does, and only then.
    #[must_use]
    pub fn collision_revision(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.collision.hash(&mut hasher);
        hasher.finish()
    }

    /// One section as it stands: what generation put there, with this
    /// world's edits on top.
    #[must_use]
    pub fn section(&self, coord: SectionCoord) -> VoxelSection {
        let mut section = self.terrain.section(coord);
        let first = coord.min_voxel();
        let inside = |axis: i32, start: i32| (start..start + SECTION_EDGE).contains(&axis);
        for (at, voxel) in &self.edits {
            if inside(at.x, first.x) && inside(at.y, first.y) && inside(at.z, first.z) {
                section.set(at.local(), *voxel);
            }
        }
        section
    }

    /// Changes whenever a section's voxels do, wherever the change came from:
    /// an edit, or a different generator in its place.
    #[must_use]
    pub fn section_revision(section: &VoxelSection) -> u64 {
        let mut hasher = DefaultHasher::new();
        let edge = u8::try_from(SECTION_EDGE).expect("a section edge fits a byte");
        for y in 0..edge {
            for z in 0..edge {
                for x in 0..edge {
                    section
                        .get(LocalVoxelCoord::new(x, y, z))
                        .value()
                        .hash(&mut hasher);
                }
            }
        }
        hasher.finish()
    }
}
