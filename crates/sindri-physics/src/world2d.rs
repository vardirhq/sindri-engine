//! The 2D world: Rapier behind Sindri's own types.
//!
//! Rapier appears nowhere else. Scenes, the editor, Decay, and games
//! speak only in the types this crate defines, which is what makes the
//! backend replaceable.

mod contacts;
mod controls;
mod ground;
mod grounded;
mod joints;
mod materials;
mod motion;
mod one_way;
mod query;
mod slide;
mod slope;
mod steps;
mod sweep;

pub use motion::BodyControl2d;
pub use sweep::ShapeHit2d;

use std::{collections::HashMap, sync::mpsc, time::Duration};

use rapier2d::prelude as r2;
use sindri_core::EntityId;

use crate::shared::RigidBodyKind;
use crate::types2d::{
    Collider2d, ColliderShape2d, DistanceJoint2d, PhysicsEvent2d, PhysicsEventKind, PhysicsPose2d,
    RigidBody2d,
};
use crate::validate::{PhysicsError, finite2, validate_body2d, validate_colliders2d};

#[derive(Clone)]
struct BodyRecord2d {
    body: r2::RigidBodyHandle,
    /// Every piece of the entity's collider.
    ///
    /// A list because one shape is often a poor description of a thing: a
    /// character is a capsule with a circle at each side, a ship is a box and
    /// two pods. Rapier parents them all to the one body, so they move as a
    /// single object and their mass properties sum — which is what makes a
    /// compound one collider rather than several colliding with each other.
    colliders: Vec<r2::ColliderHandle>,
    kind: RigidBodyKind,
}

/// The first runtime physics world. It owns Rapier completely and exposes only
/// Sindri entities, values, events, and joints.
pub struct PhysicsWorld2d {
    contacts: HashMap<EntityId, Vec<crate::Contact2d>>,
    backend: r2::PhysicsWorld,
    one_way: one_way::OneWayHooks,
    pending_drop: HashMap<EntityId, f32>,
    pending_controls: HashMap<EntityId, Vec<BodyControl2d>>,
    bodies: HashMap<EntityId, BodyRecord2d>,
    collider_entities: HashMap<r2::ColliderHandle, EntityId>,
    /// Velocities set on a body that does not exist yet.
    ///
    /// A script spawns a bullet and sets its velocity in the same pass — which
    /// is the shape `docs/scripting.md` documents and the reason spawned
    /// scripts start in the pass that made them. But a body is built when the
    /// scene next synchronizes, which is the *following* frame, so the set
    /// arrived before there was anything to set. Refusing it would make the
    /// documented shape impossible; dropping it would make a bullet sit still.
    /// So it is kept, and applied when the body arrives.
    ///
    /// Only ever holds entities that were asked about between a spawn and the
    /// next synchronize: anything still here after that never had a body
    /// authored, and is discarded by `finish_synchronize`.
    pending_velocity: HashMap<EntityId, [f32; 2]>,
    /// Runtime joints requested while one or both freshly spawned bodies have
    /// not reached the physics world yet. They are resolved after the scene has
    /// synchronized every body for the frame.
    pending_distance_joints: Vec<DistanceJoint2d>,
    owned_joints: HashMap<EntityId, joints::OwnedJoint>,
}

impl PhysicsWorld2d {
    pub fn new(gravity: [f32; 2]) -> Result<Self, PhysicsError> {
        finite2("gravity", gravity)?;
        let mut backend = r2::PhysicsWorld::new();
        backend.gravity = r2::Vector::new(gravity[0], gravity[1]);
        Ok(Self {
            contacts: HashMap::new(),
            backend,
            one_way: one_way::OneWayHooks::default(),
            pending_drop: HashMap::new(),
            pending_controls: HashMap::new(),
            bodies: HashMap::new(),
            collider_entities: HashMap::new(),
            pending_velocity: HashMap::new(),
            pending_distance_joints: Vec::new(),
            owned_joints: HashMap::new(),
        })
    }

    /// The pull every dynamic body feels, in units per second squared.
    pub fn gravity(&self) -> [f32; 2] {
        [self.backend.gravity.x, self.backend.gravity.y]
    }

    /// Changes gravity from the next step on. Bodies keep the velocity they
    /// have; only what pulls on them changes.
    pub fn set_gravity(&mut self, gravity: [f32; 2]) -> Result<(), PhysicsError> {
        finite2("gravity", gravity)?;
        self.backend.gravity = r2::Vector::new(gravity[0], gravity[1]);
        Ok(())
    }

