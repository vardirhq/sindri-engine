//! What 2D physics reads from the world each step, decoded once per change.
//!
//! Physics synchronizes every step, because scripts spawn and move things,
//! and synchronizing reads every collider, body, material and one-way policy
//! in the scene. Read through [`Decoded`], each is decoded again only when
//! its entity changed, so a level's static geometry costs nothing to read
//! after the first step. A tilemap's merged collision rectangles are kept the
//! same way, keyed by the entity's revision and the scale it is drawn at.

use std::collections::BTreeMap;

use sindri_core::{ComponentSchemaRegistry, Decoded, EntityId, Transform3D, World};
use sindri_physics::Collider2d;

use crate::components::TilemapComponent;
use crate::physics::{
    Collider2dComponent, OneWay2dComponent, PhysicsWorld2dComponent, RigidBody2dComponent,
};
use crate::physics_material::PhysicsMaterial2dComponent;
use crate::physics_sync::PhysicsSyncError;
use crate::tilemap_collision::{TilemapCollider2dComponent, TilemapCollisionError};

/// A tilemap's collision pieces, with what they were worked out from.
type TilePieces = (u64, [u32; 2], Vec<Collider2d>);

#[derive(Clone, Default)]
pub(crate) struct PhysicsReads {
    colliders: Decoded<Collider2dComponent>,
    tilemap_colliders: Decoded<TilemapCollider2dComponent>,
    tilemaps: Decoded<TilemapComponent>,
    pub(crate) bodies: Decoded<RigidBody2dComponent>,
    pub(crate) materials: Decoded<PhysicsMaterial2dComponent>,
    pub(crate) one_ways: Decoded<OneWay2dComponent>,
    pub(crate) settings: Decoded<PhysicsWorld2dComponent>,
    tile_pieces: BTreeMap<EntityId, TilePieces>,
}

impl PhysicsReads {
    /// Every entity's collision pieces, as `physics_sync::collider_pieces`
    /// works them out, read through the caches.
    pub(crate) fn collider_pieces(
        &mut self,
        world: &World,
        components: &ComponentSchemaRegistry,
    ) -> Result<BTreeMap<EntityId, Vec<Collider2d>>, PhysicsSyncError> {
        let mut pieces: BTreeMap<EntityId, Vec<Collider2d>> = self
            .colliders
            .query(components, world)?
            .into_iter()
            .map(|(entity, collider)| (entity, collider.0.clone()))
            .collect();
        let tilemap_colliders = self.tilemap_colliders.query(components, world)?;
        for (entity, collider) in tilemap_colliders {
            let scale = world
                .world_transform(entity)
                .map_or([1.0, 1.0], Transform3D::scale_2d);
            let revision = world.revision(entity).unwrap_or_default();
            let key = [scale[0].to_bits(), scale[1].to_bits()];
            let tiles = match self.tile_pieces.get(&entity) {
                Some((held, held_scale, tiles)) if *held == revision && *held_scale == key => {
                    tiles.clone()
                }
                _ => {
                    let tilemap = self
                        .tilemaps
                        .get(components, world, entity)?
                        .ok_or(TilemapCollisionError::NoTilemap)?;
                    let tiles = collider.pieces(tilemap, scale)?;
                    self.tile_pieces
                        .insert(entity, (revision, key, tiles.clone()));
                    tiles
                }
            };
            if !tiles.is_empty() {
                pieces.entry(entity).or_default().extend(tiles);
            }
        }
        Ok(pieces)
    }
}
