//! Snapshot the solver's contacts while keeping all backend handles private.

use std::cmp::Ordering;

use sindri_core::EntityId;

use super::PhysicsWorld2d;
use crate::{Contact2d, PhysicsError};

impl PhysicsWorld2d {
    /// Copied solid contacts from the last valid fixed step, ordered by other
    /// entity, world point, normal, then impulse. Sensors never appear here.
    /// Before the first step the answer is empty. Missing bodies are errors.
    /// Removal and teleports invalidate affected contacts immediately.
    pub fn contacts(&self, entity: EntityId) -> Result<Vec<Contact2d>, PhysicsError> {
        self.record(entity)?;
        Ok(self.contacts.get(&entity).cloned().unwrap_or_default())
    }

    pub(super) fn invalidate_contacts(&mut self, entity: EntityId) {
        self.contacts.remove(&entity);
        for contacts in self.contacts.values_mut() {
            contacts.retain(|contact| contact.entity != entity);
        }
    }

    pub(super) fn snapshot_contacts(&mut self, dt: f32) {
        self.contacts.clear();
        for pair in self.backend.narrow_phase.contact_pairs() {
            let Some(first) = self.collider_entities.get(&pair.collider1).copied() else {
                continue;
            };
            let Some(second) = self.collider_entities.get(&pair.collider2).copied() else {
                continue;
            };
            let Some(rigid) = pair.rigid() else {
                continue;
            };
            // Sleeping manifolds retain old solver impulses for warm starting.
            // They prove support, but are not a newly solved impact this step.
            let awake = [first, second].into_iter().any(|entity| {
                let body = &self.backend.bodies[self.bodies[&entity].body];
                body.is_dynamic() && !body.is_sleeping()
            });
            for manifold in rigid.solver_manifolds() {
                for solver in &manifold.data.solver_contacts {
                    let index = solver.contact_indices()[0];
                    let Some(contact) = usize::try_from(index)
                        .ok()
                        .and_then(|index| manifold.points.get(index))
                    else {
                        continue;
                    };
                    let (p1, p2) = manifold
                        .data
                        .solver_contact_world_points(solver, &self.backend.bodies);
                    let p = (p1 + p2) * 0.5;
                    let normal = -manifold.data.normal;
                    let normal_impulse = if awake { contact.data.impulse } else { 0.0 };
                    let tangent_impulse = if awake {
                        contact.data.tangent_impulse[0]
                    } else {
                        0.0
                    };
                    let force = [
                        (normal.x * normal_impulse - normal.y * tangent_impulse) / dt,
                        (normal.y * normal_impulse + normal.x * tangent_impulse) / dt,
                    ];
                    self.contacts.entry(first).or_default().push(Contact2d {
                        entity: second,
                        point: [p.x, p.y],
                        normal: [normal.x, normal.y],
                        normal_impulse,
                        tangent_impulse,
                        force,
                    });
                    self.contacts.entry(second).or_default().push(Contact2d {
                        entity: first,
                        point: [p.x, p.y],
                        normal: [-normal.x, -normal.y],
                        normal_impulse,
                        tangent_impulse,
                        force: [-force[0], -force[1]],
                    });
                }
            }
        }
        for contacts in self.contacts.values_mut() {
            contacts.sort_by(compare_contacts);
        }
    }
}

fn compare_contacts(a: &Contact2d, b: &Contact2d) -> Ordering {
    a.entity
        .cmp(&b.entity)
        .then_with(|| a.point[0].total_cmp(&b.point[0]))
        .then_with(|| a.point[1].total_cmp(&b.point[1]))
        .then_with(|| a.normal[0].total_cmp(&b.normal[0]))
        .then_with(|| a.normal[1].total_cmp(&b.normal[1]))
        .then_with(|| a.normal_impulse.total_cmp(&b.normal_impulse))
        .then_with(|| a.tangent_impulse.total_cmp(&b.tangent_impulse))
}
