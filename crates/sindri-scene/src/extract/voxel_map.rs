//! A voxel world drawn as a map from straight above.
//!
//! The world is the same one a `blocks` view meshes: the same generator, the
//! same block set, the same edits, and the same [`VoxelGround`] a script
//! reads. Only the drawing differs. Each column becomes one unit square, drawn
//! with the top face of its highest block that holds anything up -- grass,
//! sand, water, a tree's leaves -- so a top-down game gets coastlines, rivers,
//! biomes and forests from a generator that was written for blocks.
//!
//! A flat picture of a world with height in it loses the height, so the map
//! puts it back as light. Each square is lit as if by a sun low in the north
//! west: a column standing above its north-western neighbour faces the sun and
//! is lighter, one below it is in shadow, and a cliff becomes a hard dark
//! line. Higher ground is a little paler overall, and water darkens with its
//! depth, so a shelf and a trench read as different blues.
//!
//! Column X runs across the map and row Z runs down it, which is the way a
//! tilemap's rows run and the way `Grid.block` already names a voxel world's
//! cells: column, row, level.
//!
//! Only what the camera can see is drawn. Columns are worked out sixteen by
//! sixteen and remembered until the world changes, so a world much larger than
//! any screen costs what the screen shows.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use glam::{Mat4, Vec3, Vec4Swizzles};
use sindri_core::{EntityId, World};
use sindri_render::{SpriteInstance, TextureId, TransparentOrder, UvRect};
use sindri_voxel::{SECTION_EDGE, VoxelCoord, VoxelFace};

use crate::{
    TextureBindings, TileSetBindings, VoxelGeneratorDocument, VoxelTextureSource, VoxelView,
    VoxelWorldComponent,
};

use super::camera::ResolvedCamera;
use super::camera::view::camera_distance;
use super::sprite::{DrawSpace, SpriteBatches, SpriteDraw};
use super::voxel_appearance::{Resolved, resolve_appearance};
use super::voxel_ground::VoxelGround;
use super::voxel_world::definition;
use super::{SceneExtractError, SceneExtractor, transform_matrix};

/// How many columns a side of a remembered chunk holds.
const CHUNK: i32 = SECTION_EDGE;
/// The most chunks a side of the view may span. A camera pulled back far
/// enough to see more would be asking for a map of a continent every frame.
const MOST_CHUNKS_ACROSS: i32 = 48;
/// How many chunks one map remembers before starting again.
const REMEMBERED_CHUNKS: usize = 4_096;
/// How much lighter or darker one level of difference from the north-western
/// neighbour makes a square, and the most it may.
const SLOPE_LIGHT: f32 = 0.09;
const MOST_SLOPE_LIGHT: f32 = 0.28;
/// How much paler each level above the reference height is, and the most.
const HEIGHT_LIGHT: f32 = 0.006;
const MOST_HEIGHT_LIGHT: f32 = 0.18;
/// How much darker each level of water depth is, how deep is looked for, and
/// the darkest it gets.
const DEPTH_DARK: f32 = 0.07;
const DEEPEST: i32 = 8;

/// One column's square, ready to draw.
#[derive(Clone, Copy, Debug)]
struct MapSquare {
    column: i32,
    row: i32,
    texture: TextureId,
    rect: UvRect,
    tint: [f32; 4],
}

/// One map's remembered squares, and what they were worked out from.
struct MapWorld {
    revision: Option<u64>,
    textures: u64,
    tile_sets: u64,
    ground: VoxelGround,
    resolved: Resolved,
    reference: i32,
    chunks: HashMap<(i32, i32), Rc<Vec<MapSquare>>>,
}

#[derive(Default)]
pub(super) struct VoxelMapCache(RefCell<BTreeMap<EntityId, MapWorld>>);

impl Clone for VoxelMapCache {
    fn clone(&self) -> Self {
        // Squares are derived state, as a voxel world's meshes are.
        Self::default()
    }
}

impl std::fmt::Debug for VoxelMapCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VoxelMapCache")
            .field("maps", &self.0.borrow().len())
            .finish()
    }
}

