use glam::{Mat4, Vec3};

use super::*;

fn above(x: f32, y: f32, half: f32, roll: f32) -> ResolvedCamera {
    let up = Vec3::new(-roll.sin(), roll.cos(), 0.0);
    let view =
        glam::camera::rh::view::look_at_mat4(Vec3::new(x, y, 10.0), Vec3::new(x, y, 0.0), up);
    let projection =
        glam::camera::rh::proj::directx::orthographic(-half, half, -half, half, 0.1, 100.0);
    ResolvedCamera {
        view,
        view_projection: projection * view,
        framed_half_height: half,
    }
}

#[test]
fn the_columns_in_view_are_those_under_the_camera_padded_by_one() {
    let (min, max) = visible_columns(Mat4::IDENTITY, above(10.0, -20.0, 4.0, 0.0)).unwrap();
    // X from 6 to 14 and Y from -24 to -16, which is rows 16 to 24.
    assert_eq!(min, [5, 15]);
    assert_eq!(max, [15, 25]);
}

#[test]
fn a_turned_camera_sees_the_corners_it_swings_over() {
    let (min, max) = visible_columns(
        Mat4::IDENTITY,
        above(0.0, 0.0, 4.0, std::f32::consts::FRAC_PI_4),
    )
    .unwrap();
    // A square half four across turned an eighth reaches about 5.66 out.
    assert_eq!(min, [-7, -7]);
    assert_eq!(max, [6, 6]);
}

#[test]
fn the_map_moves_with_its_entity() {
    let shifted = Mat4::from_translation(Vec3::new(100.0, 0.0, 0.0));
    let (min, max) = visible_columns(shifted, above(100.0, 0.0, 2.0, 0.0)).unwrap();
    assert_eq!(min, [-3, -3]);
    assert_eq!(max, [3, 3]);
}

#[test]
fn a_camera_pulled_far_back_sees_a_bounded_map() {
    let (min, max) = visible_columns(Mat4::IDENTITY, above(0.0, 0.0, 100_000.0, 0.0)).unwrap();
    for axis in 0..2 {
        assert!(max[axis] - min[axis] <= MOST_CHUNKS_ACROSS * CHUNK);
    }
}

#[test]
fn a_flood_draws_below_its_level_and_reuses_the_dry_map_chunks() {
    let mut world = World::default();
    let mut component: VoxelWorldComponent = serde_json::from_str(
        r#"{
        "view":"map", "generator": {"kind":"layered_terrain", "base_height":4,
        "height_variation":0}, "materials":[
        {"voxel":1,"top":"dry","side":"dry","bottom":"dry"},
        {"voxel":2,"top":"water","side":"water","bottom":"water"},
        {"voxel":3,"top":"rock","side":"rock","bottom":"rock"}]
    }"#,
    )
    .unwrap();
    let entity = world.spawn(sindri_core::EntityData {
        components: [(
            "sindri.voxel_world".into(),
            serde_json::to_value(&component).unwrap(),
        )]
        .into(),
        ..sindri_core::EntityData::default()
    });
    let mut textures = TextureBindings::new();
    textures.bind("dry", TextureId::new(1));
    textures.bind("water", TextureId::new(2));
    textures.bind("rock", TextureId::new(3));
    let mut cache = BTreeMap::new();
    refresh(&mut cache, &world, entity, &component, &textures, None).unwrap();
    let map = cache.get_mut(&entity).unwrap();
    let before = chunk(map, 0, 0);
    let height = before[0].height;
    let draw = |map: &mut MapWorld, level| {
        let mut batches = Vec::new();
        push_map(
            map,
            Mat4::IDENTITY,
            above(0.5, -0.5, 1.0, 0.0),
            0,
            Some((level, TextureId::new(2), UvRect::FULL)),
            &mut batches,
        )
        .unwrap();
        batches
    };
    assert!(
        draw(map, height)
            .iter()
            .all(|square| square.texture == TextureId::new(1)),
        "equal height stays dry"
    );
    assert!(
        draw(map, height + 1.0)
            .iter()
            .all(|square| square.texture == TextureId::new(2)),
        "below water draws water"
    );
    component.map_flood = Some(crate::VoxelMapFlood {
        level: height + 1.0,
        block: crate::VoxelBlock::Material(2),
    });
    world.get_mut(entity).unwrap().components.insert(
        "sindri.voxel_world".into(),
        serde_json::to_value(&component).unwrap(),
    );
    refresh(&mut cache, &world, entity, &component, &textures, None).unwrap();
    assert!(
        Rc::ptr_eq(&before, &chunk(cache.get_mut(&entity).unwrap(), 0, 0)),
        "flood does not rebuild terrain"
    );
    component.map_flood = None;
    component.edits.push(crate::VoxelEdit {
        at: [0, 10, 0],
        block: crate::VoxelBlock::Material(3),
    });
    world.get_mut(entity).unwrap().components.insert(
        "sindri.voxel_world".into(),
        serde_json::to_value(&component).unwrap(),
    );
    refresh(&mut cache, &world, entity, &component, &textures, None).unwrap();
    assert!(
        !Rc::ptr_eq(&before, &chunk(cache.get_mut(&entity).unwrap(), 0, 0)),
        "block edits still rebuild terrain"
    );
}
