use crate::VoxelId;

/// Rendering pass used for geometry produced from a voxel material.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RenderClass {
    Opaque,
    Cutout,
    Transparent,
}

/// How a full-cube voxel hides a neighbouring voxel face.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FaceOcclusion {
    /// Hide every face touching this voxel.
    Solid,
    /// Hide only a face belonging to the same voxel identity.
    MatchingVoxel,
    /// Never hide a neighbouring face.
    None,
}

/// How many steps a voxel is divided into along each axis when it describes
/// a shape smaller than itself.
pub const VOXEL_STEPS: u8 = 16;

/// The part of its voxel a block fills, as a box in sixteenths: X across, Y
/// up, Z toward the front, each from zero to [`VOXEL_STEPS`].
///
/// A slab is the bottom half; a post is a thin column in the middle. Most
/// blocks are [`VoxelShape::FULL`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VoxelShape {
    pub min: [u8; 3],
    pub max: [u8; 3],
}

impl VoxelShape {
    pub const FULL: Self = Self {
        min: [0; 3],
        max: [VOXEL_STEPS; 3],
    };

    /// A box from fractions of a voxel, rounded to the nearest step and kept
    /// at least a step thick.
    #[must_use]
    pub fn from_fractions(min: [f32; 3], max: [f32; 3]) -> Self {
        let step = |value: f32| {
            let scaled = (value.clamp(0.0, 1.0) * f32::from(VOXEL_STEPS)).round();
            // Clamped to 0..=16 above, so it fits.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let whole = scaled as u8;
            whole
        };
        let mut shape = Self {
            min: min.map(step),
            max: max.map(step),
        };
        for axis in 0..3 {
            if shape.max[axis] <= shape.min[axis] {
                if shape.min[axis] >= VOXEL_STEPS {
                    shape.min[axis] = VOXEL_STEPS - 1;
                }
                shape.max[axis] = shape.min[axis] + 1;
            }
        }
        shape
    }

    #[must_use]
    pub fn is_full(self) -> bool {
        self == Self::FULL
    }
}

impl Default for VoxelShape {
    fn default() -> Self {
        Self::FULL
    }
}

/// Renderer-independent description of one voxel material.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VoxelMaterial {
    pub render_class: RenderClass,
    pub face_occlusion: FaceOcclusion,
    /// The part of the voxel the block fills.
    pub shape: VoxelShape,
}

impl VoxelMaterial {
    #[must_use]
    pub const fn new(render_class: RenderClass, face_occlusion: FaceOcclusion) -> Self {
        Self {
            render_class,
            face_occlusion,
            shape: VoxelShape::FULL,
        }
    }

    #[must_use]
    pub const fn with_shape(mut self, shape: VoxelShape) -> Self {
        self.shape = shape;
        self
    }

    #[must_use]
    pub const fn opaque() -> Self {
        Self::new(RenderClass::Opaque, FaceOcclusion::Solid)
    }

    #[must_use]
    pub const fn cutout() -> Self {
        Self::new(RenderClass::Cutout, FaceOcclusion::None)
    }

    #[must_use]
    pub const fn transparent() -> Self {
        Self::new(RenderClass::Transparent, FaceOcclusion::MatchingVoxel)
    }

    #[must_use]
    pub const fn blocks_face(self, voxel: VoxelId, neighbour: VoxelId) -> bool {
        match self.face_occlusion {
            FaceOcclusion::Solid => true,
            FaceOcclusion::MatchingVoxel => voxel.value() == neighbour.value(),
            FaceOcclusion::None => false,
        }
    }
}

/// Resolves a compact `VoxelId` into meshing policy.
///
/// Texture handles, atlas regions, shaders, and GPU bindings remain the
/// renderer's responsibility.
pub trait VoxelMaterialSource {
    fn material(&self, voxel: VoxelId) -> VoxelMaterial;
}

/// Baseline mapping used when a game has not supplied material policy.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultVoxelMaterials;

impl VoxelMaterialSource for DefaultVoxelMaterials {
    fn material(&self, _voxel: VoxelId) -> VoxelMaterial {
        VoxelMaterial::opaque()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_materials_express_expected_occlusion() {
        let glass = VoxelId::new(3);
        assert!(VoxelMaterial::opaque().blocks_face(glass, VoxelId::new(4)));
        assert!(VoxelMaterial::transparent().blocks_face(glass, glass));
        assert!(!VoxelMaterial::transparent().blocks_face(glass, VoxelId::new(4)));
        assert!(!VoxelMaterial::cutout().blocks_face(glass, glass));
    }
}
