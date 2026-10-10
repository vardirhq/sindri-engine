//! Scene ownership of 3D rigid bodies, without host-specific stepping policy.

use sindri_core::{ComponentSchemaRegistry, EntityId, World};
use sindri_physics::{
    Collider3d, ColliderShape3d, PhysicsEvent3d, PhysicsPose3d, PhysicsWorld3d, RigidBody3d,
    RigidBodyKind,
};
use std::{collections::BTreeMap, time::Duration};

use crate::voxel_collision3d::VoxelReach;
use crate::{
    Character3dComponent, CharacterMotions3d, CharacterRequests3d, Collider3dComponent,
    PhysicsSyncError, PhysicsWorld3dComponent, RigidBody3dComponent, SceneVoxelCollision3d,
    TileSetBindings,
};

#[cfg(test)]
mod authoring_tests;
#[cfg(test)]
mod hierarchy_tests;
#[cfg(test)]
mod pending_tests;
#[cfg(test)]
mod scale_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod voxel_tests;

const MOVED: f32 = 1.0e-4;

/// Runtime state beside the scene, never serialized into authored components.
/// Hosts must call this once per engine fixed step; game/editor wiring is separate.
#[derive(Clone)]
pub struct ScenePhysics3d {
    world: PhysicsWorld3d,
    registered: BTreeMap<EntityId, Authored>,
    agreed: BTreeMap<EntityId, PhysicsPose3d>,
    host_gravity: [f32; 3],
    events: Vec<PhysicsEvent3d>,
    /// Authored voxel colliders' static geometry. Lives and resets with the
    /// solver, because it owns bodies inside it.
    voxels: SceneVoxelCollision3d,
    characters: crate::characters3d::SceneCharacters3d,
}

#[derive(Clone, PartialEq)]
struct Authored {
    body: Option<RigidBody3d>,
    pieces: Vec<Collider3d>,
}

impl ScenePhysics3d {
    pub fn new(gravity: [f32; 3]) -> Result<Self, PhysicsSyncError> {
        Ok(Self {
            world: PhysicsWorld3d::new(gravity)?,
            registered: BTreeMap::new(),
            agreed: BTreeMap::new(),
            host_gravity: gravity,
            events: Vec::new(),
            voxels: SceneVoxelCollision3d::default(),
            characters: crate::characters3d::SceneCharacters3d::default(),
        })
    }
    pub const fn world(&self) -> &PhysicsWorld3d {
        &self.world
    }
    pub const fn world_mut(&mut self) -> &mut PhysicsWorld3d {
        &mut self.world
    }
    pub fn events(&self) -> &[PhysicsEvent3d] {
        &self.events
    }
    pub const fn for_scripts(&mut self) -> (&mut PhysicsWorld3d, &[PhysicsEvent3d]) {
        (&mut self.world, self.events.as_slice())
    }

    /// Pending world-space character movement for the next fixed step.
    pub fn character_requests(&mut self) -> &mut CharacterRequests3d {
        &mut self.characters.requests
    }
    /// The last applied character result, or none before synchronization.
    pub fn character_motion(&self, entity: EntityId) -> Option<&sindri_physics::CharacterMotion3d> {
        self.characters.motion(entity)
    }
    /// Disjoint runtime borrows for hosts with scene-owned character APIs.
    pub fn for_scripts_with_characters(
        &mut self,
    ) -> (
        &mut PhysicsWorld3d,
        &[PhysicsEvent3d],
        &mut CharacterRequests3d,
        CharacterMotions3d<'_>,
    ) {
        let (requests, motions) = self.characters.for_scripts();
        (&mut self.world, &self.events, requests, motions)
    }

    /// Validates the authored batch, reconciles lifecycle/edits, solves and writes
    /// XYZ/quaternion results to local transforms, retaining authored scale.
    /// Invalid scene inputs fail before runtime mutation. Structural edits rebuild
    /// their body; unchanged frames and transform teleports preserve velocity.
    pub fn step(
        &mut self,
        world: &mut World,
        components: &ComponentSchemaRegistry,
        delta: Duration,
    ) -> Result<(), PhysicsSyncError> {
        self.step_with_tile_sets(world, components, None, delta)
    }

