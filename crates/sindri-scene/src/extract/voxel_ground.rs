//! A voxel world as ground: what is in a cell, where a column's surface is,
//! and what a ray through the world touches first.
//!
//! The renderer keeps its own resident sections, because it needs them meshed
//! and on the GPU. Gameplay asks different questions -- one cell, one column,
//! one ray -- often outside whatever the camera has loaded, and it asks them
//! from places that cannot reach the renderer: a script, a placement, the
//! pathfinder. So the answer is worked out here from the same two things the
//! renderer uses, the generator and the world's edits, and agrees with it.
//!
//! Generated sections are remembered between questions, keyed by everything
//! that decides them. Edits are not part of that key: they lie on top of what
//! was generated, so a block placed or taken away costs a map entry rather
//! than regenerating the terrain around it.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, PoisonError};

use sindri_core::{TileDefinition, TileSetDocument};
use sindri_grid::GridCoord3;
use sindri_voxel::{SectionCoord, VoxelCoord, VoxelId, VoxelSection, VoxelShape, VoxelSource};

use crate::{TileSetBindings, TileSurfaces, VoxelBlock, VoxelEdit, VoxelWorldComponent};

use super::SceneExtractError;
use super::voxel_appearance::{Appearance, block_palette};
use super::voxel_source::{Palette, SceneTerrain, terrain_source};

mod collision;
mod raycast;

/// How far above a column's ground anything generated may stand: the tallest
/// tree, and the top of an overhang's band.
const ABOVE_GROUND: i32 = 32;
/// How many generated sections a terrain remembers before starting again.
const REMEMBERED_SECTIONS: usize = 4_096;
/// How many columns' generated surfaces a terrain remembers.
const REMEMBERED_COLUMNS: usize = 262_144;
/// How many differently generated worlds are remembered at once.
const REMEMBERED_TERRAINS: usize = 4;

/// What decides a world's generated voxels and where its ground is: its
/// generator, how its blocks are numbered, and what each block is to stand
/// on. Two worlds that agree on all three have the same ground.
#[derive(Clone, Debug, PartialEq)]
struct TerrainKey {
    source: SceneTerrain,
    palette: Palette,
    footing: BTreeMap<u16, Footing>,
}

/// A generator, and what it has been asked so far: the sections it made, and
/// where each column's generated surface is.
struct GeneratedTerrain {
    key: TerrainKey,
    sections: Mutex<BTreeMap<SectionCoord, VoxelSection>>,
    /// Each column's highest generated block that holds something up, or
    /// `None` where none does. A column's surface is asked far more often than
    /// it changes -- every placement, and every column a path is looked for
    /// across -- and finding it is a walk down the column.
    tops: Mutex<HashMap<(i32, i32), Option<i32>>>,
}

impl GeneratedTerrain {
    fn footing(&self, voxel: VoxelId) -> Option<Footing> {
        (!voxel.is_air())
            .then(|| self.key.footing.get(&voxel.value()).copied())
            .flatten()
    }

    /// The highest voxel generation can put anything in at a column.
    fn ceiling(&self, x: i32, z: i32) -> i32 {
        match &self.key.source {
            SceneTerrain::Layered(terrain) => terrain.highest(),
            SceneTerrain::Natural(terrain) => {
                terrain.ground(x, z).max(terrain.settings().sea_level) + ABOVE_GROUND
            }
        }
    }

    /// The lowest voxel a column's surface is looked for in.
    fn floor(&self, x: i32, z: i32) -> i32 {
        match &self.key.source {
            SceneTerrain::Layered(_) => 0,
            SceneTerrain::Natural(terrain) => {
                terrain.ground(x, z).min(terrain.settings().sea_level) - ABOVE_GROUND
            }
        }
    }

    /// The highest generated block in a column that holds something up.
    fn top(&self, x: i32, z: i32) -> Option<i32> {
        if let Some(known) = self
            .tops
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&(x, z))
        {
            return *known;
        }
        let floor = self.floor(x, z);
        let mut y = self.ceiling(x, z);
        let mut found = None;
        while y >= floor {
            if self
                .footing(self.voxel(VoxelCoord::new(x, y, z)))
                .is_some_and(|footing| footing.supports)
            {
                found = Some(y);
                break;
            }
            y -= 1;
        }
        let mut tops = self.tops.lock().unwrap_or_else(PoisonError::into_inner);
        if tops.len() >= REMEMBERED_COLUMNS {
            tops.clear();
        }
        tops.insert((x, z), found);
        found
    }

    fn voxel(&self, coord: VoxelCoord) -> VoxelId {
        let section = coord.section();
        let mut sections = self.sections.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(generated) = sections.get(&section) {
            return generated.get(coord.local());
        }
        let generated = self.key.source.generate_section(section);
        let voxel = generated.get(coord.local());
        if sections.len() >= REMEMBERED_SECTIONS {
            sections.clear();
        }
        sections.insert(section, generated);
        voxel
    }
}

