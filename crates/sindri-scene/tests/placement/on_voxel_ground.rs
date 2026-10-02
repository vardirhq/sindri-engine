//! A grid of boxes whose ground is a voxel world: things stand on what was
//! generated, and on what was built on it since.

use serde_json::json;
use sindri_core::{SCENE_FORMAT_VERSION, SceneDocument, SceneEntity, Transform3D, World};
use sindri_scene::{GridSurfaces, SceneExtractor, resolve_grid_placements};

use crate::support::{id, placed, transform_of};

/// Flat generated ground whose top block is at level 2, so its surface is at
/// height 3, with a block built on it at column 1, row 1.
fn voxel_world_with(entities: Vec<SceneEntity>) -> (World, SceneExtractor) {
    let mut floor = SceneEntity::new(id("floor"));
    floor.transform_3d = Some(Transform3D {
        position: [3.0, 0.0, -2.0],
        ..Transform3D::default()
    });
    floor.components.insert(
        "sindri.tile_grid".to_owned(),
        json!({
            "columns": 16, "rows": 16, "cell_size": [1.0, 1.0],
            "cell_height": 1.0, "space": "solid"
        }),
    );
    floor.components.insert(
        "sindri.voxel_world".to_owned(),
        json!({
            "generator": {
                "kind": "layered_terrain", "base_height": 2, "height_variation": 0
            },
            "edits": [{ "at": [1, 3, 1], "block": 2 }]
        }),
    );
    let mut all = vec![floor];
    all.extend(entities);
    let document = SceneDocument {
        format_version: SCENE_FORMAT_VERSION,
        metadata: sindri_core::SceneMetadata::default(),
        entities: all,
    };
    let extractor = SceneExtractor::new().expect("the schemas register");
    let mut world = World::default();
    sindri_core::LoadedScenes::new()
        .enter_keeping_identities(&mut world, "test", &document)
        .expect("the scene loads");
    (world, extractor)
}

#[test]
fn a_prop_stands_on_generated_ground_and_on_a_built_block() {
    let (mut world, extractor) = voxel_world_with(vec![
        placed("on-ground", &json!([4, 2])),
        placed("on-block", &json!([1, 1])),
    ]);
    let mut surfaces = GridSurfaces::default();
    let resolved = resolve_grid_placements(&mut world, extractor.components(), None, &mut surfaces)
        .expect("placements resolve");
    assert_eq!(resolved, 2);
    let near = |name: &str, expected: [f32; 3]| {
        let at = transform_of(&world, name).position;
        assert!(
            at.iter()
                .zip(expected)
                .all(|(got, want)| (got - want).abs() < 1e-5),
            "{name} is at {at:?}, not {expected:?}"
        );
    };
    near("on-ground", [7.0, 3.0, 0.0]);
    near("on-block", [4.0, 4.0, -1.0]);

    // The world is read once however many things stand on it.
    resolve_grid_placements(&mut world, extractor.components(), None, &mut surfaces)
        .expect("placements resolve again");
    assert_eq!(surfaces.derivations(), 1);
}

#[test]
fn a_walker_stands_on_whatever_column_its_script_moved_it_to() {
    let mut walker = SceneEntity::new(id("walker"));
    walker.transform_3d = Some(Transform3D {
        position: [4.1, 0.0, -0.9],
        ..Transform3D::default()
    });
    walker.components.insert(
        "sindri.grid.placement".to_owned(),
        json!({ "grid": "floor" }),
    );
    let (mut world, extractor) = voxel_world_with(vec![walker]);
    let mut surfaces = GridSurfaces::default();
    resolve_grid_placements(&mut world, extractor.components(), None, &mut surfaces)
        .expect("placements resolve");
    let standing = transform_of(&world, "walker").position;
    assert!(
        (standing[1] - 4.0).abs() < 1e-5,
        "on top of the built block: {standing:?}"
    );
}