impl SceneExtractor {
    /// Every voxel world viewed as a map, into the same sprite batches as
    /// tilemaps and loose sprites, so they sort and share draws with them.
    pub(super) fn push_voxel_maps(
        &self,
        world: &World,
        camera: Option<ResolvedCamera>,
        textures: &TextureBindings,
        tile_sets: Option<&TileSetBindings>,
        batches: &mut SpriteBatches,
    ) -> Result<(), SceneExtractError> {
        let maps: Vec<_> = self
            .components
            .query::<VoxelWorldComponent>(world)?
            .into_iter()
            .filter(|(entity, component)| component.view == VoxelView::Map && world.is_active(*entity))
            .collect();
        let mut cache = self.voxel_maps.0.borrow_mut();
        cache.retain(|entity, _| maps.iter().any(|(kept, _)| kept == entity));
        if maps.is_empty() {
            return Ok(());
        }
        let camera = camera.ok_or(SceneExtractError::MissingWorldCamera)?;
        for (entity, component) in maps {
            let refreshed = refresh(&mut cache, world, entity, &component, textures, tile_sets);
            match refreshed {
                Ok(()) => {}
                Err(error) if self.tolerant() => {
                    self.record(entity, "sindri.voxel_world", &error);
                    if !cache.contains_key(&entity) {
                        continue;
                    }
                }
                Err(error) => return Err(error),
            }
            let map = cache.get_mut(&entity).expect("refreshed or kept above");
            let root = transform_matrix(world.world_transform(entity).unwrap_or_default());
            push_map(map, root, camera, component.layer, batches)?;
        }
        Ok(())
    }
}

/// Keeps a map's remembered squares only while what they came from is
/// unchanged: the entity, its textures and its block set.
fn refresh(
    cache: &mut BTreeMap<EntityId, MapWorld>,
    world: &World,
    entity: EntityId,
    component: &VoxelWorldComponent,
    textures: &TextureBindings,
    tile_sets: Option<&TileSetBindings>,
) -> Result<(), SceneExtractError> {
    let revision = world.revision(entity);
    let textures_generation = textures.generation();
    let tile_sets_generation = tile_sets.map_or(0, TileSetBindings::generation);
    let fresh = cache.get(&entity).is_some_and(|map| {
        map.revision == revision
            && map.textures == textures_generation
            && map.tile_sets == tile_sets_generation
    });
    if fresh {
        return Ok(());
    }
    let definition = definition(component, tile_sets)?;
    let resolved = resolve_appearance(&definition.appearance, textures)?
        .with_seed(component.variant_seed);
    let ground = VoxelGround::of(component, tile_sets)?;
    cache.insert(
        entity,
        MapWorld {
            revision,
            textures: textures_generation,
            tile_sets: tile_sets_generation,
            ground,
            resolved,
            reference: reference_height(&component.generator),
            chunks: HashMap::new(),
        },
    );
    Ok(())
}

/// The height a map's light is measured from: the sea, where there is one.
const fn reference_height(generator: &VoxelGeneratorDocument) -> i32 {
    match generator {
        VoxelGeneratorDocument::LayeredTerrain { base_height, .. } => *base_height,
        VoxelGeneratorDocument::NaturalTerrain(natural) => natural.sea_level,
    }
}

fn push_map(
    map: &mut MapWorld,
    root: Mat4,
    camera: ResolvedCamera,
    layer: i32,
    batches: &mut SpriteBatches,
) -> Result<(), SceneExtractError> {
    let Some((min, max)) = visible_columns(root, camera) else {
        return Ok(());
    };
    let distance = camera_distance(camera.view, root.w_axis.truncate());
    let mut index = 0_u32;
    for chunk_row in min[1].div_euclid(CHUNK)..=max[1].div_euclid(CHUNK) {
        for chunk_column in min[0].div_euclid(CHUNK)..=max[0].div_euclid(CHUNK) {
            let squares = chunk(map, chunk_column, chunk_row);
            for square in squares.iter() {
                if square.column < min[0]
                    || square.column > max[0]
                    || square.row < min[1]
                    || square.row > max[1]
                {
                    continue;
                }
                #[allow(clippy::cast_precision_loss)]
                let local = Mat4::from_translation(Vec3::new(
                    square.column as f32 + 0.5,
                    -(square.row as f32) - 0.5,
                    0.0,
                ));
                batches.push(SpriteDraw {
                    clip: None,
                    space: DrawSpace::World,
                    texture: square.texture,
                    order: TransparentOrder::new(layer, distance, index)?,
                    sprite: SpriteInstance::new(root * local, square.tint)
                        .with_uv_rect(square.rect),
                });
                index = index.saturating_add(1);
            }
        }
    }
    Ok(())
}

