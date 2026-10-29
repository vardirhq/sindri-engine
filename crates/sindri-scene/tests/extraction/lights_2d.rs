//! The 2D lights a frame carries, the shadows they cast, and what they light.

use sindri_render::{FrameCommand, Lighting2d, PreparedFrame, TextureId};
use sindri_scene::{CameraView, SceneExtractError, SceneExtractor, TextureBindings};

use crate::support::{VIEWPORT, scene, world_from};

fn dark(color: [f32; 3]) -> bool {
    color.iter().all(|channel| (channel - 0.1).abs() < 1.0e-5)
}

fn environment(enabled: bool) -> String {
    format!(
        r#",
        {{ "id": "environment", "components": {{ "sindri.environment": {{
            "lighting_2d": {{ "enabled": {enabled}, "ambient": [0.1, 0.1, 0.1] }} }} }} }}"#
    )
}

fn light(id: &str, x: f32, extra: &str) -> String {
    format!(
        r#",
        {{ "id": "{id}", "transform_3d": {{ "position": [{x}, 0.0, 0.0] }},
          "components": {{ "sindri.light2d": {{
            "color": [1.0, 0.5, 0.0, 1.0], "intensity": 2.0, "radius": 5.0,
            "falloff": 1.0 {extra} }} }} }}"#
    )
}

fn extract(entities: &str) -> Result<PreparedFrame, SceneExtractError> {
    let mut bindings = TextureBindings::new();
    bindings.bind("a.png", TextureId::new(1));
    bindings.bind("a_n.png", TextureId::new(2));
    SceneExtractor::new().unwrap().extract(
        &world_from(&scene(entities)),
        VIEWPORT,
        CameraView::default(),
        &bindings,
    )
}

fn lighting(entities: &str) -> Option<Lighting2d> {
    extract(entities)
        .expect("the scene extracts")
        .sprite_environment()
        .lighting
        .clone()
}

#[test]
fn nothing_is_lit_unless_the_environment_turns_2d_lighting_on() {
    assert_eq!(lighting(&light("lamp", 0.0, "")), None);
    assert_eq!(
        lighting(&(environment(false) + &light("lamp", 0.0, ""))),
        None
    );
}

#[test]
fn a_light_carries_its_place_reach_and_color_times_intensity() {
    let lit = lighting(&(environment(true) + &light("lamp", 2.0, ""))).expect("lighting is on");
    assert!(dark(lit.ambient));
    assert_eq!(lit.lights.len(), 1);
    let lamp = &lit.lights[0];
    assert_eq!(
        lamp.position.map(f32::to_bits),
        [2.0_f32, 0.0].map(f32::to_bits)
    );
    assert!((lamp.radius - 5.0).abs() < 1.0e-6);
    assert_eq!(
        lamp.color.map(f32::to_bits),
        [2.0_f32, 1.0, 0.0].map(f32::to_bits)
    );
    assert!(lamp.shadow.is_none());
}

#[test]
fn a_spot_light_lights_only_its_cone() {
    let beam = r#", "cone_angle": 60.0, "cone_direction": 90.0, "cone_softness": 0.0"#;
    let lit = lighting(&(environment(true) + &light("lamp", 0.0, beam))).expect("lighting is on");
    assert!(lit.lights[0].cone.is_some(), "the beam reaches the frame");
    assert!(lit.at([0.0, 2.0])[0] > 0.1, "up the beam");
    assert!(dark(lit.at([0.0, -2.0])), "behind it");
    assert!(dark(lit.at([2.0, 0.0])), "square across it");
    let round = lighting(&(environment(true) + &light("lamp", 0.0, ""))).expect("lighting is on");
    assert!(
        round.lights[0].cone.is_none(),
        "a light with no cone shines every way"
    );
}

