//! Transactional resident voxel snapshots into keyed static solver geometry.

mod authored;
mod geometry;
pub(crate) use authored::VoxelReach;
mod types;
pub use types::*;

use sindri_core::{EntityId, World};
use sindri_physics::{Collider3d, PhysicsPose3d, PhysicsWorld3d};
use sindri_voxel::{SectionCollisionBox, SectionCoord, compile_section_collision};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone)]
struct Section {
    key: u64,
    revision: u64,
    policy_revision: u64,
    scale: [f32; 3],
    settings: VoxelCollisionSettings3d,
    boxes: Arc<[SectionCollisionBox]>,
    pieces: Arc<[Collider3d]>,
}

/// A prevalidated snapshot, ready to commit.
pub(crate) struct VoxelCollisionPlan3d {
    owners: BTreeMap<EntityId, Owner>,
    next_key: u64,
    report: VoxelCollisionReport3d,
}

struct Owner {
    pose: PhysicsPose3d,
    materialized: bool,
    sections: BTreeMap<SectionCoord, Section>,
}

/// Derived collision state beside the scene. Owns each supplied entity's static
/// solver body exclusively; hosts recreate this cache whenever the solver resets.
/// Inputs are the complete resident snapshot, not just its dirty subset.
/// This adapter does not step physics, resolve authoring or choose residency.
pub struct SceneVoxelCollision3d {
    budget: VoxelCollisionBudget3d,
    owners: BTreeMap<EntityId, Owner>,
    next_key: u64,
}

impl Default for SceneVoxelCollision3d {
    fn default() -> Self {
        Self::new(VoxelCollisionBudget3d::default())
    }
}

impl SceneVoxelCollision3d {
    pub fn new(budget: VoxelCollisionBudget3d) -> Self {
        Self {
            budget,
            owners: BTreeMap::new(),
            next_key: 0,
        }
    }

