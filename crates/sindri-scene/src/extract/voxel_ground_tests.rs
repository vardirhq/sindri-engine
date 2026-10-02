use super::*;
use crate::VoxelGeneratorDocument;
use sindri_grid::GridCoord;

/// A flat world: the layered generator with no variation lays its surface
/// block at one height everywhere.
fn flat(edits: Vec<VoxelEdit>) -> VoxelWorldComponent {
    let mut component: VoxelWorldComponent = serde_json::from_str("{}").expect("a bare world");
    component.generator = VoxelGeneratorDocument::LayeredTerrain {
        seed: 7,
        base_height: 4,
        height_variation: 0,
        surface_voxel: VoxelBlock::Material(1),
        subsurface_voxel: VoxelBlock::Material(2),
        deep_voxel: VoxelBlock::Material(3),
        subsurface_depth: 2,
    };
    component.edits = edits;
    component
}

fn ground(edits: Vec<VoxelEdit>) -> VoxelGround {
    VoxelGround::of(&flat(edits), None).expect("the world builds")
}

#[test]
#[allow(clippy::cast_precision_loss)]
fn a_column_stands_on_its_highest_block() {
    let ground = ground(Vec::new());
    let (level, height, walkable) = ground.surface(3, -9).expect("the ground is solid");
    assert_eq!(ground.block([3, level, -9]), "1");
    assert_eq!(ground.block([3, level + 1, -9]), "");
    assert!((height - (level as f32 + 1.0)).abs() < f32::EPSILON);
    assert!(walkable);
}

#[test]
fn an_edit_lies_on_top_of_what_was_generated() {
    let flat_level = ground(Vec::new()).surface(0, 0).expect("solid").0;
    let built = ground(vec![VoxelEdit {
        at: [0, flat_level + 1, 0],
        block: VoxelBlock::Material(2),
    }]);
    assert_eq!(built.block([0, flat_level + 1, 0]), "2");
    assert_eq!(built.surface(0, 0).expect("solid").0, flat_level + 1);
    // Next door is untouched.
    assert_eq!(built.surface(1, 0).expect("solid").0, flat_level);

    let dug = ground(vec![VoxelEdit {
        at: [0, flat_level, 0],
        block: VoxelBlock::air(),
    }]);
    assert_eq!(dug.block([0, flat_level, 0]), "");
    assert_eq!(dug.surface(0, 0).expect("solid").0, flat_level - 1);
}

#[test]
fn building_then_removing_leaves_no_edit_behind() {
    let ground = ground(Vec::new());
    let level = ground.surface(5, 5).expect("solid").0;
    let above = [5, level + 1, 5];
    let built = ground
        .edited(&[], above, &VoxelBlock::Material(3))
        .expect("material 3 exists");
    assert_eq!(built.len(), 1);
    let removed = ground
        .edited(&built, above, &VoxelBlock::air())
        .expect("air always exists");
    assert!(
        removed.is_empty(),
        "back to generated is no edit: {removed:?}"
    );

    let unknown = ground.edited(&[], above, &VoxelBlock::Material(9));
    assert!(unknown.is_err());
}

#[test]
fn a_ray_finds_the_face_it_came_in_through() {
    let ground = ground(Vec::new());
    let level = ground.surface(2, 2).expect("solid").0;
    #[allow(clippy::cast_precision_loss)]
    let above = glam::Vec3::new(2.5, level as f32 + 10.5, 2.5);
    let hit = ground
        .raycast(above, glam::Vec3::NEG_Y, 64.0)
        .expect("straight down meets the ground");
    assert_eq!(hit.cell, [2, level, 2]);
    assert_eq!(hit.normal, [0, 1, 0]);
    assert_eq!(hit.before(), [2, level + 1, 2]);

    // Diagonally down and across lands on a top too, never inside a block.
    let slanted = ground
        .raycast(above, glam::Vec3::new(0.7, -1.0, 0.4), 64.0)
        .expect("a slanted ray meets the ground");
    assert_eq!(slanted.cell[1], level);
    assert_eq!(slanted.normal, [0, 1, 0]);

    assert!(ground.raycast(above, glam::Vec3::Y, 64.0).is_none());
}

#[test]
fn surfaces_are_what_navigation_reads() {
    let level = ground(Vec::new()).surface(0, 0).expect("solid").0;
    let ground = ground(vec![
        VoxelEdit {
            at: [1, level + 1, 0],
            block: VoxelBlock::Material(1),
        },
        VoxelEdit {
            at: [2, level, 0],
            block: VoxelBlock::air(),
        },
    ]);
    let surfaces = ground.surfaces([0, 0], [3, 0]);
    let at = |x| GridCoord::new(x, 0);
    assert!(surfaces.step_is_walkable(at(0), at(1), 1.0));
    assert!(!surfaces.step_is_walkable(at(0), at(1), 0.5));
    assert_eq!(surfaces.top(at(1)).map(|top| top.z), Some(level + 1));
    assert_eq!(surfaces.top(at(2)).map(|top| top.z), Some(level - 1));
}