#[test]
fn a_cone_that_is_not_a_cone_is_refused_naming_its_field() {
    for (extra, field) in [
        (r#", "cone_angle": 0.0"#, "cone_angle"),
        (r#", "cone_angle": 400.0"#, "cone_angle"),
        (r#", "cone_softness": 1.5"#, "cone_softness"),
    ] {
        let error = extract(&(environment(true) + &light("lamp", 0.0, extra)))
            .expect_err("a bad cone is refused");
        assert!(error.to_string().contains(field), "{extra}: {error}");
    }
}

#[test]
fn a_switched_off_light_lights_nothing() {
    let lit = lighting(&(environment(true) + &light("lamp", 0.0, r#", "enabled": false"#)))
        .expect("lighting is on");
    assert!(lit.lights.is_empty());
}

#[test]
fn a_box_occluder_shadows_what_is_behind_it() {
    let wall = r#",
        { "id": "wall", "transform_3d": { "position": [2.0, 0.0, 0.0] },
          "components": { "sindri.light2d.occluder": { "size": [0.5, 4.0] } } }"#;
    let entities =
        environment(true) + &light("lamp", 0.0, r#", "shadows": true, "shadow_reach": 0.0"#) + wall;
    let lit = lighting(&entities).expect("lighting is on");
    assert!(lit.at([1.0, 0.0])[0] > 0.1, "before the wall");
    assert!(dark(lit.at([3.5, 0.0])), "behind the wall");
    assert!(lit.at([-3.0, 0.0])[0] > 0.1, "the other way");
}

#[test]
fn a_tilemap_occluder_casts_from_its_solid_cells() {
    // One solid cell, the middle of three, covering x 1..2 and y 0..-1.
    let map = r#",
        { "id": "map", "transform_3d": { "position": [0.0, 0.5, 0.0] },
          "components": {
            "sindri.tilemap": {
              "texture": "a.png", "palette": ["a"], "columns": 3, "rows": 1,
              "tiles": [null, 0, null] },
            "sindri.light2d.occluder": { "shape": "tilemap" } } }"#;
    let entities =
        environment(true) + &light("lamp", 0.5, r#", "shadows": true, "shadow_reach": 0.0"#) + map;
    let lit = lighting(&entities).expect("lighting is on");
    assert!(lit.at([0.8, 0.0])[0] > 0.1, "beside the cell");
    assert!(dark(lit.at([2.6, 0.0])), "behind the cell");
}

#[test]
fn a_frame_carries_the_lights_nearest_the_camera_and_no_more() {
    let mut entities = environment(true);
    for index in 0..20 {
        #[allow(clippy::cast_precision_loss)]
        let x = 100.0 + index as f32 * 10.0;
        entities += &light(&format!("far-{index}"), x, "");
    }
    entities += &light("near", 3.0, "");
    let lit = lighting(&entities).expect("lighting is on");
    assert_eq!(lit.lights.len(), sindri_render::MAX_LIGHTS_2D);
    assert_eq!(
        lit.lights[0].position.map(f32::to_bits),
        [3.0_f32, 0.0].map(f32::to_bits),
        "the camera's own first"
    );
}

#[test]
fn world_sprites_are_lit_unless_they_opt_out_and_carry_their_normal_map() {
    let sprites = r#",
        { "id": "plain", "transform_3d": { "position": [0.0, 0.0, 0.0] },
          "components": { "sindri.sprite": { "texture": "a.png" } } },
        { "id": "bumpy", "transform_3d": { "position": [1.0, 0.0, 0.0] },
          "components": { "sindri.sprite": { "texture": "a.png", "normal_map": "a_n.png" } } },
        { "id": "glow", "transform_3d": { "position": [2.0, 0.0, 0.0] },
          "components": { "sindri.sprite": { "texture": "a.png", "unlit": true } } }"#;
    let frame = extract(&(environment(true) + sprites)).expect("the scene extracts");
    let mut drawn = Vec::new();
    for pass in frame.passes() {
        if let FrameCommand::SpriteBatch {
            normal_map,
            instances,
            ..
        } = &pass.command
        {
            for instance in instances {
                drawn.push((*normal_map, instance.is_lit()));
            }
        }
    }
    drawn.sort();
    assert_eq!(
        drawn,
        [(None, false), (None, true), (Some(TextureId::new(2)), true)]
    );
}

/// A tilemap's tiles are lit and read the map's normal map, laid out like its
/// texture, so a brick wall can show its bevels to a torch.
#[test]
fn a_tilemap_carries_its_normal_map_to_every_tile() {
    let map = r#",
        { "id": "wall", "components": { "sindri.tilemap": {
            "texture": "a.png", "normal_map": "a_n.png", "palette": ["brick"],
            "columns": 2, "rows": 1, "tiles": [0, 0] } } }"#;
    let frame = extract(&(environment(true) + map)).expect("the scene extracts");
    let mut drawn = Vec::new();
    for pass in frame.passes() {
        if let FrameCommand::SpriteBatch {
            normal_map,
            instances,
            ..
        } = &pass.command
        {
            drawn.extend(instances.iter().map(|tile| (*normal_map, tile.is_lit())));
        }
    }
    assert_eq!(drawn, [(Some(TextureId::new(2)), true); 2]);
}

#[test]
fn a_light_the_engine_cannot_use_names_its_field() {
    let error =
        extract(&(environment(true) + &light("lamp", 0.0, r#", "radius": 0.0"#))).unwrap_err();
    assert!(error.to_string().contains("radius"), "{error}");
}
