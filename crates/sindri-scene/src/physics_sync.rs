//! Keeping a scene and the physics world in step.
//!
//! `sindri-physics` owns bodies, colliders, and the step; `sindri-core` owns
//! entities and transforms. Nothing joined them, so a scene could author a
//! collider and no collision would ever happen — the audit's "no exercised game
//! integration". This is that join, and it is the same shape as
//! [`crate::SpriteAnimations`]: runtime state beside the world, derived from
//! authored components, never serialized.
//!
//! What it does *not* do is decide when. `docs/physics.md` fixes the order
//! within one fixed update — author state in, step, results out, events
//! published — and the host decides how often that happens. A render frame
//! never steps physics.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use sindri_core::{ComponentRegistryError, ComponentSchemaRegistry, EntityId, Transform3D, World};
use sindri_physics::{
    PhysicsError, PhysicsEvent2d, PhysicsPose2d, PhysicsWorld2d, RigidBody2d, RigidBodyKind,
};
use thiserror::Error;

use crate::components::TilemapComponent;
use crate::physics::{
    Collider2dComponent, OneWay2dComponent, PhysicsWorld2dComponent, RigidBody2dComponent,
};
use crate::tilemap_collision::{TilemapCollider2dComponent, TilemapCollisionError};

#[cfg(test)]
mod tests;

/// How far a transform may drift from where physics last put it, in units or
/// radians, before it counts as moved by something else. Above the rounding a
/// write-back can introduce, far below any move a script means.
const MOVED: f32 = 1.0e-4;

