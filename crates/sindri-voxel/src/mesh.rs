use crate::{
    DefaultVoxelMaterials, FaceOcclusion, RenderClass, SECTION_EDGE, SectionCoord, VOXEL_STEPS,
    VoxelCoord, VoxelId, VoxelMaterialSource, VoxelShape, VoxelSource,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum VoxelFace {
    Left,
    Right,
    Bottom,
    Top,
    Back,
    Front,
}

impl VoxelFace {
    const ALL: [Self; 6] = [
        Self::Left,
        Self::Right,
        Self::Bottom,
        Self::Top,
        Self::Back,
        Self::Front,
    ];

    const fn offset(self) -> (i32, i32, i32) {
        match self {
            Self::Left => (-1, 0, 0),
            Self::Right => (1, 0, 0),
            Self::Bottom => (0, -1, 0),
            Self::Top => (0, 1, 0),
            Self::Back => (0, 0, -1),
            Self::Front => (0, 0, 1),
        }
    }

    const fn normal(self) -> [i8; 3] {
        match self {
            Self::Left => [-1, 0, 0],
            Self::Right => [1, 0, 0],
            Self::Bottom => [0, -1, 0],
            Self::Top => [0, 1, 0],
            Self::Back => [0, 0, -1],
            Self::Front => [0, 0, 1],
        }
    }

    /// The axis this face looks along, and whether it looks along it
    /// positively.
    const fn axis(self) -> (usize, bool) {
        match self {
            Self::Left => (0, false),
            Self::Right => (0, true),
            Self::Bottom => (1, false),
            Self::Top => (1, true),
            Self::Back => (2, false),
            Self::Front => (2, true),
        }
    }

    /// The four corners of this face of `shape` in the voxel at `local`, in
    /// sixteenths of a voxel from the section's corner.
    fn corners(self, local: [u8; 3], shape: VoxelShape) -> [[u16; 3]; 4] {
        let step = u16::from(VOXEL_STEPS);
        let low = [0, 1, 2].map(|axis| u16::from(local[axis]) * step + u16::from(shape.min[axis]));
        let high = [0, 1, 2].map(|axis| u16::from(local[axis]) * step + u16::from(shape.max[axis]));
        let [x, y, z] = low;
        let [right, top, front] = high;
        match self {
            Self::Left => [[x, y, z], [x, y, front], [x, top, front], [x, top, z]],
            Self::Right => [
                [right, y, front],
                [right, y, z],
                [right, top, z],
                [right, top, front],
            ],
            Self::Bottom => [[x, y, front], [x, y, z], [right, y, z], [right, y, front]],
            Self::Top => [
                [x, top, z],
                [x, top, front],
                [right, top, front],
                [right, top, z],
            ],
            Self::Back => [[right, y, z], [x, y, z], [x, top, z], [right, top, z]],
            Self::Front => [
                [x, y, front],
                [right, y, front],
                [right, top, front],
                [x, top, front],
            ],
        }
    }
}

/// Whether `neighbour`, in the next voxel past this `face` of `shape`, hides
/// that face.
///
/// It has to meet it -- this shape reaching the wall between the two voxels
/// and the neighbour's reaching it from the other side -- and it has to cover
/// all of it. A slab's top is in the middle of its voxel, so nothing above
/// hides it; a post against a wall hides only a sliver of the wall, which is
/// to say none of it can be dropped.
fn hidden_by(face: VoxelFace, shape: VoxelShape, neighbour: VoxelShape) -> bool {
    let (axis, positive) = face.axis();
    let (mine, theirs) = if positive {
        (shape.max[axis] == VOXEL_STEPS, neighbour.min[axis] == 0)
    } else {
        (shape.min[axis] == 0, neighbour.max[axis] == VOXEL_STEPS)
    };
    mine && theirs
        && (0..3).filter(|other| *other != axis).all(|other| {
            neighbour.min[other] <= shape.min[other] && neighbour.max[other] >= shape.max[other]
        })
}

/// One renderer-consumable vertex using section-local coordinates.
///
/// UV values are unit-square corners. A render bridge maps the semantic
/// material and face identities to an atlas, array texture, or custom shader.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockVertex {
    /// Where the corner is, in sixteenths of a voxel ([`VOXEL_STEPS`]) from
    /// the section's corner, so a block smaller than its voxel has corners
    /// inside it.
    pub position: [u16; 3],
    pub normal: [i8; 3],
    pub uv: [u8; 2],
    pub material: VoxelId,
    pub face: VoxelFace,
    /// 0..=3 open-neighbour samples around this corner, used for mesh-time AO.
    pub ambient_occlusion: u8,
    /// Which voxel of the section the face belongs to, so its look can be
    /// chosen by where it is.
    pub cell: [u8; 3],
    /// Whether something stands on the voxel: a block that hides faces is in
    /// the voxel above it. Grass under a block is no longer grass.
    pub covered: bool,
}