thread_local! {
    static TERRAINS: RefCell<Vec<Arc<GeneratedTerrain>>> = const { RefCell::new(Vec::new()) };
}

/// The remembered terrain for `key`, most recently used first.
fn terrain_for(key: TerrainKey) -> Arc<GeneratedTerrain> {
    TERRAINS.with(|terrains| {
        let mut terrains = terrains.borrow_mut();
        if let Some(index) = terrains.iter().position(|terrain| terrain.key == key) {
            let terrain = terrains.remove(index);
            terrains.insert(0, Arc::clone(&terrain));
            return terrain;
        }
        let terrain = Arc::new(GeneratedTerrain {
            key,
            sections: Mutex::new(BTreeMap::new()),
            tops: Mutex::new(HashMap::new()),
        });
        terrains.insert(0, Arc::clone(&terrain));
        terrains.truncate(REMEMBERED_TERRAINS);
        terrain
    })
}

/// What a block is to something standing on it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Footing {
    /// Holds something up. Water does; a flower does not.
    supports: bool,
    /// Something can walk on it. Water holds a boat up and not a walker.
    walkable: bool,
    /// How much of its cell it fills from the bottom: one for a block, a half
    /// for a slab.
    top: f32,
}

/// One voxel world's cells, as gameplay sees them.
#[derive(Clone)]
pub struct VoxelGround {
    terrain: Arc<GeneratedTerrain>,
    edits: BTreeMap<VoxelCoord, VoxelId>,
    /// The highest edited voxel in each column that has any.
    edited_columns: HashMap<(i32, i32), i32>,
    names: BTreeMap<u16, String>,
    footing: BTreeMap<u16, Footing>,
    /// What each block is to a 3D body: its box, or nothing it collides with.
    collision: BTreeMap<u16, Option<VoxelShape>>,
    /// Kept for asking what a block is tagged with.
    tile_set: Option<TileSetDocument>,
}

impl std::fmt::Debug for VoxelGround {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VoxelGround")
            .field("edits", &self.edits.len())
            .field("blocks", &self.names.len())
            .finish_non_exhaustive()
    }
}

/// Where a ray met a voxel world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoxelWorldHit {
    /// The voxel it met, as world X, up, and Z.
    pub cell: [i32; 3],
    /// Which way out of that voxel it came from: one axis, plus or minus one.
    /// Zero when the ray started inside a solid voxel.
    pub normal: [i32; 3],
}

impl VoxelWorldHit {
    /// The empty voxel the ray was in just before, which is where a block
    /// built against the face goes.
    #[must_use]
    pub const fn before(self) -> [i32; 3] {
        [
            self.cell[0] + self.normal[0],
            self.cell[1] + self.normal[1],
            self.cell[2] + self.normal[2],
        ]
    }
}

