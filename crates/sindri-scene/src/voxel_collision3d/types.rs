use sindri_core::EntityId;
use sindri_physics::{CollisionLayers, PhysicsError, PhysicsPose3d};
use sindri_voxel::{SectionCoord, VoxelId, VoxelSection, VoxelShape};
use thiserror::Error;

/// An explicit resident occupancy snapshot. Revision must change whenever its
/// generated or edited occupancy changes, including source replacement.
pub struct VoxelCollisionSection3d<'a> {
    pub coord: SectionCoord,
    pub revision: u64,
    pub voxels: &'a VoxelSection,
}

/// One collision world's resolved snapshot. Pose/scale are composed world-space
/// values; residency and block policy are supplied by the host, never inferred
/// from render visibility. All sections share these physical settings.
/// Policy revision must change whenever the policy's answer changes.
pub struct VoxelCollisionWorld3d<'a> {
    pub owner: EntityId,
    pub pose: PhysicsPose3d,
    pub scale: [f32; 3],
    pub policy_revision: u64,
    pub policy: &'a dyn Fn(VoxelId) -> Option<VoxelShape>,
    pub settings: VoxelCollisionSettings3d,
    pub sections: &'a [VoxelCollisionSection3d<'a>],
}

/// Shared coefficients/filters for a world's section groups.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoxelCollisionSettings3d {
    pub layers: CollisionLayers,
    pub friction: f32,
    pub restitution: f32,
    pub sensor: bool,
}

impl Default for VoxelCollisionSettings3d {
    fn default() -> Self {
        Self {
            layers: CollisionLayers::ALL,
            friction: 0.5,
            restitution: 0.0,
            sensor: false,
        }
    }
}

/// Per-call limits for retained worlds/sections/pieces and replaced sections.
/// Over-budget snapshots fail atomically; hosts must choose a bounded residency
/// or explicitly increase limits, rather than solve against a partial snapshot.
#[derive(Clone, Copy, Debug)]
pub struct VoxelCollisionBudget3d {
    pub worlds: usize,
    pub sections: usize,
    pub rebuilds: usize,
    pub pieces: usize,
}

impl Default for VoxelCollisionBudget3d {
    fn default() -> Self {
        Self {
            worlds: 4,
            sections: 64,
            rebuilds: 8,
            pieces: 8_192,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VoxelCollisionReport3d {
    pub worlds: usize,
    pub sections: usize,
    pub compiled: usize,
    pub rebuilt: usize,
    pub removed: usize,
    pub pieces: usize,
}

#[derive(Debug, Error)]
pub enum VoxelCollisionError3d {
    #[error("voxel collision {0} budget exceeded")]
    Budget(&'static str),
    #[error("duplicate voxel collision owner {0:?}")]
    DuplicateOwner(EntityId),
    #[error("duplicate voxel collision section {1:?} on owner {0:?}")]
    DuplicateSection(EntityId, SectionCoord),
    #[error("voxel collision owner {0:?} is missing")]
    MissingOwner(EntityId),
    #[error("voxel collision owner {0:?} has unrelated solver state or lost its owned body")]
    Ownership(EntityId),
    #[error("voxel collision scale must be positive and finite")]
    Scale,
    #[error("voxel collision group keys exhausted")]
    Keys,
    #[error(transparent)]
    Geometry(#[from] sindri_voxel::VoxelCollisionError),
    #[error(transparent)]
    Physics(#[from] PhysicsError),
}