    /// Registers `entity` with a body and the pieces of its collider.
    ///
    /// Nothing is inserted until every piece validates, so a compound with one
    /// bad piece leaves the world exactly as it was rather than half-built.
    pub fn insert_body(
        &mut self,
        entity: EntityId,
        body: RigidBody2d,
        colliders: &[Collider2d],
    ) -> Result<(), PhysicsError> {
        if self.bodies.contains_key(&entity) {
            return Err(PhysicsError::EntityAlreadyRegistered(entity));
        }
        if colliders.is_empty() {
            return Err(PhysicsError::NoColliderPieces(entity));
        }
        validate_body2d(&body)?;
        validate_colliders2d(colliders)?;
        if self.pending_drop.contains_key(&entity) && body.kind != RigidBodyKind::Dynamic {
            return Err(PhysicsError::WrongBodyKind(
                entity,
                "drop through",
                body.kind,
            ));
        }

        if let Some(controls) = self.pending_controls.get(&entity) {
            for control in controls {
                control.validate(entity, body.kind)?;
            }
        }

        let body_handle = self.backend.insert_body(body_builder(body));
        let handles: Vec<r2::ColliderHandle> = colliders
            .iter()
            .map(|collider| {
                self.backend
                    .insert_collider(collider_builder(entity, *collider), Some(body_handle))
            })
            .collect();
        self.backend.bodies[body_handle]
            .recompute_mass_properties_from_colliders(&self.backend.colliders);
        for handle in &handles {
            self.collider_entities.insert(*handle, entity);
        }
        self.bodies.insert(
            entity,
            BodyRecord2d {
                body: body_handle,
                colliders: handles,
                kind: body.kind,
            },
        );
        if let Some(seconds) = self.pending_drop.remove(&entity) {
            self.drop_through(entity, seconds)?;
        }
        // What a script asked for before this existed.
        if let Some(velocity) = self.pending_velocity.remove(&entity)
            && matches!(
                body.kind,
                RigidBodyKind::Dynamic | RigidBodyKind::KinematicVelocity
            )
        {
            self.backend.bodies[body_handle]
                .set_linvel(r2::Vector::new(velocity[0], velocity[1]), true);
        }
        if let Some(controls) = self.pending_controls.remove(&entity) {
            for control in controls {
                self.apply_control(entity, control)?;
            }
        }
        Ok(())
    }

    /// Remembers a velocity for a body that has not been built yet.
    ///
    /// For the one caller that knows the entity is real and authored physics —
    /// the scripting host, which can see the components on it — and would
    /// otherwise have to refuse the frame a bullet is spawned on. Everything
    /// else should use `set_linear_velocity` and hear about a body that is not
    /// there.
    pub fn remember_linear_velocity(
        &mut self,
        entity: EntityId,
        velocity: [f32; 2],
    ) -> Result<(), PhysicsError> {
        finite2("linear_velocity", velocity)?;
        self.pending_velocity.insert(entity, velocity);
        self.pending_controls
            .entry(entity)
            .or_default()
            .push(BodyControl2d::LinearVelocity(velocity));
        Ok(())
    }

    /// Inserts an entity with no authored rigid-body as static collision
    /// geometry. The hidden backend may synthesize a fixed body; that choice is
    /// intentionally not observable through the Sindri API.
    pub fn insert_static_collider(
        &mut self,
        entity: EntityId,
        pose: PhysicsPose2d,
        colliders: &[Collider2d],
    ) -> Result<(), PhysicsError> {
        self.insert_body(
            entity,
            RigidBody2d {
                kind: RigidBodyKind::Static,
                pose,
                ..RigidBody2d::default()
            },
            colliders,
        )
    }

    pub fn remove(&mut self, entity: EntityId) -> bool {
        self.remove_owned_joint(entity);
        self.owned_joints.retain(|_, record| {
            let (first, second) = record.joint.endpoints();
            first != entity && second != entity
        });
        self.invalidate_contacts(entity);
        self.pending_controls.remove(&entity);
        self.pending_drop.remove(&entity);
        self.pending_velocity.remove(&entity);
        self.pending_distance_joints
            .retain(|joint| joint.first != entity && joint.second != entity);
        let Some(record) = self.bodies.remove(&entity) else {
            return false;
        };
        for handle in &record.colliders {
            self.one_way.policies.remove(handle);
            self.collider_entities.remove(handle);
        }
        // Rapier drops a body's colliders and attached joints with it. That is
        // important for breakable chains: removing one link severs both sides
        // without a stale constraint surviving on behalf of a dead entity.
        self.one_way.dropping.remove(&record.body);
        let _ = self.backend.remove_body(record.body);
        true
    }

    pub fn contains(&self, entity: EntityId) -> bool {
        self.bodies.contains_key(&entity)
    }

    pub fn len(&self) -> usize {
        self.bodies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }

    /// Number of runtime impulse joints currently owned by the backend.
    ///
    /// Distance joints are the first public kind, so this is primarily a small
    /// proof/debugging surface rather than an attempt to expose backend handles.
    pub fn joint_count(&self) -> usize {
        self.backend.impulse_joints.len()
    }

    pub fn body_kind(&self, entity: EntityId) -> Result<RigidBodyKind, PhysicsError> {
        Ok(self.record(entity)?.kind)
    }

    pub fn pose(&self, entity: EntityId) -> Result<PhysicsPose2d, PhysicsError> {
        let record = self.record(entity)?;
        let body = &self.backend.bodies[record.body];
        let position = body.translation();
        Ok(PhysicsPose2d {
            position: [position.x, position.y],
            rotation: body.rotation().angle(),
        })
    }