#[derive(Debug, Error)]
pub enum PhysicsSyncError {
    #[error("joint entity {0:?} has more than one joint component")]
    ConflictingJointComponents(EntityId),
    #[error("a fixed step cannot be {0:?} long")]
    BadStep(Duration),
    #[error(transparent)]
    Registry(#[from] ComponentRegistryError),
    #[error(transparent)]
    Physics(#[from] PhysicsError),
    #[error(transparent)]
    Tilemap(#[from] TilemapCollisionError),
    #[error(transparent)]
    Material(#[from] crate::PhysicsMaterialError),
}

/// The physics world a scene's authored bodies and colliders drive.
///
/// Held beside [`World`] rather than in it, for the reason every derived thing
/// is: a scene saved mid-run has to be the scene that was opened, and a body's
/// solver state is not something an author wrote.
pub struct ScenePhysics2d {
    world: PhysicsWorld2d,
    /// Which entities are in the physics world, and what they were authored as.
    ///
    /// Kept so a re-registration can be told from an unchanged one: rebuilding
    /// every body every step would throw away the velocity the simulation just
    /// computed, which is the whole state physics owns.
    registered: BTreeMap<EntityId, Authored>,
    /// Where each body's transform said it was when physics last agreed with
    /// it: at registration, and after each write-back.
    ///
    /// A transform that no longer says this was moved by something other than
    /// physics, a script respawning the player or clamping it to the screen,
    /// and the body is moved to match rather than the move being overwritten
    /// by the next write-back.
    agreed: BTreeMap<EntityId, PhysicsPose2d>,
    events: Vec<PhysicsEvent2d>,
    /// What the host asked for, which a scene naming no gravity of its own
    /// keeps.
    host_gravity: [f32; 2],
    materials: crate::PhysicsMaterialSources,
    joints: crate::physics_joints::SceneJoints2d,
}

/// What a scene said about one entity, as far as physics is concerned.
///
/// Compared rather than the whole component: structural changes rebuild the
/// body, while live control and coefficient edits preserve its solver state.
/// Fields physics does not read cannot cause a rebuild.
#[derive(Clone, PartialEq)]
struct Authored {
    body: Option<RigidBody2d>,
    collider: Vec<sindri_physics::Collider2d>,
    kind: RigidBodyKind,
    one_way: Option<sindri_physics::OneWay2d>,
}

impl ScenePhysics2d {
    /// A physics world under `gravity`, in scene units per second squared.
    ///
    /// Gravity belongs to the host rather than to a component, because it is a
    /// fact about the world and not about an entity. A top-down game passes
    /// zero, which is why [`Self::top_down`] exists and says so by name.
    pub fn new(gravity: [f32; 2]) -> Result<Self, PhysicsSyncError> {
        Ok(Self {
            world: PhysicsWorld2d::new(gravity)?,
            registered: BTreeMap::new(),
            agreed: BTreeMap::new(),
            events: Vec::new(),
            host_gravity: gravity,
            materials: crate::PhysicsMaterialSources::default(),
            joints: crate::physics_joints::SceneJoints2d::default(),
        })
    }

    /// A world with no gravity, for a game seen from above.
    pub fn top_down() -> Result<Self, PhysicsSyncError> {
        Self::new([0.0, 0.0])
    }

    /// The physics world itself, for the operations gameplay drives directly:
    /// velocity, impulses, kinematic targets.
    pub const fn world(&self) -> &PhysicsWorld2d {
        &self.world
    }

    pub const fn world_mut(&mut self) -> &mut PhysicsWorld2d {
        &mut self.world
    }

    /// Replaces resolved material assets. Changes reach existing colliders at
    /// the next fixed step without replacing their bodies.
    pub fn set_materials(&mut self, materials: crate::PhysicsMaterialSources) {
        self.materials = materials;
    }

    /// What collided during the last step, in the order the backend reported.
    pub fn events(&self) -> &[PhysicsEvent2d] {
        &self.events
    }

    /// The world to drive and the events to read, together.
    ///
    /// Both at once because a script pass needs both and they are disjoint
    /// halves of this: asking for them separately would be one mutable and one
    /// shared borrow of the same value, which is a thing a caller should not
    /// have to work around.
    pub const fn for_scripts(&mut self) -> (&mut PhysicsWorld2d, &[PhysicsEvent2d]) {
        (&mut self.world, self.events.as_slice())
    }

    /// One fixed update: author state in, step, results out.
    ///
    /// The order is `docs/physics.md`'s and is not a detail — a body registered
    /// after the step would miss it, and a transform written back before the
    /// step would be the previous frame's.
    pub fn step(
        &mut self,
        world: &mut World,
        components: &ComponentSchemaRegistry,
        delta: Duration,
    ) -> Result<(), PhysicsSyncError> {
        if delta.is_zero() || !delta.as_secs_f32().is_finite() {
            return Err(PhysicsSyncError::BadStep(delta));
        }
        let gravity = components
            .query::<PhysicsWorld2dComponent>(world)?
            .first()
            .map_or(self.host_gravity, |(_, settings)| settings.gravity);
        // Exact on purpose: this asks whether the authored value changed, and
        // setting the same value again would be harmless anyway.
        #[allow(clippy::float_cmp)]
        let changed = gravity != self.world.gravity();
        if changed {
            self.world.set_gravity(gravity)?;
        }
        self.synchronize(world, components)?;
        self.joints
            .synchronize(world, components, &mut self.world)?;
        // Scripts may set velocity or connect two freshly spawned bodies before
        // either backend body exists. Synchronization above gave every authored
        // body its chance to materialize; resolve those deferred operations now
        // so the coming fixed step sees the intended chain rather than one frame
        // of independent pieces.
        self.world.finish_synchronize()?;
        self.events = self.world.step(delta)?;
        self.write_back(world);
        Ok(())
    }

    /// Registers what is newly authored, updates what changed, and forgets what
    /// is gone.
    ///
    /// Called every step rather than once, because entities are spawned and
    /// despawned by scripts now: a physics world built at load would know
    /// nothing about a bullet.
    fn synchronize(
        &mut self,
        world: &World,
        components: &ComponentSchemaRegistry,
    ) -> Result<(), PhysicsSyncError> {
        let mut live = BTreeSet::new();
        for (entity, mut collider) in collider_pieces(world, components)? {
            if let Some(material) =
                components.get::<crate::PhysicsMaterial2dComponent>(world, entity)?
            {
                self.materials.apply(&material, &mut collider)?;
            }
            live.insert(entity);
            let body = components
                .get::<RigidBody2dComponent>(world, entity)?
                .map(|authored| authored.0);
            let authored = Authored {
                body,
                collider,
                one_way: components
                    .get::<OneWay2dComponent>(world, entity)?
                    .map(|policy| policy.0),
                kind: body.map_or(RigidBodyKind::Static, |body| body.kind),
            };
            if self.update_registered(world, entity, &authored)? {
                continue;
            }
            // The authored pose comes from the entity's transform, which is the
            // one place a position is written down. A body's own pose field is
            // where physics puts the answer back, not a second authored truth.
            let pose = pose_of(world, entity, authored.body);
            let outcome = match authored.body {
                Some(body) => {
                    self.world
                        .insert_body(entity, RigidBody2d { pose, ..body }, &authored.collider)
                }
                None => self
                    .world
                    .insert_static_collider(entity, pose, &authored.collider),
            };
            match outcome {
                Ok(()) => {
                    self.world.set_one_way(entity, authored.one_way)?;
                    self.registered.insert(entity, authored);
                    self.agreed.insert(entity, pose);
                }
                // A body the backend refuses is reported once and skipped, not
                // retried every step: an invalid collider would otherwise fill
                // a console sixty times a second with the same line.
                Err(error) => {
                    self.registered.remove(&entity);
                    return Err(error.into());
                }
            }
        }

        // Anything that stopped being authored — despawned, switched off, or
        // had its collider removed — leaves the physics world with it. A body
        // outliving its entity would collide with things on behalf of nothing.
        let gone: Vec<EntityId> = self
            .registered
            .keys()
            .filter(|entity| !live.contains(entity))
            .copied()
            .collect();
        for entity in gone {
            self.world.remove(entity);
            self.registered.remove(&entity);
            self.agreed.remove(&entity);
        }
        Ok(())
    }

    /// Support policy, CCD and coefficient edits preserve solver state.
    fn update_registered(
        &mut self,
        world: &World,
        entity: EntityId,
        authored: &Authored,
    ) -> Result<bool, PhysicsSyncError> {
        let Some(previous) = self.registered.get(&entity) else {
            return Ok(false);
        };
        if previous == authored {
            self.follow_transform(world, entity, authored.body)?;
            return Ok(true);
        }
        let mut comparable = previous.clone();
        comparable.one_way = authored.one_way;
        for (before, after) in comparable.collider.iter_mut().zip(&authored.collider) {
            before.friction = after.friction;
            before.restitution = after.restitution;
        }
        if let (Some(before), Some(after)) = (&mut comparable.body, authored.body)
            && authored.kind == RigidBodyKind::Dynamic
        {
            before.continuous_collision = after.continuous_collision;
        }
        if comparable == *authored {
            if previous.collider != authored.collider {
                let materials: Vec<_> = authored
                    .collider
                    .iter()
                    .map(|piece| sindri_physics::PhysicsMaterial {
                        friction: piece.friction,
                        restitution: piece.restitution,
                    })
                    .collect();
                self.world.set_materials(entity, &materials)?;
            }
            self.world.set_one_way(entity, authored.one_way)?;
            if authored.kind == RigidBodyKind::Dynamic {
                self.world.set_continuous_collision(
                    entity,
                    authored.body.is_some_and(|body| body.continuous_collision),
                )?;
            }
            self.registered.insert(entity, authored.clone());
            self.follow_transform(world, entity, authored.body)?;
            return Ok(true);
        }
        self.world.remove(entity);
        Ok(false)
    }

    /// Moves a registered body to where its transform now is, when something
    /// other than physics moved the transform since they last agreed.
    fn follow_transform(
        &mut self,
        world: &World,
        entity: EntityId,
        body: Option<RigidBody2d>,
    ) -> Result<(), PhysicsSyncError> {
        let now = pose_of(world, entity, body);
        let moved = self.agreed.get(&entity).is_none_or(|agreed| {
            (agreed.position[0] - now.position[0]).abs() > MOVED
                || (agreed.position[1] - now.position[1]).abs() > MOVED
                || (agreed.rotation - now.rotation).abs() > MOVED
        });
        if moved {
            self.world.move_to(entity, now)?;
            self.agreed.insert(entity, now);
        }
        Ok(())
    }

    /// Puts what physics decided back into the transforms the renderer reads.
    ///
    /// Only for bodies physics owns the position of. A static body's position
    /// is the author's, and writing it back would fight whoever moved it.
    fn write_back(&mut self, world: &mut World) {
        for (entity, authored) in &self.registered {
            if !matches!(
                authored.kind,
                RigidBodyKind::Dynamic | RigidBodyKind::KinematicVelocity
            ) {
                continue;
            }
            let Ok(pose) = self.world.pose(*entity) else {
                continue;
            };
            // Physics answers in the world; a child body stores its place
            // relative to its parent, so the answer is carried back into it.
            let Some(mut placed) = world.world_transform(*entity) else {
                continue;
            };
            // X, Y and the rotation about Z. The Z position and the 3D scale are
            // the author's and are preserved, which is what `docs/2d-model.md`
            // means by one transform: a 2D entity keeps to a plane rather than
            // having a transform of its own kind.
            placed.set_position_2d(pose.position);
            placed.set_rotation_z_radians(pose.rotation);
            let transform = world.local_for_world(*entity, placed);
            let Some(data) = world.get_mut(*entity) else {
                continue;
            };
            // A Z-locked transform is one an author said stays on its layer,
            // and physics is a write path like any other.
            if !transform.z_lock_rejects(data.transform_3d) {
                data.transform_3d = Some(transform);
            }
            // Whatever the transform holds now is what physics agrees with,
            // including a write the lock refused.
            if let Some(held) = world.world_transform(*entity) {
                self.agreed.insert(
                    *entity,
                    PhysicsPose2d {
                        position: held.position_2d(),
                        rotation: held.rotation_z_radians(),
                    },
                );
            }
        }
    }
}

/// Every entity's collider pieces: the ones authored as a collider, and the
/// ones its tilemap's solid tiles make.
///
/// One list per entity because they are one object: a tilemap on a moving
/// platform carries its tiles' collision with it. A tilemap with nothing
/// solid painted yet has no pieces and is left out, rather than refused for
/// being a collider with no shape.
pub(crate) fn collider_pieces(
    world: &World,
    components: &ComponentSchemaRegistry,
) -> Result<BTreeMap<EntityId, Vec<sindri_physics::Collider2d>>, PhysicsSyncError> {
    let mut pieces: BTreeMap<EntityId, Vec<_>> = components
        .query::<Collider2dComponent>(world)?
        .into_iter()
        .map(|(entity, collider)| (entity, collider.0))
        .collect();
    for (entity, collider) in components.query::<TilemapCollider2dComponent>(world)? {
        let tilemap = components
            .get::<TilemapComponent>(world, entity)?
            .ok_or(TilemapCollisionError::NoTilemap)?;
        let scale = world
            .world_transform(entity)
            .map_or([1.0, 1.0], Transform3D::scale_2d);
        let tiles = collider.pieces(&tilemap, scale)?;
        if !tiles.is_empty() {
            pieces.entry(entity).or_default().extend(tiles);
        }
    }
    Ok(pieces)
}

/// Where an entity is, for physics to start from.
///
/// The transform, because that is where a position is written down, composed
/// through the parent chain because physics is in the world. A body
/// component's pose is where physics writes its answer, and treating it as a
/// second authored truth is how the two drift.
pub(crate) fn pose_of(world: &World, entity: EntityId, body: Option<RigidBody2d>) -> PhysicsPose2d {
    world.world_transform(entity).map_or_else(
        || body.map(|body| body.pose).unwrap_or_default(),
        |transform: Transform3D| PhysicsPose2d {
            position: transform.position_2d(),
            rotation: transform.rotation_z_radians(),
        },
    )
}
