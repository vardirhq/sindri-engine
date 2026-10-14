use std::path::Path;

use sindri_assets::normal_map::encode_png;
use sindri_core::{SCENE_FORMAT_VERSION, SceneDocument};
use sindri_render::{TextureId, Viewport};
use sindri_scene::TextureBindings;

use super::*;

fn studio(folder: &Path) -> NormalStudio {
    let source = folder.join("floor.png");
    std::fs::write(
        &source,
        encode_png(32, 16, &[120, 90, 60, 255].repeat(32 * 16)).unwrap(),
    )
    .unwrap();
    NormalStudio::open(&source)
}

fn near(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-6)
}

#[test]
fn the_frame_carries_the_studios_light_as_a_scene_would() {
    let folder = tempfile::tempdir().unwrap();
    let studio = studio(folder.path());
    let mut scene = StudioScene::new();
    scene.follow(&studio);
    scene.wear(Look::Mapped);
    let frame = scene
        .extractor
        .extract(
            &scene.world,
            Viewport::new(64, 32),
            camera_for(&studio, 2.0),
            &TextureBindings::new(),
        )
        .unwrap();
    let lighting = frame
        .sprite_environment()
        .lighting
        .clone()
        .expect("2D lighting is on");
    assert!(near(&lighting.ambient, &studio.light.ambient));
    assert_eq!(lighting.lights.len(), 1);
    let light = &lighting.lights[0];
    assert!(near(&light.position, &studio.light.position));
    assert!((light.height - studio.light.light.height).abs() < 1e-6);
    assert!(near(&light.color, &studio.light.light.radiance()));
}

/// The scene a game would hold once the map is saved and used: the picture
/// and its saved map by file name, the studio's light as a `sindri.light2d`,
/// and its ambient as the environment's.
fn scene_using_the_map(studio: &NormalStudio) -> World {
    let [width, height] = studio.surface_size();
    let [x, y] = studio.light.position;
    let json = format!(
        r#"{{ "format_version": {SCENE_FORMAT_VERSION}, "entities": [
        {{ "id": "environment", "transform_3d": {{}}, "components": {{ "sindri.environment": {{
            "background": [0.055, 0.06, 0.075, 1.0],
            "lighting_2d": {{ "enabled": true, "ambient": {ambient:?} }} }} }} }},
        {{ "id": "floor", "transform_3d": {{ "scale": [{width}, {height}, 1.0] }},
          "components": {{ "sindri.sprite": {{ "texture": "floor.png", "normal_map": "floor_n.png" }} }} }},
        {{ "id": "lamp", "transform_3d": {{ "position": [{x}, {y}, 0.0] }},
          "components": {{ "sindri.light2d": {light} }} }}
        ] }}"#,
        ambient = studio.light.ambient,
        light = studio.light.component_json(),
    );
    let document = SceneDocument::from_json(&json).expect("the scene parses");
    World::from_scene(&document).expect("the scene loads").world
}

#[test]
fn a_scene_using_the_saved_map_extracts_the_frame_the_studio_draws() {
    let folder = tempfile::tempdir().unwrap();
    let mut studio = studio(folder.path());
    studio.save().unwrap();
    assert!(studio.map_path().ends_with("floor_n.png"));
    let mut scene = StudioScene::new();
    scene.follow(&studio);
    scene.wear(Look::Mapped);
    let game = scene_using_the_map(&studio);
    let (picture, map) = (TextureId::new(1), TextureId::new(2));
    let mut studio_bindings = TextureBindings::new();
    studio_bindings.bind(SOURCE, picture);
    studio_bindings.bind(NORMAL, map);
    let mut game_bindings = TextureBindings::new();
    game_bindings.bind("floor.png", picture);
    game_bindings.bind("floor_n.png", map);
    for aspect in [1.0, 2.0] {
        let camera = camera_for(&studio, aspect);
        let viewport = Viewport::new(64, 64);
        let drawn = scene
            .extractor
            .extract(&scene.world, viewport, camera, &studio_bindings)
            .unwrap();
        let played = scene
            .extractor
            .extract(&game, viewport, camera, &game_bindings)
            .unwrap();
        assert_eq!(format!("{drawn:?}"), format!("{played:?}"));
    }
}