/// The squares of one chunk, worked out the first time it is seen.
fn chunk(map: &mut MapWorld, chunk_column: i32, chunk_row: i32) -> Rc<Vec<MapSquare>> {
    if let Some(known) = map.chunks.get(&(chunk_column, chunk_row)) {
        return Rc::clone(known);
    }
    let mut squares = Vec::with_capacity((CHUNK * CHUNK) as usize);
    for row in chunk_row * CHUNK..(chunk_row + 1) * CHUNK {
        for column in chunk_column * CHUNK..(chunk_column + 1) * CHUNK {
            if let Some(square) = square(map, column, row) {
                squares.push(square);
            }
        }
    }
    if map.chunks.len() >= REMEMBERED_CHUNKS {
        map.chunks.clear();
    }
    let squares = Rc::new(squares);
    map.chunks
        .insert((chunk_column, chunk_row), Rc::clone(&squares));
    squares
}

/// One column as the map draws it, or nothing where nothing stands.
fn square(map: &MapWorld, column: i32, row: i32) -> Option<MapSquare> {
    let (level, height, _) = map.ground.surface(column, row)?;
    let voxel = map.ground.voxel([column, level, row]);
    let face = map.resolved.texture_at(
        voxel,
        VoxelFace::Top,
        VoxelCoord::new(column, level, row),
        false,
    );
    let shade = match map.ground.surface(column - 1, row - 1) {
        Some((_, beside, _)) => ((height - beside) * SLOPE_LIGHT).clamp(-MOST_SLOPE_LIGHT, MOST_SLOPE_LIGHT),
        None => 0.0,
    };
    #[allow(clippy::cast_precision_loss)]
    let raised = ((level - map.reference) as f32 * HEIGHT_LIGHT).clamp(-MOST_HEIGHT_LIGHT, MOST_HEIGHT_LIGHT);
    let depth = depth_below(map, column, level, row);
    #[allow(clippy::cast_precision_loss)]
    let light = (1.0 + shade + raised - depth as f32 * DEPTH_DARK).max(0.2);
    Some(MapSquare {
        column,
        row,
        texture: face.texture,
        rect: face.uv,
        tint: [light, light, light, 1.0],
    })
}

/// How many more of the same block lie under a column's top: a water
/// column's depth, up to `DEEPEST`. Solid ground repeats too, so only blocks
/// a walker cannot stand on -- water, lava -- are measured.
fn depth_below(map: &MapWorld, column: i32, level: i32, row: i32) -> i32 {
    let Some((_, _, walkable)) = map.ground.surface(column, row) else {
        return 0;
    };
    if walkable {
        return 0;
    }
    let top = map.ground.voxel([column, level, row]);
    (1..=DEEPEST)
        .take_while(|below| map.ground.voxel([column, level - below, row]) == top)
        .count()
        .try_into()
        .unwrap_or(DEEPEST)
}

/// The columns and rows the camera can see of a map, inclusive, padded by
/// one so a square half on screen is drawn.
///
/// Where the four corners of the picture meet the map's plane, carried into
/// the map's own space. A camera looking along the plane meets it nowhere,
/// and draws none of it.
fn visible_columns(root: Mat4, camera: ResolvedCamera) -> Option<([i32; 2], [i32; 2])> {
    let to_world = camera.view_projection.inverse();
    let to_map = root.inverse();
    let mut low = [f32::INFINITY; 2];
    let mut high = [f32::NEG_INFINITY; 2];
    for [x, y] in [[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]] {
        let near = to_map * to_world * glam::Vec4::new(x, y, 0.0, 1.0);
        let far = to_map * to_world * glam::Vec4::new(x, y, 1.0, 1.0);
        let (near, far) = (near.xyz() / near.w, far.xyz() / far.w);
        let along = far.z - near.z;
        if along.abs() < f32::EPSILON {
            return None;
        }
        let point = near + (far - near) * (-near.z / along);
        low = [low[0].min(point.x), low[1].min(-point.y)];
        high = [high[0].max(point.x), high[1].max(-point.y)];
    }
    if !(low[0].is_finite() && low[1].is_finite() && high[0].is_finite() && high[1].is_finite()) {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    let mut min = [low[0].floor() as i32 - 1, low[1].floor() as i32 - 1];
    #[allow(clippy::cast_possible_truncation)]
    let mut max = [high[0].floor() as i32 + 1, high[1].floor() as i32 + 1];
    let most = MOST_CHUNKS_ACROSS * CHUNK;
    for axis in 0..2 {
        if max[axis] - min[axis] > most {
            let middle = (min[axis] + max[axis]) / 2;
            min[axis] = middle - most / 2;
            max[axis] = middle + most / 2;
        }
    }
    Some((min, max))
}

#[cfg(test)]
#[path = "voxel_map_tests.rs"]
mod tests;
