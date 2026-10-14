//! Which shader a sprite is drawn through, and what that does to batching.

use sindri_render::{FrameCommand, TextureId};
use sindri_scene::{CameraView, SceneExtractError, SceneExtractor, SceneRuntime, TextureBindings};

use crate::support::{VIEWPORT, scene, world_from};

const SHADER: &str =
    "fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> { return input.color; }\n";

fn sprite(id: &str, x: f32, shader: &str, params: &str) -> String {
    format!(
        r#",
        {{ "id": "{id}", "transform_3d": {{ "position": [{x}, 0.0, 0.0] }},
          "components": {{ "sindri.sprite": {{
            "texture": "a.png", "shader": "{shader}", "shader_params": {params} }} }} }}"#
    )
}

fn bindings() -> TextureBindings {
    let mut bindings = TextureBindings::new();
    bindings.bind("a.png", TextureId::new(1));
    bindings
        .shaders_mut()
        .bind("shaders/glow.wgsl", SHADER)
        .expect("the test shader compiles");
    bindings
}

/// What each batch was drawn through, in draw order, as asset paths.
fn batches(entities: &str, bindings: &TextureBindings) -> Vec<(Option<String>, usize)> {
    let frame = SceneExtractor::new()
        .unwrap()
        .extract(
            &world_from(&scene(entities)),
            VIEWPORT,
            CameraView::default(),
            bindings,
        )
        .expect("the scene extracts");
    frame
        .passes()
        .iter()
        .map(|pass| match &pass.command {
            FrameCommand::SpriteBatch {
                shader, instances, ..
            } => (
                shader.as_ref().map(|shader| shader.name().to_owned()),
                instances.len(),
            ),
            _ => panic!("the fixture holds only sprites"),
        })
        .collect()
}

#[test]
fn a_sprite_carries_its_shader_and_parameters() {
    let entities = sprite("a", 0.0, "shaders/glow.wgsl", "[0.25, 0.5, 0.75, 1.0]");
    let frame = SceneExtractor::new()
        .unwrap()
        .extract(
            &world_from(&scene(&entities)),
            VIEWPORT,
            CameraView::default(),
            &bindings(),
        )
        .expect("the scene extracts");
    let FrameCommand::SpriteBatch {
        shader, instances, ..
    } = &frame.passes()[0].command
    else {
        panic!("a sprite is a sprite batch");
    };
    assert_eq!(shader.as_ref().unwrap().name(), "shaders/glow.wgsl");
    // Quarters survive f64 to f32 exactly, so the bits are compared.
    assert_eq!(
        instances[0].shader_params().map(f32::to_bits),
        [0.25_f32, 0.5, 0.75, 1.0].map(f32::to_bits)
    );
}

/// Parameters are per instance, so sprites sharing a shader and a texture
/// share a draw whatever their parameters.
#[test]
fn sprites_sharing_a_shader_share_a_batch() {
    let entities = sprite("a", 0.0, "shaders/glow.wgsl", "[1.0, 0.0, 0.0, 0.0]")
        + &sprite("b", 1.0, "shaders/glow.wgsl", "[0.0, 1.0, 0.0, 0.0]");
    assert_eq!(
        batches(&entities, &bindings()),
        [(Some("shaders/glow.wgsl".to_owned()), 2)]
    );
}

/// A shader is pipeline state, so a sprite with one and a sprite without one
/// cannot be one draw even on one texture.
#[test]
fn a_shader_splits_a_batch() {
    let entities = sprite("a", 0.0, "shaders/glow.wgsl", "[0.0, 0.0, 0.0, 0.0]")
        + &sprite("b", 1.0, "", "[0.0, 0.0, 0.0, 0.0]");
    let drawn = batches(&entities, &bindings());
    assert_eq!(drawn.len(), 2, "{drawn:?}");
    assert!(drawn.contains(&(None, 1)));
    assert!(drawn.contains(&(Some("shaders/glow.wgsl".to_owned()), 1)));
}

/// A shader nobody bound draws the built-in look rather than failing the
/// frame; the host reports the missing file by name.
#[test]
fn an_unbound_shader_draws_the_built_in_look() {
    let entities = sprite("a", 0.0, "shaders/missing.wgsl", "[0.0, 0.0, 0.0, 0.0]");
    assert_eq!(batches(&entities, &bindings()), [(None, 1)]);
}

#[test]
fn a_parameter_that_is_not_a_number_is_refused() {
    // JSON has no NaN, so the world is edited after loading, which is how a
    // script or an inspector drag would get one there.
    let entities = sprite("a", 0.0, "shaders/glow.wgsl", "[0.0, 0.0, 0.0, 0.0]");
    let mut world = world_from(&scene(&entities));
    let entity = world
        .entities()
        .find(|(_, data)| data.components.contains_key("sindri.sprite"))
        .map(|(entity, _)| entity)
        .unwrap();
    world
        .get_mut(entity)
        .unwrap()
        .components
        .get_mut("sindri.sprite")
        .unwrap()["shader_params"] = serde_json::json!([1.0e39, 0.0, 0.0, 0.0]);
    let error = SceneExtractor::new()
        .unwrap()
        .extract_animated(
            &world,
            VIEWPORT,
            CameraView::default(),
            &bindings(),
            SceneRuntime::default(),
        )
        .unwrap_err();
    assert!(
        matches!(error, SceneExtractError::InvalidShaderParams),
        "{error}"
    );
}

/// The clock a host passes is the one shaders read.
#[test]
fn the_frame_carries_the_hosts_clock() {
    let entities = sprite("a", 0.0, "shaders/glow.wgsl", "[0.0, 0.0, 0.0, 0.0]");
    let frame = SceneExtractor::new()
        .unwrap()
        .extract_animated(
            &world_from(&scene(&entities)),
            VIEWPORT,
            CameraView::default(),
            &bindings(),
            SceneRuntime::default().with_seconds(3.5),
        )
        .unwrap();
    assert!((frame.sprite_environment().time - 3.5).abs() < f32::EPSILON);
}