    /// [`Self::step`], with the block sets authored voxel colliders name.
    /// Voxel geometry resident near each dynamic body or character is planned beside the
    /// body batch, and both validate before either changes the solver.
    pub fn step_with_tile_sets(
        &mut self,
        world: &mut World,
        components: &ComponentSchemaRegistry,
        tile_sets: Option<&TileSetBindings>,
        delta: Duration,
    ) -> Result<(), PhysicsSyncError> {
        if delta.is_zero() || !delta.as_secs_f32().is_finite() {
            return Err(PhysicsSyncError::BadStep(delta));
        }
        crate::physics3d::validate_dimensions(world)?;
        // A scene with nothing 3D in it, and a solver holding nothing, has
        // nothing to synchronize or step: a 2D game pays nothing for 3D.
        if self.world.is_empty() && self.registered.is_empty() && !authors_3d(world) {
            self.events.clear();
            self.characters.commit(BTreeMap::new());
            return Ok(());
        }
        let settings = components.query::<PhysicsWorld3dComponent>(world)?;
        if settings.len() > 1 {
            return Err(PhysicsSyncError::MultipleWorlds3d);
        }
        let gravity = settings
            .first()
            .map_or(self.host_gravity, |(_, value)| value.gravity);
        // Validate gravity before changing lifecycle or any existing body.
        if !gravity.into_iter().all(f32::is_finite) {
            return Err(sindri_physics::PhysicsError::NonFinite("gravity").into());
        }
        let plan = prepare(world, components, &self.world)?;
        let characters = crate::characters3d::SceneCharacters3d::plan(world, components)?;
        let mut reaches = self.reaches(world, &plan, gravity, delta.as_secs_f32());
        reaches.extend(self.characters.reaches(world, &characters)?);
        let voxels =
            self.voxels
                .plan_authored(world, components, tile_sets, &self.world, &reaches)?;
        self.world.set_gravity(gravity)?;
        self.synchronize(world, plan)?;
        self.characters.commit(characters);
        self.voxels.commit(&mut self.world, voxels)?;
        self.world.finish_synchronize();
        self.characters.hold(&mut self.world)?;
        self.events = self.world.step(delta)?;
        self.characters.apply(world, &mut self.world)?;
        self.write_back(world);
        Ok(())
    }

    fn synchronize(
        &mut self,
        world: &World,
        plan: BTreeMap<EntityId, Authored>,
    ) -> Result<(), PhysicsSyncError> {
        let gone: Vec<_> = self
            .registered
            .keys()
            .filter(|entity| !plan.contains_key(entity))
            .copied()
            .collect();
        for entity in gone {
            self.remove(entity);
        }
        for (entity, authored) in plan {
            let pose = pose_of(world, entity, authored.body);
            if self.registered.get(&entity) == Some(&authored) {
                if self.agreed.get(&entity).is_none_or(|old| moved(*old, pose)) {
                    self.characters.invalidate(entity);
                    self.world.move_to(entity, pose)?;
                    self.agreed.insert(entity, pose);
                }
                continue;
            }
            if self.world.contains(entity) {
                self.remove(entity);
            }
            let body = body_at(authored.body, pose);
            self.world.insert_body(entity, body, &authored.pieces)?;
            self.registered.insert(entity, authored);
            self.agreed.insert(entity, pose);
        }
        Ok(())
    }

    /// How far each planned dynamic body can be from where it starts by the
    /// end of this step: its farthest piece, plus twice the travel its current
    /// velocity and gravity allow, so a fast fall never outruns residency.
    fn reaches(
        &self,
        world: &World,
        plan: &BTreeMap<EntityId, Authored>,
        gravity: [f32; 3],
        seconds: f32,
    ) -> Vec<VoxelReach> {
        let pull = gravity.iter().map(|axis| axis * axis).sum::<f32>().sqrt();
        plan.iter()
            .filter_map(|(&entity, authored)| {
                let body = authored.body?;
                if body.kind != RigidBodyKind::Dynamic {
                    return None;
                }
                let velocity = self
                    .world
                    .linear_velocity(entity)
                    .unwrap_or(body.linear_velocity);
                let speed = velocity.iter().map(|axis| axis * axis).sum::<f32>().sqrt();
                let size = authored.pieces.iter().map(extent).fold(0.0, f32::max);
                Some(VoxelReach {
                    position: pose_of(world, entity, Some(body)).position,
                    reach: size + 2.0 * (speed * seconds + pull * seconds * seconds),
                })
            })
            .collect()
    }

    fn remove(&mut self, entity: EntityId) {
        self.characters.invalidate(entity);
        self.world.remove(entity);
        self.registered.remove(&entity);
        self.agreed.remove(&entity);
    }

