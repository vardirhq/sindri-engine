//! Fixed-step 3D rigid bodies behind the Sindri entity boundary.

mod build;
mod controls;
mod pending;
mod query;
mod spatial;
mod static_groups;
mod sweep;
mod validation;

use std::{
    collections::{BTreeMap, HashMap},
    sync::mpsc,
    time::Duration,
};

use rapier3d::prelude as r3;
use sindri_core::EntityId;

use crate::{
    Collider3d, PhysicsError, PhysicsEvent3d, PhysicsEventKind, PhysicsPose3d, RigidBody3d,
    RigidBodyKind,
};
pub use pending::BodyControl3d;
use validation::{finite3, validate_body, validate_colliders};

struct BodyRecord {
    body: r3::RigidBodyHandle,
    colliders: Vec<r3::ColliderHandle>,
    kind: RigidBodyKind,
    base_colliders: usize,
    groups: BTreeMap<u64, Vec<r3::ColliderHandle>>,
}

/// A standalone 3D world. Hosts supply each fixed step; backend types stay private.
/// Hosts own scene synchronization, scripting and streamed geometry lifecycle.
pub struct PhysicsWorld3d {
    backend: r3::PhysicsWorld,
    bodies: HashMap<EntityId, BodyRecord>,
    collider_entities: HashMap<r3::ColliderHandle, EntityId>,
    spatial: spatial::SpatialIndex,
    pending_controls: HashMap<EntityId, Vec<BodyControl3d>>,
}

impl PhysicsWorld3d {
    pub fn new(gravity: [f32; 3]) -> Result<Self, PhysicsError> {
        finite3("gravity", gravity)?;
        let mut backend = r3::PhysicsWorld::new();
        backend.gravity = r3::Vector::from_array(gravity);
        Ok(Self {
            backend,
            bodies: HashMap::new(),
            collider_entities: HashMap::new(),
            spatial: spatial::SpatialIndex::default(),
            pending_controls: HashMap::new(),
        })
    }

    pub fn gravity(&self) -> [f32; 3] {
        self.backend.gravity.to_array()
    }

    pub fn set_gravity(&mut self, gravity: [f32; 3]) -> Result<(), PhysicsError> {
        finite3("gravity", gravity)?;
        let gravity = r3::Vector::from_array(gravity);
        if gravity != self.backend.gravity {
            self.backend.gravity = gravity;
            // Resting bodies sleep; without a wake they would ignore the change.
            for (_, body) in self.backend.bodies.iter_mut() {
                if body.is_dynamic() {
                    body.wake_up(true);
                }
            }
        }
        Ok(())
    }

    /// Validates the body and every collider before inserting anything.
    /// Collider poses are relative to their parent; mass sums across pieces.
    pub fn insert_body(
        &mut self,
        entity: EntityId,
        body: RigidBody3d,
        colliders: &[Collider3d],
    ) -> Result<(), PhysicsError> {
        if self.bodies.contains_key(&entity) {
            return Err(PhysicsError::EntityAlreadyRegistered(entity));
        }
        self.validate_insertion(entity, body, colliders)?;
        let handle = self.backend.insert_body(build::body(body));
        let handles: Vec<_> = colliders
            .iter()
            .map(|collider| {
                self.backend
                    .insert_collider(build::collider(entity, *collider), Some(handle))
            })
            .collect();
        self.backend.bodies[handle]
            .recompute_mass_properties_from_colliders(&self.backend.colliders);
        for &collider in &handles {
            self.collider_entities.insert(collider, entity);
        }
        self.bodies.insert(
            entity,
            BodyRecord {
                body: handle,
                colliders: handles,
                kind: body.kind,
                base_colliders: colliders.len(),
                groups: BTreeMap::new(),
            },
        );
        self.index_body(entity);
        if let Some(controls) = self.pending_controls.remove(&entity) {
            for control in controls {
                self.apply_control(entity, control)?;
            }
        }
        Ok(())
    }

    /// Checks a complete insertion request without touching runtime state.
    /// Scene drivers use this before reconciling a batch of authored edits.
    pub fn validate_body(
        entity: EntityId,
        body: RigidBody3d,
        colliders: &[Collider3d],
    ) -> Result<(), PhysicsError> {
        if colliders.is_empty() {
            return Err(PhysicsError::NoColliderPieces(entity));
        }
        validate_body(body)?;
        validate_colliders(colliders)
    }

    pub fn insert_static_collider(
        &mut self,
        entity: EntityId,
        pose: PhysicsPose3d,
        colliders: &[Collider3d],
    ) -> Result<(), PhysicsError> {
        self.insert_body(
            entity,
            RigidBody3d {
                kind: RigidBodyKind::Static,
                position: pose.position,
                rotation: pose.rotation,
                ..RigidBody3d::default()
            },
            colliders,
        )
    }

    pub fn remove(&mut self, entity: EntityId) -> bool {
        self.pending_controls.remove(&entity);
        let Some(record) = self.bodies.remove(&entity) else {
            return false;
        };
        self.spatial.remove_body(entity);
        for handle in record.colliders {
            self.spatial.remove(handle);
            self.collider_entities.remove(&handle);
        }
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
    pub fn body_kind(&self, entity: EntityId) -> Result<RigidBodyKind, PhysicsError> {
        Ok(self.record(entity)?.kind)
    }
    pub fn mass(&self, entity: EntityId) -> Result<f32, PhysicsError> {
        Ok(self.backend.bodies[self.record(entity)?.body].mass())
    }

    pub fn pose(&self, entity: EntityId) -> Result<PhysicsPose3d, PhysicsError> {
        let body = &self.backend.bodies[self.record(entity)?.body];
        Ok(PhysicsPose3d {
            position: body.translation().to_array(),
            rotation: body.rotation().to_array(),
        })
    }

    /// Advances exactly one positive finite engine fixed step.
    /// Events contain normalized Sindri entity pairs, never backend handles.
    pub fn step(&mut self, delta: Duration) -> Result<Vec<PhysicsEvent3d>, PhysicsError> {
        let dt = delta.as_secs_f32();
        if !dt.is_finite() || dt <= 0.0 {
            return Err(PhysicsError::InvalidTimestep);
        }
        self.backend.integration_parameters.dt = dt;
        let (collision_send, collision_recv) = mpsc::channel();
        let (force_send, _force_recv) = mpsc::channel();
        let (tear_send, _tear_recv) = mpsc::channel();
        let events = r3::ChannelEventCollector::new(collision_send, force_send, tear_send);
        self.backend.step_with_events(&(), &events);
        self.index_simulated_bodies();
        Ok(collision_recv
            .try_iter()
            .filter_map(|event| self.normalize_event(event))
            .collect())
    }

    fn record(&self, entity: EntityId) -> Result<&BodyRecord, PhysicsError> {
        self.bodies
            .get(&entity)
            .ok_or(PhysicsError::MissingEntity(entity))
    }

    fn normalize_event(&self, event: r3::CollisionEvent) -> Option<PhysicsEvent3d> {
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
        Some(PhysicsEvent3d {
            first,
            second,
            kind,
        })
    }
}
