use super::*;
use crate::{ResidencyConfig, SectionCoord, VoxelCoord, VoxelSource, VoxelWorld};

fn solid(voxel: VoxelId) -> Option<VoxelShape> {
    (!voxel.is_air()).then_some(VoxelShape::FULL)
}

#[test]
fn air_is_never_classified_and_solid_sections_merge_across_materials() {
    assert!(
        compile_section_collision(&VoxelSection::default(), |_| panic!("air policy"))
            .unwrap()
            .is_empty()
    );
    let mut section = VoxelSection::uniform(VoxelId::new(1));
    section.set(LocalVoxelCoord::new(7, 8, 9), VoxelId::new(2));
    let classified = std::cell::Cell::new(0_usize);
    assert_eq!(
        compile_section_collision(&section, |voxel| {
            classified.set(classified.get() + 1);
            solid(voxel)
        })
        .unwrap(),
        vec![SectionCollisionBox {
            min: [0; 3],
            max: [256; 3],
        }]
    );
    assert_eq!(classified.get(), SECTION_VOLUME);
}

#[test]
fn merged_boxes_cover_every_occupied_cell_once_and_never_cover_air() {
    for seed in 0_u32..24 {
        let mut section = VoxelSection::default();
        for y in 0..EDGE {
            for z in 0..EDGE {
                for x in 0..EDGE {
                    let hash =
                        u32::from(x) * 17 + u32::from(y) * 31 + u32::from(z) * 47 + seed * 53;
                    if hash % 11 < 7 {
                        section.set(LocalVoxelCoord::new(x, y, z), VoxelId::new(1));
                    }
                }
            }
        }
        let boxes = compile_section_collision(&section, solid).unwrap();
        assert_eq!(boxes, compile_section_collision(&section, solid).unwrap());
        assert!(boxes.len() <= SECTION_VOLUME);
        let mut covered = [0_u8; SECTION_VOLUME];
        for bounds in boxes {
            let min = bounds.min.map(|axis| u8::try_from(axis / 16).unwrap());
            let max = bounds.max.map(|axis| u8::try_from(axis / 16).unwrap());
            for cell in cells(min, max) {
                assert!(!section.get(cell).is_air());
                covered[cell.index()] += 1;
                assert_eq!(covered[cell.index()], 1);
            }
        }
        for cell in cells([0; 3], [EDGE; 3]) {
            assert_eq!(covered[cell.index()], u8::from(!section.get(cell).is_air()));
        }
    }
}

#[test]
fn partial_shapes_and_noncolliding_blocks_keep_exact_occupancy_and_bound_output() {
    let slab = VoxelShape {
        min: [0; 3],
        max: [16, 8, 16],
    };
    let post = VoxelShape {
        min: [6, 0, 6],
        max: [10, 16, 10],
    };
    let mut section = VoxelSection::default();
    section.set(LocalVoxelCoord::new(1, 2, 3), VoxelId::new(1));
    section.set(LocalVoxelCoord::new(2, 2, 3), VoxelId::new(2));
    section.set(LocalVoxelCoord::new(3, 2, 3), VoxelId::new(3));
    let policy = |voxel: VoxelId| match voxel.value() {
        1 => Some(slab),
        2 => Some(post),
        _ => None,
    };
    assert_eq!(
        compile_section_collision(&section, policy).unwrap(),
        vec![
            SectionCollisionBox {
                min: [16, 32, 48],
                max: [32, 40, 64]
            },
            SectionCollisionBox {
                min: [38, 32, 54],
                max: [42, 48, 58]
            },
        ]
    );
    let worst = VoxelSection::uniform(VoxelId::new(1));
    assert_eq!(
        compile_section_collision(&worst, |_| Some(post))
            .unwrap()
            .len(),
        SECTION_VOLUME
    );
}

#[test]
fn invalid_policy_shapes_return_typed_errors() {
    let voxel = VoxelId::new(4);
    for shape in [
        VoxelShape {
            min: [0; 3],
            max: [0, 16, 16],
        },
        VoxelShape {
            min: [9, 0, 0],
            max: [8, 16, 16],
        },
        VoxelShape {
            min: [0; 3],
            max: [17, 16, 16],
        },
    ] {
        assert_eq!(
            compile_section_collision(&VoxelSection::uniform(voxel), |_| Some(shape)),
            Err(VoxelCollisionError { voxel, shape })
        );
    }
}

struct Air;
impl VoxelSource for Air {
    fn voxel(&self, _: VoxelCoord) -> VoxelId {
        VoxelId::AIR
    }
}

#[test]
fn edited_negative_sections_recompile_after_unload_without_sampling_unresident_worlds() {
    let mut world = VoxelWorld::new(Air, ResidencyConfig::new(0, 0, 0, 0));
    let section = SectionCoord::new(-1, -1, -1);
    world.move_focus(section);
    let at = VoxelCoord::new(-1, -2, -3);
    world.set_voxel(at, VoxelId::new(1));
    let expected = vec![SectionCollisionBox {
        min: [240, 224, 208],
        max: [256, 240, 224],
    }];
    assert_eq!(
        compile_section_collision(world.resident(section).unwrap(), solid).unwrap(),
        expected
    );
    world.move_focus(SectionCoord::default());
    assert!(world.resident(section).is_none());
    world.move_focus(section);
    assert_eq!(
        compile_section_collision(world.resident(section).unwrap(), solid).unwrap(),
        expected
    );
    world.set_voxel(at, VoxelId::AIR);
    assert!(
        compile_section_collision(world.resident(section).unwrap(), solid)
            .unwrap()
            .is_empty()
    );
}
