//! Authored voxel worlds as collision snapshots near the bodies that can hit
//! them.
//!
//! A voxel world is unbounded and its render window follows the camera, so
//! neither is a collision residency. What can collide with voxels is a dynamic
//! body, so the sections resident here are the ones each dynamic body could
//! reach this step, plus an authored margin. A body far from the camera still
//! lands; terrain nobody is near costs nothing.

use std::collections::BTreeSet;

use glam::{Quat, Vec3};
use sindri_core::{ComponentSchemaRegistry, EntityId, World};
use sindri_physics::{PhysicsError, PhysicsPose3d, PhysicsWorld3d};
use sindri_voxel::{SECTION_EDGE, SectionCoord, VoxelId, VoxelSection, VoxelShape};

use super::{
    SceneVoxelCollision3d, VoxelCollisionError3d, VoxelCollisionPlan3d, VoxelCollisionSection3d,
    VoxelCollisionSettings3d, VoxelCollisionWorld3d,
};
use crate::{
    Collider3dComponent, PhysicsSyncError, RigidBody3dComponent, TileSetBindings,
    VoxelCollider3dComponent, VoxelGround, VoxelWorldComponent,
};

/// Something that can hit voxels this step: where it is, and how far from
/// there any part of it can be by the end of the step, in world units.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VoxelReach {
    pub position: [f32; 3],
    pub reach: f32,
}

struct Resolved {
    owner: EntityId,
    ground: VoxelGround,
    pose: PhysicsPose3d,
    scale: [f32; 3],
    settings: VoxelCollisionSettings3d,
    sections: Vec<(SectionCoord, VoxelSection, u64)>,
}

impl SceneVoxelCollision3d {
    /// Plans every active authored voxel collider against the bodies that can
    /// reach it, changing nothing. Worlds no longer authored are released by
    /// the plan's commit.
    pub(crate) fn plan_authored(
        &self,
        world: &World,
        components: &ComponentSchemaRegistry,
        tile_sets: Option<&TileSetBindings>,
        physics: &PhysicsWorld3d,
        reaches: &[VoxelReach],
    ) -> Result<VoxelCollisionPlan3d, PhysicsSyncError> {
        let mut resolved = Vec::new();
        for (owner, collider) in components.query::<VoxelCollider3dComponent>(world)? {
            if !world.is_active(owner) {
                continue;
            }
            resolved.push(self.resolve(world, components, tile_sets, owner, collider, reaches)?);
        }
        let sections: Vec<Vec<VoxelCollisionSection3d<'_>>> = resolved
            .iter()
            .map(|world| {
                world
                    .sections
                    .iter()
                    .map(|(coord, voxels, revision)| VoxelCollisionSection3d {
                        coord: *coord,
                        revision: *revision,
                        voxels,
                    })
                    .collect()
            })
            .collect();
        let policies: Vec<_> = resolved
            .iter()
            .map(|world| {
                let ground = &world.ground;
                move |voxel: VoxelId| -> Option<VoxelShape> { ground.collision_shape(voxel) }
            })
            .collect();
        let inputs: Vec<_> = resolved
            .iter()
            .zip(&sections)
            .zip(&policies)
            .map(|((world, sections), policy)| VoxelCollisionWorld3d {
                owner: world.owner,
                pose: world.pose,
                scale: world.scale,
                policy_revision: world.ground.collision_revision(),
                policy,
                settings: world.settings,
                sections,
            })
            .collect();
        Ok(self.plan(world, physics, &inputs)?)
    }