impl BlockVertex {
    /// The corner's position in voxels from the section's corner.
    #[must_use]
    pub fn voxel_position(&self) -> [f32; 3] {
        self.position
            .map(|value| f32::from(value) / f32::from(VOXEL_STEPS))
    }
}

/// Indexed geometry for one render class.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlockMeshPart {
    pub vertices: Vec<BlockVertex>,
    pub indices: Vec<u32>,
}

impl BlockMeshPart {
    #[must_use]
    pub fn face_count(&self) -> usize {
        self.indices.len() / 6
    }

    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    fn push_face(
        &mut self,
        local: [u8; 3],
        placed: Placed,
        face: VoxelFace,
        ambient_occlusion: [u8; 4],
    ) {
        // Texture images use a top-left origin. Face corners begin along the
        // bottom edge, so V=1 belongs to the first two vertices.
        const UVS: [[u8; 2]; 4] = [[0, 1], [1, 1], [1, 0], [0, 0]];
        let base =
            u32::try_from(self.vertices.len()).expect("one section mesh fits in u32 indices");
        for ((position, uv), ambient_occlusion) in face
            .corners(local, placed.shape)
            .into_iter()
            .zip(UVS)
            .zip(ambient_occlusion)
        {
            self.vertices.push(BlockVertex {
                position,
                normal: face.normal(),
                uv,
                material: placed.voxel,
                face,
                ambient_occlusion,
                cell: local,
                covered: placed.covered,
            });
        }
        if ambient_occlusion[0] + ambient_occlusion[2] > ambient_occlusion[1] + ambient_occlusion[3]
        {
            self.indices.extend_from_slice(&[
                base,
                base + 1,
                base + 3,
                base + 1,
                base + 2,
                base + 3,
            ]);
        } else {
            self.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
}

/// What one voxel's faces are meshed as.
#[derive(Clone, Copy)]
struct Placed {
    voxel: VoxelId,
    shape: VoxelShape,
    covered: bool,
}

/// World-space bounds of a compiled section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SectionBounds {
    pub min: VoxelCoord,
    pub max_exclusive: VoxelCoord,
}

impl SectionBounds {
    #[must_use]
    pub fn from_section(section: SectionCoord) -> Self {
        let min = section.min_voxel();
        Self {
            min,
            max_exclusive: VoxelCoord::new(
                min.x + SECTION_EDGE,
                min.y + SECTION_EDGE,
                min.z + SECTION_EDGE,
            ),
        }
    }
}

/// CPU-side compiled block geometry, split into renderer passes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockMesh {
    pub section: SectionCoord,
    pub bounds: SectionBounds,
    pub opaque: BlockMeshPart,
    pub cutout: BlockMeshPart,
    pub transparent: BlockMeshPart,
}

impl BlockMesh {
    #[must_use]
    pub fn face_count(&self) -> usize {
        self.opaque.face_count() + self.cutout.face_count() + self.transparent.face_count()
    }

    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.opaque.triangle_count()
            + self.cutout.triangle_count()
            + self.transparent.triangle_count()
    }

    #[must_use]
    pub const fn part(&self, render_class: RenderClass) -> &BlockMeshPart {
        match render_class {
            RenderClass::Opaque => &self.opaque,
            RenderClass::Cutout => &self.cutout,
            RenderClass::Transparent => &self.transparent,
        }
    }

    fn part_mut(&mut self, render_class: RenderClass) -> &mut BlockMeshPart {
        match render_class {
            RenderClass::Opaque => &mut self.opaque,
            RenderClass::Cutout => &mut self.cutout,
            RenderClass::Transparent => &mut self.transparent,
        }
    }
}

