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
    let height = before.squares[0].height;
    let draw = |map: &mut MapWorld, level| {
        let mut batches = Vec::new();
        push_map(
            map,
            Mat4::IDENTITY,
            above(0.5, -0.5, 1.0, 0.0),
            0,
            Some((level, TextureId::new(2), UvRect::FULL)),
            &[],
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

#[test]
fn a_square_reads_the_normal_map_named_for_its_texture() {
    let world_with = |normal_maps: &str| {
        let component: VoxelWorldComponent = serde_json::from_str(&format!(
            r#"{{
            "view":"map", "generator": {{"kind":"layered_terrain", "base_height":4,
            "height_variation":0}}, "materials":[
            {{"voxel":1,"top":"dry","side":"dry","bottom":"dry"}},
            {{"voxel":2,"top":"dry","side":"dry","bottom":"dry"}},
            {{"voxel":3,"top":"dry","side":"dry","bottom":"dry"}}],
            "normal_maps": {normal_maps}
        }}"#
        ))
        .unwrap();
        let mut world = World::default();
        world.spawn(sindri_core::EntityData {
            components: [(
                "sindri.voxel_world".into(),
                serde_json::to_value(&component).unwrap(),
            )]
            .into(),
            transform_3d: Some(sindri_core::Transform3D::default()),
            ..sindri_core::EntityData::default()
        });
        world
    };
    let mut textures = TextureBindings::new();
    textures.bind("dry", TextureId::new(1));
    textures.bind("dry_n", TextureId::new(7));
    let drawn = |world: &World| {
        let extractor = SceneExtractor::new().unwrap();
        let mut batches = Vec::new();
        extractor
            .push_voxel_maps(
                world,
                Some(above(0.5, -0.5, 1.0, 0.0)),
                &textures,
                None,
                &mut batches,
            )
            .unwrap();
        assert!(!batches.is_empty(), "the map draws");
        batches
            .iter()
            .map(|square| square.normal_map)
            .collect::<Vec<_>>()
    };
    assert!(
        drawn(&world_with(r#"{"dry": "dry_n"}"#))
            .iter()
            .all(|normal_map| *normal_map == Some(TextureId::new(7)))
    );
    assert!(
        drawn(&world_with(r#"{"rock": "dry_n"}"#))
            .iter()
            .all(Option::is_none),
        "another texture's normal map is not borrowed"
    );
    assert!(
        drawn(&world_with(r#"{"dry": "unbound_n"}"#))
            .iter()
            .all(Option::is_none),
        "an unbound normal map lights by distance alone"
    );
}

/// With `map_water`, a pond is drawn through the water shader: paler at its
/// edge than in its deep middle, and its edge knows where the land is.
#[test]
fn map_water_has_depth_and_a_shore() {
    let mut component: VoxelWorldComponent = serde_json::from_str(
        r#"{
        "view":"map", "generator": {"kind":"layered_terrain", "base_height":8,
        "height_variation":0}, "materials":[
        {"voxel":1,"top":"dry","side":"dry","bottom":"dry"},
        {"voxel":2,"top":"water","side":"water","bottom":"water"},
        {"voxel":3,"top":"dry","side":"dry","bottom":"dry"}],
        "map_water": {"blocks": ["2"], "depth": 4}
    }"#,
    )
    .unwrap();
    let level = VoxelGround::of(&component, None)
        .unwrap()
        .surface(0, 0)
        .unwrap()
        .0;
    // A pond of nine squares, one block deep but three in its middle.
    for row in 0..3 {
        for column in 0..3 {
            let deep = if (column, row) == (1, 1) { 3 } else { 1 };
            for down in 0..deep {
                component.edits.push(crate::VoxelEdit {
                    at: [column, level - down, row],
                    block: crate::VoxelBlock::Material(2),
                });
            }
        }
    }
    let mut world = World::default();
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
    let mut cache = BTreeMap::new();
    refresh(&mut cache, &world, entity, &component, &textures, None).unwrap();
    let map = cache.get_mut(&entity).unwrap();
    let pond = chunk(map, 0, 0);
    assert_eq!(pond.water.len(), 9, "the pond and nothing else is water");
    let middle = pond.water[&(1, 1)];
    let corner = pond.water[&(0, 0)];
    assert!(middle.params[0] > corner.params[0], "deeper in the middle");
    assert!(middle.params[1].abs() < 0.5, "no land beside the middle");
    // North 1, west 8, north-east 16, south-west 64, north-west 128.
    assert!((corner.params[1] - (1.0 + 8.0 + 16.0 + 64.0 + 128.0)).abs() < 0.5);
    assert!(pond.wet[&(3, 1)] < 0.75, "the bank beside it is wet");
    assert!(pond.wet[&(3, 3)] > pond.wet[&(3, 1)], "its corner less so");
    assert!(
        !pond.wet.contains_key(&(6, 6)),
        "the land away from it is dry"
    );
    let mut batches = Vec::new();
    push_map(
        map,
        Mat4::IDENTITY,
        above(1.5, -1.5, 3.0, 0.0),
        0,
        None,
        &[],
        &mut batches,
    )
    .unwrap();
    let water: Vec<_> = batches
        .iter()
        .filter(|draw| draw.texture == TextureId::new(2))
        .collect();
    assert_eq!(water.len(), 9);
    assert!(
        water.iter().all(|draw| draw.shader.is_some()),
        "drawn as water"
    );
    assert!(
        batches
            .iter()
            .filter(|draw| draw.texture == TextureId::new(1))
            .all(|draw| draw.shader.is_none()),
        "the land is not"
    );
    // A picture two tall standing on the bank north of the pond is mirrored
    // in the water in front of it, and only there.
    let standing = Mat4::from_translation(Vec3::new(1.5, 0.5, 1.0))
        * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)
        * Mat4::from_scale(Vec3::new(1.0, 2.0, 1.0));
    let mut mirrored = Vec::new();
    map.reflections(standing, TextureId::new(9), UvRect::FULL, &mut mirrored);
    assert_eq!(
        mirrored.len(),
        2,
        "over the pond's first two rows, not the bank"
    );
    assert!(mirrored.iter().all(|draw| draw.shader.is_some()));
    let far_away = Mat4::from_translation(Vec3::new(40.5, 0.5, 1.0))
        * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
    let mut none = Vec::new();
    map.reflections(far_away, TextureId::new(9), UvRect::FULL, &mut none);
    assert!(none.is_empty(), "nothing is mirrored on dry land");
}