impl VoxelGround {
    /// The ground a world component describes, with its edits on top.
    ///
    /// # Errors
    /// The same refusals that keep the world from drawing: a block set that is
    /// not bound, a generator naming a block the set does not define, and an
    /// edit naming one.
    pub fn of(
        component: &VoxelWorldComponent,
        tile_sets: Option<&TileSetBindings>,
    ) -> Result<Self, SceneExtractError> {
        let (palette, appearance, tile_set) = match component.blocks.as_deref() {
            Some(set) if !set.trim().is_empty() => {
                let tile_set = tile_sets
                    .and_then(|bindings| bindings.get(set))
                    .ok_or_else(|| SceneExtractError::UnboundTileSet(set.to_owned()))?;
                let (palette, appearance) = block_palette(set, tile_set)?;
                (palette, appearance, Some(tile_set.clone()))
            }
            _ => (
                Palette::of_materials(component.materials.iter().map(|material| material.voxel)),
                Appearance::Materials(component.materials.clone()),
                None,
            ),
        };
        let source = terrain_source(&component.generator, &palette)?;
        let (names, footing): (BTreeMap<u16, String>, BTreeMap<u16, Footing>) = match &appearance {
            Appearance::Blocks(blocks) => (
                blocks
                    .iter()
                    .map(|(voxel, name, _)| (*voxel, name.clone()))
                    .collect(),
                blocks
                    .iter()
                    .map(|(voxel, _, definition)| (*voxel, footing_of(definition)))
                    .collect(),
            ),
            Appearance::Materials(materials) => (
                materials
                    .iter()
                    .map(|material| (material.voxel, material.voxel.to_string()))
                    .collect(),
                materials
                    .iter()
                    .map(|material| {
                        (
                            material.voxel,
                            Footing {
                                supports: true,
                                walkable: true,
                                top: 1.0,
                            },
                        )
                    })
                    .collect(),
            ),
        };
        let collision = collision::shapes(&appearance);
        let mut ground = Self {
            terrain: terrain_for(TerrainKey {
                source,
                palette: palette.clone(),
                footing: footing.clone(),
            }),
            edits: BTreeMap::new(),
            edited_columns: HashMap::new(),
            names,
            footing,
            collision,
            tile_set,
        };
        for edit in &component.edits {
            let voxel = ground.id_of(&edit.block)?;
            ground
                .edits
                .insert(VoxelCoord::new(edit.at[0], edit.at[1], edit.at[2]), voxel);
            let highest = ground
                .edited_columns
                .entry((edit.at[0], edit.at[2]))
                .or_insert(edit.at[1]);
            *highest = (*highest).max(edit.at[1]);
        }
        Ok(ground)
    }

    /// The ground of the voxel world on `entity`, if it carries one.
    ///
    /// # Errors
    /// A world component that does not decode, or one [`VoxelGround::of`]
    /// refuses.
    pub fn of_entity(
        world: &sindri_core::World,
        components: &sindri_core::ComponentSchemaRegistry,
        entity: sindri_core::EntityId,
        tile_sets: Option<&TileSetBindings>,
    ) -> Result<Option<Self>, SceneExtractError> {
        components
            .get::<VoxelWorldComponent>(world, entity)?
            .map(|component| Self::of(&component, tile_sets))
            .transpose()
    }

    /// The stored voxel for a block, by name or number; air for `""`.
    /// Resolves a block in this world's palette without changing the terrain.
    ///
    /// # Errors
    /// A block or material this world does not define.
    pub fn id_of(&self, block: &VoxelBlock) -> Result<VoxelId, SceneExtractError> {
        if block.is_air() {
            return Ok(VoxelId::AIR);
        }
        match block {
            VoxelBlock::Named(name) => self
                .names
                .iter()
                .find(|(_, known)| *known == name)
                .map(|(voxel, _)| VoxelId::new(*voxel))
                .ok_or_else(|| SceneExtractError::UnknownVoxelBlock(name.clone())),
            VoxelBlock::Material(voxel) if self.names.contains_key(voxel) => {
                Ok(VoxelId::new(*voxel))
            }
            VoxelBlock::Material(voxel) => Err(SceneExtractError::MissingVoxelMaterial(*voxel)),
        }
    }

    /// Every edited voxel, by where it is.
    pub(super) const fn edit_voxels(&self) -> &BTreeMap<VoxelCoord, VoxelId> {
        &self.edits
    }

    /// The voxel at a cell, edits included.
    #[must_use]
    pub fn voxel(&self, at: [i32; 3]) -> VoxelId {
        let coord = VoxelCoord::new(at[0], at[1], at[2]);
        self.edits
            .get(&coord)
            .copied()
            .unwrap_or_else(|| self.terrain.voxel(coord))
    }

    /// What generation alone put at a cell.
    #[must_use]
    pub fn generated(&self, at: [i32; 3]) -> VoxelId {
        self.terrain.voxel(VoxelCoord::new(at[0], at[1], at[2]))
    }

    /// The block at a cell by name, or `""` for air.
    ///
    /// A world of numbered materials names each by its number.
    #[must_use]
    pub fn block(&self, at: [i32; 3]) -> String {
        let voxel = self.voxel(at);
        if voxel.is_air() {
            return String::new();
        }
        self.names.get(&voxel.value()).cloned().unwrap_or_default()
    }