/// Compiles exposed block faces using the default all-opaque material mapping.
#[must_use]
pub fn mesh_block_section(source: &impl VoxelSource, section: SectionCoord) -> BlockMesh {
    mesh_block_section_with_materials(source, &DefaultVoxelMaterials, section)
}

/// Compiles one section using game-provided material and face-occlusion policy.
///
/// Neighbours are queried through `VoxelSource`, including coordinates outside
/// the target section. Geometry remains section-local while `SectionBounds`
/// provides the world-space extent used by render caches and culling.
#[must_use]
pub fn mesh_block_section_with_materials(
    source: &impl VoxelSource,
    materials: &impl VoxelMaterialSource,
    section: SectionCoord,
) -> BlockMesh {
    let min = section.min_voxel();
    let mut mesh = BlockMesh {
        section,
        bounds: SectionBounds::from_section(section),
        opaque: BlockMeshPart::default(),
        cutout: BlockMeshPart::default(),
        transparent: BlockMeshPart::default(),
    };
    for y in 0..SECTION_EDGE {
        for z in 0..SECTION_EDGE {
            for x in 0..SECTION_EDGE {
                let voxel = VoxelCoord::new(min.x + x, min.y + y, min.z + z);
                let voxel_id = source.voxel(voxel);
                if voxel_id.is_air() {
                    continue;
                }
                let material = materials.material(voxel_id);
                let local = [
                    u8::try_from(x).expect("section x fits in u8"),
                    u8::try_from(y).expect("section y fits in u8"),
                    u8::try_from(z).expect("section z fits in u8"),
                ];
                let above = source.voxel(VoxelCoord::new(voxel.x, voxel.y + 1, voxel.z));
                let placed = Placed {
                    voxel: voxel_id,
                    shape: material.shape,
                    covered: !above.is_air()
                        && materials.material(above).face_occlusion != FaceOcclusion::None,
                };
                for face in VoxelFace::ALL {
                    let (dx, dy, dz) = face.offset();
                    let neighbour_coord = VoxelCoord::new(voxel.x + dx, voxel.y + dy, voxel.z + dz);
                    let neighbour = source.voxel(neighbour_coord);
                    let visible = neighbour.is_air() || {
                        let beside = materials.material(neighbour);
                        !(beside.blocks_face(neighbour, voxel_id)
                            && hidden_by(face, material.shape, beside.shape))
                    };
                    if visible {
                        let ambient_occlusion =
                            face_ambient_occlusion(source, materials, voxel, voxel_id, face, local);
                        mesh.part_mut(material.render_class).push_face(
                            local,
                            placed,
                            face,
                            ambient_occlusion,
                        );
                    }
                }
            }
        }
    }
    mesh
}

fn face_ambient_occlusion(
    source: &impl VoxelSource,
    materials: &impl VoxelMaterialSource,
    voxel: VoxelCoord,
    voxel_id: VoxelId,
    face: VoxelFace,
    local: [u8; 3],
) -> [u8; 4] {
    let normal = face.normal();
    let step = u16::from(VOXEL_STEPS);
    face.corners(local, VoxelShape::FULL).map(|corner| {
        let mut tangents = [[0_i32; 3]; 2];
        let mut next = 0;
        for axis in 0..3 {
            if normal[axis] == 0 {
                tangents[next][axis] = if corner[axis] == u16::from(local[axis]) * step {
                    -1
                } else {
                    1
                };
                next += 1;
            }
        }
        let base = [
            voxel.x + i32::from(normal[0]),
            voxel.y + i32::from(normal[1]),
            voxel.z + i32::from(normal[2]),
        ];
        let occupied = |offset: [i32; 3]| {
            let neighbour = source.voxel(VoxelCoord::new(
                base[0] + offset[0],
                base[1] + offset[1],
                base[2] + offset[2],
            ));
            !neighbour.is_air() && {
                let beside = materials.material(neighbour);
                beside.blocks_face(neighbour, voxel_id) && beside.shape.is_full()
            }
        };
        let side_a = occupied(tangents[0]);
        let side_b = occupied(tangents[1]);
        let corner_offset = [
            tangents[0][0] + tangents[1][0],
            tangents[0][1] + tangents[1][1],
            tangents[0][2] + tangents[1][2],
        ];
        let diagonal = occupied(corner_offset);
        if side_a && side_b {
            0
        } else {
            3 - u8::from(side_a) - u8::from(side_b) - u8::from(diagonal)
        }
    })
}
