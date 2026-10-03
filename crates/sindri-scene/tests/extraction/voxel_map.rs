//! A voxel world viewed as a map: the same world, drawn from straight above
//! as one square per column with the top face of its highest block.

use glam::Vec3;
use sindri_core::TileSetDocument;
use sindri_render::{FrameCommand, PreparedFrame, SpriteInstance, TextureId};
use sindri_scene::{SceneExtractor, SceneRuntime, TextureBindings, TileSetBindings, ViewCamera};

use crate::support::{VIEWPORT, document, world_from};

/// A world of grass over dirt over stone, viewed as a map, with `extra`
/// spliced into its component (a height variation, edits).
fn world(view: &str, variation: u32, extra: &str) -> sindri_core::World {
    let entity = format!(
        r#"{{ "id": "terrain", "transform_3d": {{}}, "components": {{
          "sindri.voxel_world": {{
            "generator": {{
              "kind": "layered_terrain", "seed": 3,
              "base_height": 4, "height_variation": {variation},
              "surface_voxel": "grass", "subsurface_voxel": "dirt",
              "deep_voxel": "stone", "subsurface_depth": 2
            }},
            "blocks": "terrain.tileset",
            "view": "{view}"{extra}
          }}
        }} }}"#
    );
    world_from(&document(&entity))
}

fn blocks() -> TileSetBindings {
    let face = |sprite: &str| format!(r#"{{ "sprite": "{sprite}", "size": [1.0, 1.0] }}"#);
    let block = |top: &str| {
        format!(
            r#"{{ "faces": {{ "top": {}, "south": {}, "east": {} }} }}"#,
            face(top),
            face("dirt.png"),
            face("dirt.png")
        )
    };
    let document = TileSetDocument::from_json(&format!(
        r#"{{ "format_version": 1, "tiles": {{
             "grass": {}, "dirt": {}, "stone": {} }} }}"#,
        block("grass.png"),
        block("dirt.png"),
        block("stone.png")
    ))
    .expect("the block set parses");
    let mut bindings = TileSetBindings::new();
    bindings
        .bind("terrain.tileset", document)
        .expect("the block set is valid");
    bindings
}

fn textures() -> TextureBindings {
    let mut bindings = TextureBindings::new();
    bindings.bind("grass.png", TextureId::new(1));
    bindings.bind("dirt.png", TextureId::new(2));
    bindings.bind("stone.png", TextureId::new(3));
    bindings
}

/// A camera looking straight down at the map's plane, framing `half` units
/// either side of `(x, y)`.
fn above(x: f32, y: f32, half: f32) -> ViewCamera {
    let view =
        glam::camera::rh::view::look_at_mat4(Vec3::new(x, y, 10.0), Vec3::new(x, y, 0.0), Vec3::Y);
    let projection =
        glam::camera::rh::proj::directx::orthographic(-half, half, -half, half, 0.1, 100.0);
    ViewCamera {
        view,
        view_projection: projection * view,
        framed_half_height: half,
    }
}

fn extract(world: &sindri_core::World, camera: ViewCamera) -> PreparedFrame {
    let tile_sets = blocks();
    SceneExtractor::new()
        .unwrap()
        .extract_animated_with_world_camera(
            world,
            VIEWPORT,
            camera,
            &textures(),
            SceneRuntime {
                tile_sets: Some(&tile_sets),
                ..SceneRuntime::default()
            },
        )
        .expect("the map extracts")
}

/// Every sprite drawn, with the texture it was drawn with.
fn squares(frame: &PreparedFrame) -> Vec<(TextureId, SpriteInstance)> {
    frame
        .passes()
        .iter()
        .filter_map(|pass| match &pass.command {
            FrameCommand::SpriteBatch {
                texture, instances, ..
            } => Some(instances.iter().map(|instance| (*texture, *instance))),
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn a_map_draws_each_column_it_can_see_as_its_top_block() {
    let frame = extract(&world("map", 0, ""), above(4.0, -4.0, 4.0));
    let drawn = squares(&frame);
    // Eight columns across and eight rows down are in view, and one more on
    // each side so a square half on screen is drawn.
    assert!(
        (64..=144).contains(&drawn.len()),
        "about the columns in view: {}",
        drawn.len()
    );
    assert!(
        drawn
            .iter()
            .all(|(texture, _)| *texture == TextureId::new(1)),
        "a flat world of grass is all grass from above"
    );
    // Column 2, row 5 is the unit square from (2, -5) to (3, -6).
    let middle = Vec3::new(2.5, -5.5, 0.0);
    assert!(
        drawn
            .iter()
            .any(|(_, square)| square.model().w_axis.truncate().distance(middle) < 1.0e-4),
        "row Z runs down the map"
    );
    assert!(
        !frame
            .passes()
            .iter()
            .any(|pass| matches!(pass.command, FrameCommand::CachedTexturedMesh { .. })),
        "and no blocks are meshed for it"
    );
}

#[test]
fn the_same_world_viewed_as_blocks_draws_no_map() {
    let frame = extract(&world("blocks", 0, ""), above(4.0, -4.0, 4.0));
    assert!(squares(&frame).is_empty());
}

#[test]
fn relief_is_drawn_as_light() {
    let flat = squares(&extract(&world("map", 0, ""), above(8.0, -8.0, 8.0)));
    assert!(
        flat.iter()
            .all(|(_, square)| (square.tint()[0] - 1.0).abs() < 1.0e-6),
        "flat ground at the reference height is drawn as it is"
    );
    let rolling = squares(&extract(&world("map", 6, ""), above(8.0, -8.0, 8.0)));
    let lit = rolling
        .iter()
        .filter(|(_, square)| square.tint()[0] > 1.0)
        .count();
    let shaded = rolling
        .iter()
        .filter(|(_, square)| square.tint()[0] < 1.0)
        .count();
    assert!(
        lit > 0 && shaded > 0,
        "slopes catch the light and fall into shadow: {lit} lit, {shaded} shaded"
    );
}

#[test]
fn an_edit_is_drawn() {
    // The grass at column 2, row 5 dug away shows the dirt under it.
    let edit = r#", "edits": [{ "at": [2, 4, 5], "block": "" }]"#;
    let drawn = squares(&extract(&world("map", 0, edit), above(4.0, -4.0, 4.0)));
    let dug = Vec3::new(2.5, -5.5, 0.0);
    let (texture, _) = drawn
        .iter()
        .find(|(_, square)| square.model().w_axis.truncate().distance(dug) < 1.0e-4)
        .expect("the dug column is still drawn");
    assert_eq!(*texture, TextureId::new(2), "as the dirt under the grass");
}
