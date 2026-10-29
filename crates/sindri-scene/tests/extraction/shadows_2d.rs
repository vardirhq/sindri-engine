//! What things cast onto the ground: a contact ellipse under each, and a
//! silhouette laid away from the lights near it.

use glam::{Vec3, Vec4};
use sindri_render::{FrameCommand, PreparedFrame, RenderLayer, TextureId};
use sindri_scene::{CameraView, SceneExtractor, TextureBindings};

use crate::support::{VIEWPORT, close, scene, world_from};

fn environment(enabled: bool) -> String {
    format!(
        r#",
        {{ "id": "environment", "components": {{ "sindri.environment": {{
            "lighting_2d": {{ "enabled": {enabled}, "ambient": [0.1, 0.1, 0.1] }} }} }} }}"#
    )
}

/// A lamp at the origin, two units up, reaching eight, bright enough that its
/// light at a tree's foot is worth a shadow.
fn lamp() -> &'static str {
    r#",
        { "id": "lamp", "components": { "sindri.light2d": {
            "color": [1.0, 1.0, 1.0, 1.0], "intensity": 2.0, "radius": 8.0,
            "falloff": 1.0, "height": 2.0 } } }"#
}

/// A sprite two units tall standing at (4, 0), its picture anchored at its middle,
/// so its feet are at (4, -1).
fn tree(extra: &str) -> String {
    format!(
        r#",
        {{ "id": "tree", "transform_3d": {{ "position": [4.0, 0.0, 0.0], "scale": [1.0, 2.0, 1.0] }},
          "components": {{ "sindri.sprite": {{ "texture": "a.png", "layer": 5 }},
            "sindri.shadow2d": {{ {extra} }} }} }}"#
    )
}

fn extract(entities: &str) -> PreparedFrame {
    let mut bindings = TextureBindings::new();
    bindings.bind("a.png", TextureId::new(1));
    SceneExtractor::new()
        .unwrap()
        .extract(
            &world_from(&scene(entities)),
            VIEWPORT,
            CameraView::default(),
            &bindings,
        )
        .expect("the scene extracts")
}

/// Every sprite instance drawn, in draw order, as (model, tint, lit).
fn sprites(frame: &PreparedFrame) -> Vec<(glam::Mat4, [f32; 4], bool)> {
    let mut drawn = Vec::new();
    for pass in frame.passes() {
        if let FrameCommand::SpriteBatch { instances, .. } = &pass.command {
            drawn.extend(
                instances
                    .iter()
                    .map(|sprite| (sprite.model(), sprite.tint(), sprite.is_lit())),
            );
        }
    }
    drawn
}

#[test]
fn a_contact_shadow_is_a_dark_feathered_ellipse_under_the_things_sprite() {
    let frame = extract(&tree(
        r#""contact_size": [1.5, 0.5], "contact_opacity": 0.4, "contact_offset": [0.0, -1.0]"#,
    ));
    let mut found = Vec::new();
    for pass in frame.passes() {
        if let FrameCommand::Shapes { instances, .. } = &pass.command {
            found.extend(instances.iter().map(|shape| (pass.layer, *shape)));
        }
    }
    assert_eq!(found.len(), 1, "one ellipse: {}", found.len());
    let (layer, ellipse) = found[0];
    assert_eq!(layer, RenderLayer(4), "a layer below the sprite's own");
    assert_eq!(
        ellipse.fill().map(f32::to_bits),
        [0.0, 0.0, 0.0, 0.4].map(f32::to_bits)
    );
    assert!(
        !ellipse.is_lit(),
        "painted on the lit ground, not lit itself"
    );
    let model = ellipse.model();
    let middle = model.transform_point3(Vec3::ZERO);
    assert!(
        close(middle.x, 4.0) && close(middle.y, -1.0),
        "at the offset: {middle:?}"
    );
    assert!(
        close(model.x_axis.x, 1.5) && close(model.y_axis.y, 0.5),
        "its size"
    );
}

