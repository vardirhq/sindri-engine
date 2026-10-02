use std::collections::BTreeSet;

use sindri_render::FrameCommand;
use sindri_scene::{VoxelRenderBridge, VoxelRenderError, VoxelTextureSource, compile_block_mesh};
use sindri_voxel::{
    MeshingProfile, NaturalTerrain, NaturalTerrainSettings, ResidencyConfig, SectionCoord,
    SectionMeshKey, TerrainBiome, TerrainPalette, VoxelCoord, VoxelId, VoxelSource, VoxelWorld,
    mesh_block_section,
};

pub const GRASS: VoxelId = VoxelId::new(1);
pub const DIRT: VoxelId = VoxelId::new(2);
pub const STONE: VoxelId = VoxelId::new(3);
pub const SAND: VoxelId = VoxelId::new(4);
pub const ROCK: VoxelId = VoxelId::new(5);
pub const SNOW: VoxelId = VoxelId::new(6);
pub const WATER: VoxelId = VoxelId::new(7);
pub const LOG: VoxelId = VoxelId::new(8);
pub const LEAVES: VoxelId = VoxelId::new(9);
pub const MUD: VoxelId = VoxelId::new(10);
pub const MOSS: VoxelId = VoxelId::new(11);
pub const GRAVEL: VoxelId = VoxelId::new(12);
pub const CLAY: VoxelId = VoxelId::new(13);
pub const ICE: VoxelId = VoxelId::new(14);

/// The engine's natural terrain generator, configured as a compact showcase.
#[derive(Clone, Debug, PartialEq)]
pub struct LabTerrain {
    natural: NaturalTerrain,
}

impl Default for LabTerrain {
    fn default() -> Self {
        Self {
            natural: NaturalTerrain::new(lab_terrain_settings()),
        }
    }
}

impl LabTerrain {
    #[must_use]
    pub fn ground(&self, x: i32, z: i32) -> i32 {
        self.natural.ground(x, z)
    }
}

impl VoxelSource for LabTerrain {
    fn voxel(&self, coord: VoxelCoord) -> VoxelId {
        self.natural.voxel(coord)
    }

    fn generate_section(&self, section: SectionCoord) -> sindri_voxel::VoxelSection {
        self.natural.generate_section(section)
    }
}

fn biome(
    temperature: f32,
    moisture: f32,
    surface: VoxelId,
    subsurface: VoxelId,
    trees: f32,
    relief: f32,
    terraces: u32,
) -> TerrainBiome {
    TerrainBiome {
        temperature,
        moisture,
        surface,
        subsurface,
        subsurface_depth: 3,
        trees,
        relief,
        terraces,
    }
}

fn lab_terrain_settings() -> NaturalTerrainSettings {
    NaturalTerrainSettings {
        seed: 0x51_4E_44_52_49,
        sea_level: 0,
        relief: 28,
        feature_size: 72,
        tree_line: 18,
        snow_line: 25,
        caves: true,
        rivers: true,
        palette: TerrainPalette {
            stone: STONE,
            water: Some(WATER),
            beach: Some(SAND),
            sea_bed: Some(GRAVEL),
            cliff: Some(ROCK),
            snow: Some(SNOW),
            ice: Some(ICE),
            trunk: Some(LOG),
            leaves: Some(LEAVES),
        },
        biomes: vec![
            biome(0.55, 0.38, GRASS, DIRT, 0.06, 0.75, 0),
            biome(0.55, 0.74, GRASS, DIRT, 0.48, 1.0, 0),
            biome(0.78, 0.90, MUD, MUD, 0.16, 0.35, 0),
            biome(0.92, 0.10, SAND, SAND, 0.0, 0.55, 0),
            biome(0.84, 0.34, CLAY, CLAY, 0.0, 1.35, 3),
            biome(0.28, 0.66, MOSS, DIRT, 0.38, 1.05, 0),
            biome(0.10, 0.30, SNOW, DIRT, 0.02, 0.65, 0),
        ],
    }
}

/// Observable work performed while producing one Voxel Lab frame.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VoxelLabStats {
    pub resident_sections: usize,
    pub entering_sections: usize,
    pub leaving_sections: usize,
    pub mesh_jobs: usize,
    pub remeshes: usize,
    pub compiled_sections: usize,
    pub cache_batches: usize,
    pub triangles: usize,
    pub uploads: usize,
    pub releases: usize,
}

/// Renderer commands and counters produced from the authoritative voxel world.
#[derive(Debug)]
pub struct VoxelLabFrame {
    pub commands: Vec<FrameCommand>,
    pub stats: VoxelLabStats,
}

/// Small synchronous driver for the same queue/cache flow an asynchronous host
/// will drain later. Camera motion only queues newly entering sections.
pub struct VoxelLabRuntime {
    terrain: LabTerrain,
    world: VoxelWorld<LabTerrain>,
    render: VoxelRenderBridge,
    resident: BTreeSet<SectionCoord>,
}