    /// What the body weighs, summed over every piece of its collider.
    ///
    /// Exposed because a compound's mass is derived rather than authored: the
    /// pieces decide it, and a claim about how they add up is worth being able
    /// to check from outside the crate.
    pub fn mass(&self, entity: EntityId) -> Result<f32, PhysicsError> {
        Ok(self.backend.bodies[self.record(entity)?.body].mass())
    }

    /// Advances exactly one engine fixed step and returns normalized Sindri
    /// collision/sensor events generated during that step.
    pub fn step(&mut self, delta: Duration) -> Result<Vec<PhysicsEvent2d>, PhysicsError> {
        let dt = delta.as_secs_f32();
        if !dt.is_finite() || dt <= 0.0 {
            return Err(PhysicsError::InvalidTimestep);
        }
        self.backend.integration_parameters.dt = dt;

        let (collision_send, collision_recv) = mpsc::channel();
        let (force_send, _force_recv) = mpsc::channel();
        // Sindri currently creates only rigid bodies, so tear events stay private.
        let (tear_send, _tear_recv) = mpsc::channel();
        let events = r2::ChannelEventCollector::new(collision_send, force_send, tear_send);
        self.backend.step_with_events(&self.one_way, &events);
        self.snapshot_contacts(dt);
        self.clear_step_forces();
        self.one_way.dropping.retain(|_, seconds| {
            *seconds -= dt;
            *seconds > 0.0
        });

        Ok(collision_recv
            .try_iter()
            .filter_map(|event| self.normalize_event(event))
            .collect())
    }

    fn record(&self, entity: EntityId) -> Result<&BodyRecord2d, PhysicsError> {
        self.bodies
            .get(&entity)
            .ok_or(PhysicsError::MissingEntity(entity))
    }

    fn normalize_event(&self, event: r2::CollisionEvent) -> Option<PhysicsEvent2d> {
        let mut first = *self.collider_entities.get(&event.collider1())?;
        let mut second = *self.collider_entities.get(&event.collider2())?;
        if second < first {
            std::mem::swap(&mut first, &mut second);
        }
        let kind = match (event.started(), event.sensor()) {
            (true, false) => PhysicsEventKind::CollisionStarted,
            (false, false) => PhysicsEventKind::CollisionStopped,
            (true, true) => PhysicsEventKind::SensorEntered,
            (false, true) => PhysicsEventKind::SensorExited,
        };
        Some(PhysicsEvent2d {
            first,
            second,
            kind,
        })
    }
}

fn body_builder(body: RigidBody2d) -> r2::RigidBodyBuilder {
    let builder = match body.kind {
        RigidBodyKind::Static => r2::RigidBodyBuilder::fixed(),
        RigidBodyKind::Dynamic => r2::RigidBodyBuilder::dynamic(),
        RigidBodyKind::KinematicPosition => r2::RigidBodyBuilder::kinematic_position_based(),
        RigidBodyKind::KinematicVelocity => r2::RigidBodyBuilder::kinematic_velocity_based(),
    }
    .translation(r2::Vector::new(
        body.pose.position[0],
        body.pose.position[1],
    ))
    .rotation(body.pose.rotation)
    .linvel(r2::Vector::new(
        body.linear_velocity[0],
        body.linear_velocity[1],
    ))
    .ccd_enabled(body.continuous_collision)
    .angvel(body.angular_velocity)
    .gravity_scale(body.gravity_scale)
    .linear_damping(body.linear_damping)
    .angular_damping(body.angular_damping);

    if body.lock_rotation {
        builder.lock_rotations()
    } else {
        builder
    }
}

fn collider_builder(entity: EntityId, collider: Collider2d) -> r2::ColliderBuilder {
    let groups = r2::InteractionGroups::new(
        r2::Group::from_bits_retain(collider.layers.memberships),
        r2::Group::from_bits_retain(collider.layers.filter),
        r2::InteractionTestMode::And,
    );
    let builder = match collider.shape {
        ColliderShape2d::Box { half_extents } => {
            r2::ColliderBuilder::cuboid(half_extents[0], half_extents[1])
        }
        ColliderShape2d::Circle { radius } => r2::ColliderBuilder::ball(radius),
        ColliderShape2d::Capsule {
            half_height,
            radius,
        } => r2::ColliderBuilder::capsule_y(half_height, radius),
    };

    builder
        .translation(r2::Vector::new(collider.offset[0], collider.offset[1]))
        .rotation(collider.rotation)
        .sensor(collider.sensor)
        .collision_groups(groups)
        .active_collision_types(r2::ActiveCollisionTypes::all())
        .active_events(r2::ActiveEvents::COLLISION_EVENTS)
        .friction(collider.friction)
        // The smaller of the two frictions, so a frictionless collider is
        // frictionless against everything: a platformer's hero pressed into a
        // wall slides down it rather than clinging. Averaging, the backend's
        // default, would give it half the wall's friction. Two equal
        // frictions combine to the same value either way.
        .friction_combine_rule(r2::CoefficientCombineRule::Min)
        .restitution(collider.restitution)
        .user_data(u128::from(entity.to_bits()))
}