#[test]
fn a_shadow_with_no_contact_size_or_opacity_draws_no_ellipse() {
    for extra in [r#""contact_size": [1.0, 0.0]"#, r#""contact_opacity": 0.0"#] {
        let frame = extract(&tree(extra));
        assert!(
            frame
                .passes()
                .iter()
                .all(|pass| !matches!(pass.command, FrameCommand::Shapes { .. })),
            "{extra}"
        );
    }
}

#[test]
fn a_cast_shadow_is_the_picture_laid_away_from_the_light_with_its_feet_in_place() {
    let frame = extract(
        &(environment(true) + lamp() + &tree(r#""contact_size": [0.0, 0.0], "cast": true"#)),
    );
    let drawn = sprites(&frame);
    assert_eq!(drawn.len(), 2, "the shadow and the tree");
    let (model, tint, lit) = drawn[0];
    assert!(!lit, "not lit itself");
    assert_eq!(tint[..3], [0.0, 0.0, 0.0], "black");
    assert!(
        tint[3] > 0.0 && tint[3] <= 0.45,
        "fainter than the full opacity: {}",
        tint[3]
    );

    // Feet at (4, -1), the lamp at the origin: away is (4, -1) normalized, a
    // length of height 2 * distance 4.12 / lamp height 2, held to the 2.5 maximum.
    let feet = model.transform_point3(Vec3::new(0.0, -0.5, 0.0));
    assert!(
        close(feet.x, 4.0) && close(feet.y, -1.0),
        "its feet stay put: {feet:?}"
    );
    let away = Vec4::new(4.0, -1.0, 0.0, 0.0).truncate().normalize();
    let head = model.y_axis.truncate();
    assert!(
        close(head.length(), 2.5),
        "capped at the longest: {}",
        head.length()
    );
    assert!(
        close(head.normalize().dot(away), 1.0),
        "pointing away from the lamp"
    );
    let across = model.x_axis.truncate();
    assert!(close(across.length(), 1.0), "as wide as the picture");
    assert!(
        close(across.dot(head), 0.0),
        "across the shadow, not along it"
    );
}

#[test]
fn a_taller_thing_and_a_lower_lamp_cast_longer_until_the_maximum() {
    let long = r#""contact_size": [0.0, 0.0], "cast": true, "max_length": 100.0"#;
    let short = |height: f32| {
        extract(&(environment(true) + lamp() + &tree(&format!(r#"{long}, "height": {height}"#))))
    };
    let length = |frame: &PreparedFrame| sprites(frame)[0].0.y_axis.truncate().length();
    let one = length(&short(1.0));
    let two = length(&short(2.0));
    assert!(
        close(two, one * 2.0),
        "twice as tall, twice as long: {one} then {two}"
    );
    // 1 * 4.123 / 2 for a lamp two units up.
    assert!(close(one, 4.123_106 / 2.0), "{one}");
}

#[test]
fn a_light_that_the_thing_carries_casts_nothing_from_it() {
    let carried = r#",
        { "id": "lantern", "parent": "tree", "components": { "sindri.light2d": {
            "color": [1.0, 1.0, 1.0, 1.0], "intensity": 2.0, "radius": 8.0, "falloff": 1.0 } } }"#;
    let frame = extract(
        &(environment(true) + &tree(r#""contact_size": [0.0, 0.0], "cast": true"#) + carried),
    );
    assert_eq!(sprites(&frame).len(), 1, "only the tree itself");
}

#[test]
fn nothing_is_cast_without_2d_lighting_a_light_in_reach_or_the_cast_switched_on() {
    let on = r#""contact_size": [0.0, 0.0], "cast": true"#;
    assert_eq!(
        sprites(&extract(&(environment(false) + lamp() + &tree(on)))).len(),
        1,
        "lighting off"
    );
    assert_eq!(
        sprites(&extract(&(environment(true) + &tree(on)))).len(),
        1,
        "no light"
    );
    let far = r#",
        { "id": "far", "transform_3d": { "position": [100.0, 0.0, 0.0] },
          "components": { "sindri.light2d": { "radius": 3.0 } } }"#;
    assert_eq!(
        sprites(&extract(&(environment(true) + far + &tree(on)))).len(),
        1,
        "out of reach"
    );
    let off = r#""contact_size": [0.0, 0.0], "cast": false"#;
    assert_eq!(
        sprites(&extract(&(environment(true) + lamp() + &tree(off)))).len(),
        1,
        "not casting"
    );
}

#[test]
fn the_shadow_is_drawn_before_the_thing_that_casts_it() {
    let frame = extract(
        &(environment(true) + lamp() + &tree(r#""contact_size": [0.0, 0.0], "cast": true"#)),
    );
    let drawn = sprites(&frame);
    assert!(
        !drawn[0].2 && drawn[1].2,
        "the unlit black shadow first, the lit tree over it"
    );
}

#[test]
fn a_shadow_value_out_of_range_names_its_field() {
    let mut bindings = TextureBindings::new();
    bindings.bind("a.png", TextureId::new(1));
    let error = SceneExtractor::new()
        .unwrap()
        .extract(
            &world_from(&scene(&tree(r#""cast_opacity": 2.0"#))),
            VIEWPORT,
            CameraView::default(),
            &bindings,
        )
        .unwrap_err();
    // The contact pass is the first to read it, whether or not it casts.
    assert!(error.to_string().contains("cast_opacity"), "{error}");
}

/// How many of the sprites drawn are lit from where they stand.
fn standing(frame: &PreparedFrame) -> usize {
    frame
        .passes()
        .iter()
        .filter_map(|pass| match &pass.command {
            FrameCommand::SpriteBatch { instances, .. } => Some(instances),
            _ => None,
        })
        .flatten()
        .filter(|sprite| sprite.is_standing())
        .count()
}

#[test]
fn a_thing_with_a_shadow_is_lit_from_where_it_stands() {
    let lit = |extra: &str, lighting: bool| {
        extract(&format!(
            "{}{}{}",
            environment(lighting),
            lamp(),
            tree(extra)
        ))
    };
    assert_eq!(standing(&lit("", true)), 1, "by default");
    assert_eq!(
        standing(&lit(r#""cast": true"#, true)),
        1,
        "its silhouette is not another sprite that stands"
    );
    assert_eq!(
        standing(&lit(r#""lit_from_base": false"#, true)),
        0,
        "unless it opts out"
    );
    assert_eq!(
        standing(&lit("", false)),
        0,
        "and with no lighting nothing is"
    );
}

#[test]
fn a_sprite_without_a_shadow_is_lit_pixel_by_pixel() {
    let frame = extract(&format!(
        "{}{}{}",
        environment(true),
        lamp(),
        r#",
        { "id": "rug", "components": { "sindri.sprite": { "texture": "a.png", "layer": 1 } } }"#
    ));
    assert_eq!(standing(&frame), 0);
}

/// A lamp at the origin whose beam points `direction` degrees round and is 60 across.
fn spot(direction: f32) -> String {
    format!(
        r#",
        {{ "id": "lamp", "components": {{ "sindri.light2d": {{
            "color": [1.0, 1.0, 1.0, 1.0], "intensity": 2.0, "radius": 8.0,
            "falloff": 1.0, "height": 2.0,
            "cone_angle": 60.0, "cone_direction": {direction}, "cone_softness": 0.0 }} }} }}"#
    )
}

#[test]
fn a_spot_light_casts_a_shadow_only_where_it_shines() {
    let on = r#""contact_size": [0.0, 0.0], "cast": true"#;
    // The tree stands east of the lamp.
    assert_eq!(
        sprites(&extract(&(environment(true) + &spot(0.0) + &tree(on)))).len(),
        2,
        "a beam on the tree casts its shadow"
    );
    assert_eq!(
        sprites(&extract(&(environment(true) + &spot(180.0) + &tree(on)))).len(),
        1,
        "a beam pointed away casts none"
    );
}