    fn write_back(&mut self, world: &mut World) {
        // Parents must be written first even when reparented under a newer ID.
        // Snapshot world transforms before writes to avoid carrying child scale twice.
        let mut writes: Vec<_> = self
            .registered
            .iter()
            .filter_map(|(&entity, authored)| {
                if body_at(authored.body, PhysicsPose3d::default()).kind == RigidBodyKind::Static {
                    return None;
                }
                let pose = self.world.pose(entity).ok()?;
                let mut placed = world.world_transform(entity).unwrap_or_default();
                placed.position = pose.position;
                placed.rotation = pose.rotation;
                Some((depth(world, entity), entity, placed))
            })
            .collect();
        writes.sort_unstable_by_key(|(depth, entity, _)| (*depth, *entity));
        for (_, entity, placed) in writes {
            let transform = world.local_for_world(entity, placed);
            if let Some(data) = world.get_mut(entity) {
                data.transform_3d = Some(transform);
            }
            if let Some(held) = world.world_transform(entity) {
                self.agreed.insert(
                    entity,
                    PhysicsPose3d {
                        position: held.position,
                        rotation: held.rotation,
                    },
                );
            }
        }
    }
}

fn prepare(
    world: &World,
    components: &ComponentSchemaRegistry,
    physics: &PhysicsWorld3d,
) -> Result<BTreeMap<EntityId, Authored>, PhysicsSyncError> {
    let mut plan = BTreeMap::new();
    for (entity, collider) in components.query::<Collider3dComponent>(world)? {
        let character = components.get::<Character3dComponent>(world, entity)?;
        let mut body = components
            .get::<RigidBody3dComponent>(world, entity)?
            .map(|value| value.0);
        if character.is_some() && body.is_none() {
            body = Some(RigidBody3d {
                kind: RigidBodyKind::KinematicVelocity,
                ..RigidBody3d::default()
            });
        }
        let pose = pose_of(world, entity, body);
        let placed = body_at(body, pose);
        if placed.kind != RigidBodyKind::Static
            && world
                .get(entity)
                .and_then(|data| data.transform_3d)
                .is_some_and(|transform| transform.z_locked)
        {
            return Err(PhysicsSyncError::LockedDepth3d(entity));
        }
        physics.validate_insertion(entity, placed, &collider.0)?;
        let scale = world.world_transform(entity).unwrap_or_default().scale;
        let pieces = collider
            .scaled(scale)
            .map_err(|error| PhysicsSyncError::ColliderScale3d(entity, error))?;
        physics.validate_insertion(entity, placed, &pieces)?;
        plan.insert(entity, Authored { body, pieces });
    }
    Ok(plan)
}

/// The farthest any point of a piece is from its body's origin.
pub(crate) fn extent(piece: &Collider3d) -> f32 {
    let offset = piece
        .offset
        .iter()
        .map(|axis| axis * axis)
        .sum::<f32>()
        .sqrt();
    offset
        + match piece.shape {
            ColliderShape3d::Box { half_extents } => half_extents
                .iter()
                .map(|axis| axis * axis)
                .sum::<f32>()
                .sqrt(),
            ColliderShape3d::Sphere { radius } => radius,
            ColliderShape3d::Capsule {
                half_height,
                radius,
            } => half_height + radius,
        }
}

/// Whether anything in `world` carries 3D physics: every 3D physics
/// component is named under `sindri.physics3d.`.
fn authors_3d(world: &World) -> bool {
    world.entities().any(|(_, data)| {
        data.components
            .keys()
            .any(|name| name.starts_with("sindri.physics3d."))
    })
}

fn body_at(body: Option<RigidBody3d>, pose: PhysicsPose3d) -> RigidBody3d {
    RigidBody3d {
        position: pose.position,
        rotation: pose.rotation,
        ..body.unwrap_or(RigidBody3d {
            kind: RigidBodyKind::Static,
            ..RigidBody3d::default()
        })
    }
}
pub(crate) fn pose_of(world: &World, entity: EntityId, body: Option<RigidBody3d>) -> PhysicsPose3d {
    world.world_transform(entity).map_or_else(
        || {
            body.map_or_else(PhysicsPose3d::default, |body| PhysicsPose3d {
                position: body.position,
                rotation: body.rotation,
            })
        },
        |transform| PhysicsPose3d {
            position: transform.position,
            rotation: transform.rotation,
        },
    )
}
fn moved(old: PhysicsPose3d, new: PhysicsPose3d) -> bool {
    old.position.into_iter().zip(new.position).any(|(a,b)| (a-b).abs() > MOVED)
        // q and -q represent the same orientation.
        || [1.0, -1.0].into_iter().all(|sign| old.rotation.into_iter().zip(new.rotation)
            .any(|(a,b)| (a-sign*b).abs() > MOVED))
}
fn depth(world: &World, entity: EntityId) -> usize {
    let mut current = world.get(entity).and_then(|data| data.parent);
    let mut depth = 0;
    while let Some(parent) = current {
        depth += 1;
        if depth >= 256 {
            break;
        }
        current = world.get(parent).and_then(|data| data.parent);
    }
    depth
}