    fn resolve(
        &self,
        world: &World,
        components: &ComponentSchemaRegistry,
        tile_sets: Option<&TileSetBindings>,
        owner: EntityId,
        collider: VoxelCollider3dComponent,
        reaches: &[VoxelReach],
    ) -> Result<Resolved, PhysicsSyncError> {
        if components
            .get::<Collider3dComponent>(world, owner)?
            .is_some()
            || components
                .get::<RigidBody3dComponent>(world, owner)?
                .is_some()
        {
            return Err(PhysicsSyncError::ConflictingVoxelOwner(owner));
        }
        let voxels = components
            .get::<VoxelWorldComponent>(world, owner)?
            .ok_or(PhysicsSyncError::MissingVoxelWorld(owner))?;
        if !collider.margin.is_finite() {
            return Err(PhysicsError::NonFinite("voxel collider margin").into());
        }
        if collider.margin < 0.0 {
            return Err(PhysicsError::Negative("voxel collider margin").into());
        }
        let ground = VoxelGround::of(&voxels, tile_sets)
            .map_err(|error| PhysicsSyncError::VoxelWorld(owner, error))?;
        let (pose, scale) = placement(world, components, owner);
        if !scale.into_iter().all(|axis| axis.is_finite() && axis > 0.0) {
            return Err(VoxelCollisionError3d::Scale.into());
        }
        let resident = self.resident(pose, scale, collider.margin, reaches)?;
        let sections = resident
            .into_iter()
            .map(|coord| {
                let section = ground.section(coord);
                let revision = VoxelGround::section_revision(&section);
                (coord, section, revision)
            })
            .collect();
        Ok(Resolved {
            owner,
            ground,
            pose,
            scale,
            settings: VoxelCollisionSettings3d {
                layers: collider.layers,
                friction: collider.friction,
                restitution: collider.restitution,
                sensor: false,
            },
            sections,
        })
    }

    /// Every section within reach of any body, in the world's voxel space.
    /// Refuses before enumerating a window larger than the section budget.
    fn resident(
        &self,
        pose: PhysicsPose3d,
        scale: [f32; 3],
        margin: f32,
        reaches: &[VoxelReach],
    ) -> Result<BTreeSet<SectionCoord>, VoxelCollisionError3d> {
        let rotation = Quat::from_array(pose.rotation).normalize();
        let origin = Vec3::from_array(pose.position);
        let scale = Vec3::from_array(scale);
        let edge = f64::from(SECTION_EDGE);
        let mut resident = BTreeSet::new();
        for reach in reaches {
            let position = Vec3::from_array(reach.position);
            if !position.is_finite() || !reach.reach.is_finite() {
                continue;
            }
            let local = rotation.inverse() * (position - origin) / scale;
            let radius = Vec3::splat(reach.reach) / scale + Vec3::splat(margin);
            let range = |axis: usize| {
                let low = f64::from(local[axis] - radius[axis]);
                let high = f64::from(local[axis] + radius[axis]);
                ((low / edge).floor(), (high / edge).floor())
            };
            let ranges = [range(0), range(1), range(2)];
            let count: f64 = ranges.iter().map(|(low, high)| high - low + 1.0).product();
            #[allow(clippy::cast_precision_loss)]
            let budget = self.budget.sections as f64;
            if !count.is_finite() || count > budget {
                return Err(VoxelCollisionError3d::Budget("sections"));
            }
            // Bounded by the budget check above, so each axis fits an i32.
            #[allow(clippy::cast_possible_truncation)]
            let [x, y, z] = ranges.map(|(low, high)| (low as i32, high as i32));
            for sx in x.0..=x.1 {
                for sy in y.0..=y.1 {
                    for sz in z.0..=z.1 {
                        resident.insert(SectionCoord::new(sx, sy, sz));
                    }
                }
            }
        }
        Ok(resident)
    }
}

/// The voxel world's composed pose and per-axis scale: the entity's world
/// transform, then its voxel space (a grid's cell size and centring).
fn placement(
    world: &World,
    components: &ComponentSchemaRegistry,
    owner: EntityId,
) -> (PhysicsPose3d, [f32; 3]) {
    let placed = world.world_transform(owner).unwrap_or_default();
    let space = crate::voxel_space(world, components, owner);
    let cell = Vec3::new(space.x_axis.x, space.y_axis.y, space.z_axis.z);
    let shift = space.w_axis.truncate();
    let rotation = Quat::from_array(placed.rotation);
    let owner_scale = Vec3::from_array(placed.scale);
    let position = Vec3::from_array(placed.position) + rotation * (owner_scale * shift);
    (
        PhysicsPose3d {
            position: position.to_array(),
            rotation: placed.rotation,
        },
        (owner_scale * cell).to_array(),
    )
}