impl Default for VoxelLabRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl VoxelLabRuntime {
    #[must_use]
    pub fn new() -> Self {
        let terrain = LabTerrain::default();
        Self {
            // Five-by-five horizontally, with enough vertical depth for the
            // natural generator's caves, valleys, trees and modest peaks.
            world: VoxelWorld::new(terrain.clone(), ResidencyConfig::new(2, 2, 0, 0)),
            terrain,
            render: VoxelRenderBridge::default(),
            resident: BTreeSet::new(),
        }
    }

    /// Applies an edit to authoritative voxel data. The next frame drains only
    /// the affected section work, including a resident boundary neighbour.
    pub fn set_voxel(&mut self, coord: VoxelCoord, voxel: VoxelId) -> bool {
        self.world.set_voxel(coord, voxel)
    }

    /// Removes the generated surface voxel at a world column.
    pub fn dig_surface(&mut self, x: i32, z: i32) -> bool {
        self.set_voxel(
            VoxelCoord::new(x, self.terrain.ground(x, z), z),
            VoxelId::AIR,
        )
    }

    /// Moves residency to the camera section and drains the current CPU work.
    pub fn frame(
        &mut self,
        focus: SectionCoord,
        textures: &impl VoxelTextureSource,
    ) -> Result<VoxelLabFrame, VoxelRenderError> {
        let delta = self.world.move_focus(focus);
        for section in &delta.left {
            self.resident.remove(section);
            self.render.remove_section(*section);
        }
        self.resident.extend(delta.entered.iter().copied());

        let jobs = self.world.take_mesh_work();
        for job in &jobs {
            if self.render.schedule(*job) {
                let mesh = mesh_block_section(&self.world, job.key.section);
                let compiled = compile_block_mesh(&mesh, textures)?;
                self.render.finish(*job, compiled)?;
            }
        }

        let bridge_stats = self.render.stats();
        let uploads = bridge_stats.pending_uploads;
        let mut commands = self.render.take_release_commands();
        let releases = commands.len();
        for section in &self.resident {
            commands.extend(self.render.draw_commands(
                SectionMeshKey::new(*section, MeshingProfile::Block),
                &|_| sindri_render::MeshSurface::default(),
            ));
        }

        Ok(VoxelLabFrame {
            commands,
            stats: VoxelLabStats {
                resident_sections: self.world.resident_len(),
                entering_sections: delta.entered.len(),
                leaving_sections: delta.left.len(),
                mesh_jobs: jobs.len(),
                remeshes: jobs.len().saturating_sub(delta.entered.len()),
                compiled_sections: bridge_stats.compiled_sections,
                cache_batches: bridge_stats.batches,
                triangles: bridge_stats.triangles,
                uploads,
                releases,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use sindri_render::{TextureId, UvRect};
    use sindri_scene::VoxelTexture;

    use super::*;

    fn texture(_voxel: VoxelId, _face: sindri_voxel::VoxelFace) -> VoxelTexture {
        VoxelTexture::new(TextureId::new(1), UvRect::FULL)
    }

    #[test]
    fn settled_camera_produces_zero_mesh_work_or_uploads() {
        let mut lab = VoxelLabRuntime::new();
        let focus = SectionCoord::new(0, 0, 0);
        let first = lab.frame(focus, &texture).unwrap();
        assert_eq!(first.stats.entering_sections, 125);
        assert!(first.stats.mesh_jobs > 0);
        assert!(first.stats.uploads > 0);

        let settled = lab.frame(focus, &texture).unwrap();
        assert_eq!(settled.stats.entering_sections, 0);
        assert_eq!(settled.stats.mesh_jobs, 0);
        assert_eq!(settled.stats.remeshes, 0);
        assert_eq!(settled.stats.uploads, 0);
    }

    #[test]
    fn boundary_edit_remeshes_both_resident_sections() {
        let mut lab = VoxelLabRuntime::new();
        let focus = SectionCoord::new(0, 0, 0);
        lab.frame(focus, &texture).unwrap();
        assert!(lab.set_voxel(VoxelCoord::new(15, -8, 4), VoxelId::AIR));

        let edited = lab.frame(focus, &texture).unwrap();
        assert_eq!(edited.stats.entering_sections, 0);
        assert_eq!(edited.stats.mesh_jobs, 2);
        assert_eq!(edited.stats.remeshes, 2);
    }

    #[test]
    fn moving_one_section_only_meshes_the_entering_edge() {
        let mut lab = VoxelLabRuntime::new();
        lab.frame(SectionCoord::new(0, 0, 0), &texture).unwrap();

        let moved = lab.frame(SectionCoord::new(1, 0, 0), &texture).unwrap();
        assert_eq!(moved.stats.entering_sections, 25);
        assert_eq!(moved.stats.leaving_sections, 25);
        assert_eq!(moved.stats.mesh_jobs, 25);
        assert_eq!(moved.stats.remeshes, 0);
        assert!(moved.stats.releases > 0);
    }

    #[test]
    fn lab_uses_engine_natural_terrain_and_real_depth() {
        let terrain = LabTerrain::default();
        assert_eq!(terrain.voxel(VoxelCoord::new(0, -32, 0)), STONE);

        let mut biomes = BTreeSet::new();
        for z in (-192..=192).step_by(24) {
            for x in (-192..=192).step_by(24) {
                biomes.insert(terrain.natural.biome(x, z));
            }
        }
        assert!(biomes.len() > 1);
    }
}