    /// Whether the block at a cell carries a tag where its block set defines
    /// it. Always false in a world of numbered materials, which have none.
    #[must_use]
    pub fn tagged(&self, at: [i32; 3], tag: &str) -> bool {
        let name = self.block(at);
        self.tile_set
            .as_ref()
            .and_then(|set| set.tile(&name))
            .is_some_and(|block| block.tags.iter().any(|carried| carried == tag))
    }

    /// The edit list a component should carry once `at` holds `block`.
    ///
    /// An edit back to what was generated is not kept, so building and then
    /// taking a block away leaves the world exactly as it was written.
    ///
    /// # Errors
    /// A block the world does not define.
    pub fn edited(
        &self,
        edits: &[VoxelEdit],
        at: [i32; 3],
        block: &VoxelBlock,
    ) -> Result<Vec<VoxelEdit>, SceneExtractError> {
        let voxel = self.id_of(block)?;
        let mut kept: Vec<VoxelEdit> = edits.iter().filter(|edit| edit.at != at).cloned().collect();
        if voxel != self.generated(at) {
            kept.push(VoxelEdit {
                at,
                block: if voxel.is_air() {
                    VoxelBlock::air()
                } else {
                    block.clone()
                },
            });
        }
        Ok(kept)
    }

    fn footing(&self, voxel: VoxelId) -> Option<Footing> {
        (!voxel.is_air())
            .then(|| self.footing.get(&voxel.value()).copied())
            .flatten()
    }

    /// The highest block in a column that holds something up: its level, the
    /// height of its top, and whether a walker can stand on it. `None` where
    /// nothing in the column does.
    ///
    /// A column nobody has edited is the generator's, and its answer is
    /// remembered; an edited one is walked, from whichever is higher of its
    /// highest edit and the generator's ceiling.
    #[must_use]
    pub fn surface(&self, x: i32, z: i32) -> Option<(i32, f32, bool)> {
        let level = match self.edited_columns.get(&(x, z)) {
            None => self.terrain.top(x, z)?,
            Some(&highest_edit) => {
                let floor = self.terrain.floor(x, z);
                let mut y = self.terrain.ceiling(x, z).max(highest_edit);
                loop {
                    if y < floor {
                        return None;
                    }
                    if self
                        .footing(self.voxel([x, y, z]))
                        .is_some_and(|footing| footing.supports)
                    {
                        break y;
                    }
                    y -= 1;
                }
            }
        };
        let footing = self.footing(self.voxel([x, level, z]))?;
        #[allow(clippy::cast_precision_loss)]
        let height = level as f32 + footing.top;
        Some((level, height, footing.walkable))
    }

    /// Every column's surface between two corners, inclusive, as the surfaces
    /// a tile grid's navigation and placement read.
    ///
    /// A column is its X and Z: the grid's column and row. Its height is the
    /// top of its surface block.
    #[must_use]
    pub fn surfaces(&self, min: [i32; 2], max: [i32; 2]) -> TileSurfaces {
        let mut columns = Vec::new();
        for z in min[1]..=max[1] {
            for x in min[0]..=max[0] {
                if let Some((top, height, walkable)) = self.surface(x, z) {
                    columns.push((GridCoord3::new(x, z, top), height, walkable));
                }
            }
        }
        TileSurfaces::from_tops(columns)
    }
}

/// From a voxel world's own space, where a voxel is the unit cube from its
/// coordinate, to the space of the entity carrying it.
///
/// The identity on its own. Beside a grid of boxes the voxels become that
/// grid's cells: as wide, deep and tall as a cell, and centred on the cell's
/// column and row as every box on such a grid is, so a block a script names
/// by its grid cell is the block drawn there.
#[must_use]
pub fn voxel_space(
    world: &sindri_core::World,
    components: &sindri_core::ComponentSchemaRegistry,
    entity: sindri_core::EntityId,
) -> glam::Mat4 {
    let Some([across, into, up]) = components
        .get::<crate::TileGridComponent>(world, entity)
        .ok()
        .flatten()
        .and_then(|grid| grid.solid_cell())
    else {
        return glam::Mat4::IDENTITY;
    };
    glam::Mat4::from_scale(glam::Vec3::new(across, up, into))
        * glam::Mat4::from_translation(glam::Vec3::new(-0.5, 0.0, -0.5))
}

fn footing_of(definition: &TileDefinition) -> Footing {
    Footing {
        supports: definition.supports,
        walkable: definition.walkable,
        top: definition.bounds().top(),
    }
}

#[cfg(test)]
#[path = "voxel_ground_tests.rs"]
mod tests;
