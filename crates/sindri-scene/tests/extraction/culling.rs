//! Cubes and models wholly out of view are not drawn, unless they may still
//! cast a shadow into it.

use std::sync::Arc;

use glam::{Mat4, Vec3};
use sindri_core::AssetId;
use sindri_render::{
    FrameCommand, ModelData, ModelGeometry, ModelHierarchyNode, ModelIndexData, ModelVertex,
    RenderModel,
};
use sindri_scene::{CameraView, SceneExtractor, TextureBindings};

use crate::support::{VIEWPORT, document, world_from};

/// A perspective camera at the origin, looking down -Z out to 100 units.
const CAMERA: &str = r#"
    { "id": "camera", "transform_3d": { "position": [0, 0, 0] },
      "components": { "sindri.camera": {
        "projection": "perspective", "vertical_fov_degrees": 60.0, "near": 0.1, "far": 100.0 } } }"#;

fn cube(id: &str, position: [f32; 3]) -> String {
    format!(
        r#", {{ "id": "{id}", "transform_3d": {{ "position": [{}, {}, {}] }},
           "components": {{ "sindri.mesh": {{ "primitive": "cube", "texture": "t" }} }} }}"#,
        position[0], position[1], position[2]
    )
}

fn drawn_cubes(entities: &str) -> Vec<Vec3> {
    let world = world_from(&document(&format!("{CAMERA}{entities}")));
    let frame = SceneExtractor::new()
        .unwrap()
        .extract(
            &world,
            VIEWPORT,
            CameraView::default(),
            &TextureBindings::new(),
        )
        .unwrap();
    frame
        .passes()
        .iter()
        .filter_map(|pass| match &pass.command {
            FrameCommand::TexturedCube { model, .. } => Some(model.transform_point3(Vec3::ZERO)),
            _ => None,
        })
        .collect()
}

#[test]
fn cubes_out_of_view_are_not_drawn() {
    let drawn = drawn_cubes(
        &[
            cube("ahead", [0.0, 0.0, -10.0]),
            cube("behind", [0.0, 0.0, 10.0]),
            cube("far-left", [-200.0, 0.0, -10.0]),
            cube("past-the-far-plane", [0.0, 0.0, -150.0]),
            // Its middle is out of view, but its near face is not.
            cube("on-the-edge", [-6.2, 0.0, -10.0]),
        ]
        .concat(),
    );
    assert_eq!(drawn.len(), 2, "{drawn:?}");
    assert!(drawn.contains(&Vec3::new(0.0, 0.0, -10.0)));
    assert!(drawn.contains(&Vec3::new(-6.2, 0.0, -10.0)));
}

#[test]
fn a_cube_out_of_view_that_can_shadow_it_is_kept_while_shadows_are_on() {
    // A sun overhead and shadows on: the cube above the camera's view is out
    // of sight, but inside the light's shadow view, so it may shadow what is.
    let lit = |enabled: bool| {
        format!(
            r#", {{ "id": "environment", "components": {{ "sindri.environment": {{
                    "shadows": {{ "enabled": {enabled}, "map_size": 1024, "distance": 30.0, "bias": 0.002 }} }} }} }},
                {{ "id": "sun", "transform_3d": {{ "rotation": [-0.7071068, 0, 0, 0.7071068] }},
                  "components": {{ "sindri.light": {{ "kind": "directional", "intensity": 1.0 }} }} }}{}"#,
            cube("overhead", [0.0, 12.0, -3.0])
        )
    };
    assert_eq!(drawn_cubes(&lit(true)).len(), 1, "kept as a shadow caster");
    assert!(
        drawn_cubes(&lit(false)).is_empty(),
        "culled with shadows off"
    );
}

/// A model whose only triangle lies from x = 10 to x = 12 in its own space.
fn offset_model() -> Arc<RenderModel> {
    let vertex = |x: f32, y: f32| ModelVertex {
        position: [x, y, 0.0],
        normal: [0.0, 0.0, 1.0],
        uv: [0.0, 0.0],
    };
    Arc::new(
        RenderModel::new(ModelData {
            nodes: vec![ModelHierarchyNode {
                name: None,
                local_transform: Mat4::IDENTITY,
                children: Vec::new(),
                mesh: Some(0),
            }],
            roots: vec![0],
            meshes: vec![vec![ModelGeometry {
                vertices: vec![vertex(10.0, 0.0), vertex(12.0, 0.0), vertex(11.0, 1.0)],
                indices: ModelIndexData::U16(vec![0, 1, 2]),
                material: None,
            }]],
            ..ModelData::default()
        })
        .unwrap(),
    )
}

#[test]
fn a_model_is_culled_by_where_its_geometry_is_not_where_its_origin_is() {
    let asset = offset_model();
    let (min, max) = asset.bounds().unwrap();
    assert_eq!(
        (min, max),
        (Vec3::new(10.0, 0.0, 0.0), Vec3::new(12.0, 1.0, 0.0))
    );
    let model = |id: &str, x: f32| {
        format!(
            r#", {{ "id": "{id}", "transform_3d": {{ "position": [{x}, 0, -10] }},
                  "components": {{ "sindri.model": {{ "asset": "models/offset.glb" }} }} }}"#
        )
    };
    // "origin-in-view" sits at the middle of the view, but its triangle is
    // ten units to the right, out of it. "origin-out-of-view" is the reverse.
    let world = world_from(&document(&format!(
        "{CAMERA}{}{}",
        model("origin-in-view", 0.0),
        model("origin-out-of-view", -11.0)
    )));
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.bind_model(AssetId::new("models/offset.glb").unwrap(), asset);
    let frame = extractor
        .extract(
            &world,
            VIEWPORT,
            CameraView::default(),
            &TextureBindings::new(),
        )
        .unwrap();
    let drawn: Vec<_> = frame
        .passes()
        .iter()
        .filter_map(|pass| match &pass.command {
            FrameCommand::Model { model, .. } => Some(model.transform_point3(Vec3::ZERO).x),
            _ => None,
        })
        .collect();
    assert_eq!(drawn, vec![-11.0]);
}
