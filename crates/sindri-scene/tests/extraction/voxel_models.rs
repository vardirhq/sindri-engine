//! Modeled voxel blocks share the section lifecycle with textured boxes.

use glam::Mat4;
use sindri_render::{FrameCommand, look_at, orthographic_projection};
use sindri_scene::{SceneExtractor, SceneRuntime, TextureBindings, ViewCamera};

use crate::support::{VIEWPORT, scene, world_from};
use crate::voxel_world::{edited, uploads};

fn vent_model(depth: f32) -> std::sync::Arc<sindri_render::RenderModel> {
    use sindri_render::{
        ModelData, ModelGeometry, ModelHierarchyNode, ModelIndexData, ModelVertex, RenderModel,
    };

    std::sync::Arc::new(
        RenderModel::new(ModelData {
            nodes: vec![ModelHierarchyNode {
                name: None,
                local_transform: Mat4::from_translation(glam::Vec3::new(0.0, 0.0, depth)),
                children: vec![],
                mesh: Some(0),
            }],
            roots: vec![0],
            meshes: vec![vec![ModelGeometry {
                vertices: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
                    .into_iter()
                    .map(|position| ModelVertex {
                        position,
                        normal: [0.0, 0.0, 1.0],
                        uv: [0.0; 2],
                    })
                    .collect(),
                indices: ModelIndexData::U16(vec![0, 1, 2]),
                material: None,
            }]],
            ..ModelData::default()
        })
        .unwrap(),
    )
}

fn fixture() -> (
    sindri_core::World,
    sindri_scene::TileSetBindings,
    serde_json::Value,
) {
    let mut world = world_from(&scene(
        r#",
        {"id":"terrain","transform_3d":{"position":[2,0,0],"scale":[0.5,0.5,0.5]},
         "components":{"sindri.voxel_world":{"generator":{"kind":"built"},
         "blocks":"blocks.tileset","render_radius":1,"vertical_radius":0}}}"#,
    ));
    let edits = serde_json::json!([
        {"at":[14,0,0],"block":"box"}, {"at":[15,0,0],"block":"vent"},
        {"at":[16,0,0],"block":"vent"}, {"at":[17,0,0],"block":"vent"}
    ]);
    edited(&mut world, edits.clone());
    let mut sets = sindri_scene::TileSetBindings::new();
    let face = serde_json::json!({"sprite":"box.png","size":[1,1]});
    let faces = serde_json::json!({"top":face,"east":face,"south":face});
    let tile_set = serde_json::json!({
        "format_version": 1, "tiles": {
            "box": {"faces": faces},
            "vent": {"model": "vent.glb", "faces": faces}
        }
    });
    sets.bind(
        "blocks.tileset",
        sindri_core::TileSetDocument::from_json(&tile_set.to_string()).unwrap(),
    )
    .unwrap();
    (world, sets, edits)
}

fn draw(
    scene: &SceneExtractor,
    world: &sindri_core::World,
    sets: &sindri_scene::TileSetBindings,
) -> Result<sindri_render::PreparedFrame, sindri_scene::SceneExtractError> {
    use glam::Vec3;
    let view = look_at(Vec3::new(8.0, 4.0, 25.0), Vec3::ZERO, Vec3::Y);
    scene.extract_animated_with_world_camera(
        world,
        VIEWPORT,
        ViewCamera {
            view,
            view_projection: orthographic_projection(-30.0, 30.0, -20.0, 20.0, 0.1, 100.0) * view,
            framed_half_height: 20.0,
        },
        &TextureBindings::new(),
        SceneRuntime::default().with_tile_sets(sets),
    )
}

#[test]
fn a_tolerant_preview_draws_textured_boxes_while_models_load() {
    let (world, sets, _) = fixture();
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.tolerate_invalid_components();
    let frame = draw(&extractor, &world, &sets).unwrap();
    assert_eq!(uploads(&frame), 2);
    assert!(
        extractor
            .problems()
            .iter()
            .any(|problem| problem.message.contains("vent.glb"))
    );
}

#[test]
fn modeled_blocks_batch_per_section_and_follow_edits_and_reloads() {
    use glam::Vec3;
    use sindri_scene::SceneExtractError;
    use std::sync::Arc;

    let mut extractor = SceneExtractor::new().unwrap();
    let (mut world, sets, edits) = fixture();
    let id = sindri_core::AssetId::new("vent.glb").unwrap();
    assert!(
        matches!(draw(&extractor, &world, &sets), Err(SceneExtractError::MissingModel(missing)) if missing == id)
    );
    extractor.bind_model(id.clone(), vent_model(1.5));
    assert!(
        matches!(draw(&extractor, &world, &sets), Err(SceneExtractError::InvalidVoxelModel(invalid)) if invalid == id)
    );
    extractor.bind_model(id.clone(), vent_model(0.25));

    let models = |frame: &sindri_render::PreparedFrame| {
        frame
            .passes()
            .iter()
            .filter_map(|pass| match &pass.command {
                FrameCommand::Model { model, asset } => Some((*model, Arc::clone(asset))),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let first = draw(&extractor, &world, &sets).unwrap();
    assert_eq!(
        uploads(&first),
        1,
        "the ordinary neighboring block still draws"
    );
    let first = models(&first);
    assert_eq!(first.len(), 2);
    assert_eq!(first[0].1.data().meshes[0][0].vertices.len(), 3);
    assert_eq!(first[1].1.data().meshes[0][0].vertices.len(), 6);
    assert!(
        first[1]
            .0
            .transform_point3(Vec3::ZERO)
            .abs_diff_eq(Vec3::new(10.0, 0.0, 0.0), 1e-6)
    );

    let settled = models(&draw(&extractor, &world, &sets).unwrap());
    assert!(Arc::ptr_eq(&first[1].1, &settled[1].1));
    extractor.bind_model(id, vent_model(0.75));
    let reloaded = models(&draw(&extractor, &world, &sets).unwrap());
    assert!(!Arc::ptr_eq(&first[1].1, &reloaded[1].1));
    let (_, max) = reloaded[1].1.bounds().unwrap();
    assert!((max.z - 0.75).abs() < 1e-6);

    edited(
        &mut world,
        serde_json::Value::Array(edits.as_array().unwrap()[..2].to_vec()),
    );
    let removed = models(&draw(&extractor, &world, &sets).unwrap());
    assert_eq!(
        removed.len(),
        1,
        "removing all vents releases that section's model batch"
    );
}