    /// Prepares and validates the complete snapshot before changing any solver
    /// geometry or cached revisions. Inactive owners are omitted; missing owners,
    /// duplicate inputs, bad geometry/poses and budget excess fail atomically.
    /// Unchanged sections retain geometry. Pose-only changes move the owner;
    /// scale/settings changes rebuild pieces from retained compiled boxes.
    ///
    /// # Errors
    /// Returns [`VoxelCollisionError3d`] without committing a partial snapshot.
    pub fn synchronize(
        &mut self,
        world: &World,
        physics: &mut PhysicsWorld3d,
        inputs: &[VoxelCollisionWorld3d<'_>],
    ) -> Result<VoxelCollisionReport3d, VoxelCollisionError3d> {
        let plan = self.plan(world, physics, inputs)?;
        self.commit(physics, plan)
    }

    /// The validated snapshot `synchronize` would commit, changing nothing.
    pub(crate) fn plan(
        &self,
        world: &World,
        physics: &PhysicsWorld3d,
        inputs: &[VoxelCollisionWorld3d<'_>],
    ) -> Result<VoxelCollisionPlan3d, VoxelCollisionError3d> {
        let (owners, next_key, report) = self.prepare(world, physics, inputs)?;
        Ok(VoxelCollisionPlan3d {
            owners,
            next_key,
            report,
        })
    }

    /// Commits a plan made against this solver state. Hosts that change other
    /// solver bodies in between must not touch the plan's owners.
    pub(crate) fn commit(
        &mut self,
        physics: &mut PhysicsWorld3d,
        plan: VoxelCollisionPlan3d,
    ) -> Result<VoxelCollisionReport3d, VoxelCollisionError3d> {
        let VoxelCollisionPlan3d {
            owners: mut plan,
            next_key,
            mut report,
        } = plan;
        for (&entity, old) in &self.owners {
            if !plan.contains_key(&entity) {
                if old.materialized {
                    physics.remove(entity);
                }
                report.removed += old.sections.len();
            }
        }
        for (&entity, owner) in &mut plan {
            let previous = self.owners.get(&entity);
            if let Some(old) = previous {
                for (coord, section) in &old.sections {
                    if !owner.sections.contains_key(coord) {
                        if old.materialized {
                            physics.remove_static_group(entity, section.key)?;
                        }
                        report.removed += 1;
                    }
                }
            }
            for (coord, section) in &owner.sections {
                let unchanged = previous
                    .and_then(|old| old.sections.get(coord))
                    .is_some_and(|old| Arc::ptr_eq(&old.pieces, &section.pieces));
                if !unchanged && (!section.pieces.is_empty() || physics.contains(entity)) {
                    physics.replace_static_group(
                        entity,
                        section.key,
                        owner.pose,
                        &section.pieces,
                    )?;
                }
            }
            owner.materialized = physics.contains(entity);
            if owner.materialized && previous.is_none_or(|old| old.pose != owner.pose) {
                physics.move_to(entity, owner.pose)?;
            }
        }
        self.owners = plan;
        self.next_key = next_key;
        Ok(report)
    }

    fn prepare(
        &self,
        world: &World,
        physics: &PhysicsWorld3d,
        inputs: &[VoxelCollisionWorld3d<'_>],
    ) -> Result<(BTreeMap<EntityId, Owner>, u64, VoxelCollisionReport3d), VoxelCollisionError3d>
    {
        let mut plan = BTreeMap::new();
        let mut seen = BTreeSet::new();
        let mut next_key = self.next_key;
        let mut report = VoxelCollisionReport3d::default();
        for input in inputs {
            if !seen.insert(input.owner) {
                return Err(VoxelCollisionError3d::DuplicateOwner(input.owner));
            }
            if world.get(input.owner).is_none() {
                return Err(VoxelCollisionError3d::MissingOwner(input.owner));
            }
            if !world.is_active(input.owner) {
                continue;
            }
            report.worlds += 1;
            limit(report.worlds, self.budget.worlds, "worlds")?;
            let previous = self.owners.get(&input.owner);
            if physics.contains(input.owner) != previous.is_some_and(|old| old.materialized) {
                return Err(VoxelCollisionError3d::Ownership(input.owner));
            }
            physics.validate_static_group(input.owner, input.pose, &[])?;
            // Even an empty snapshot must reject bad scale/coefficients.
            let sample = geometry::pieces(
                SectionCoord::default(),
                &[SectionCollisionBox {
                    min: [0; 3],
                    max: [16; 3],
                }],
                input.scale,
                input.settings,
            )?;
            physics.validate_static_group(input.owner, input.pose, &sample)?;
            report.sections = report.sections.saturating_add(input.sections.len());
            limit(report.sections, self.budget.sections, "sections")?;
            let mut sections = BTreeMap::new();
            for snapshot in ordered_sections(input)?.into_values() {
                let old = previous.and_then(|owner| owner.sections.get(&snapshot.coord));
                let section = self.section(input, snapshot, old, &mut next_key, &mut report)?;
                report.pieces = report.pieces.saturating_add(section.pieces.len());
                limit(report.pieces, self.budget.pieces, "pieces")?;
                physics.validate_static_group(input.owner, input.pose, &section.pieces)?;
                sections.insert(snapshot.coord, section);
            }
            plan.insert(
                input.owner,
                Owner {
                    pose: input.pose,
                    materialized: false,
                    sections,
                },
            );
        }
        // A lost owned body is an error even when this snapshot removes it.
        for (&entity, owner) in &self.owners {
            if owner.materialized {
                if !physics.contains(entity) {
                    return Err(VoxelCollisionError3d::Ownership(entity));
                }
                physics.validate_static_group(entity, owner.pose, &[])?;
            }
        }
        Ok((plan, next_key, report))
    }

    fn section(
        &self,
        input: &VoxelCollisionWorld3d<'_>,
        snapshot: &VoxelCollisionSection3d<'_>,
        old: Option<&Section>,
        next_key: &mut u64,
        report: &mut VoxelCollisionReport3d,
    ) -> Result<Section, VoxelCollisionError3d> {
        let changed = old.is_none_or(|old| {
            old.revision != snapshot.revision || old.policy_revision != input.policy_revision
        });
        if let Some(old) = old.filter(|old| {
            !changed
                && old.scale.map(f32::to_bits) == input.scale.map(f32::to_bits)
                && old.settings == input.settings
        }) {
            return Ok(old.clone());
        }
        report.rebuilt += 1;
        limit(report.rebuilt, self.budget.rebuilds, "rebuilds")?;
        let boxes = if let Some(old) = old.filter(|_| !changed) {
            old.boxes.clone()
        } else {
            report.compiled += 1;
            Arc::from(compile_section_collision(snapshot.voxels, input.policy)?)
        };
        let pieces = Arc::from(geometry::pieces(
            snapshot.coord,
            &boxes,
            input.scale,
            input.settings,
        )?);
        let key = if let Some(old) = old {
            old.key
        } else {
            let key = *next_key;
            *next_key = next_key.checked_add(1).ok_or(VoxelCollisionError3d::Keys)?;
            key
        };
        Ok(Section {
            key,
            revision: snapshot.revision,
            policy_revision: input.policy_revision,
            scale: input.scale,
            settings: input.settings,
            boxes,
            pieces,
        })
    }
}

fn ordered_sections<'a>(
    input: &VoxelCollisionWorld3d<'a>,
) -> Result<BTreeMap<SectionCoord, &'a VoxelCollisionSection3d<'a>>, VoxelCollisionError3d> {
    let mut sections = BTreeMap::new();
    for section in input.sections {
        if sections.insert(section.coord, section).is_some() {
            return Err(VoxelCollisionError3d::DuplicateSection(
                input.owner,
                section.coord,
            ));
        }
    }
    Ok(sections)
}

fn limit(value: usize, maximum: usize, name: &'static str) -> Result<(), VoxelCollisionError3d> {
    if value > maximum {
        Err(VoxelCollisionError3d::Budget(name))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
