//! Engine voxel worlds are extracted through the same scene path as games and
//! the editor, while their compiled sections persist between frames.

use glam::Mat4;
use sindri_render::{FrameCommand, TextureId};
use sindri_scene::{CameraView, SceneExtractor, SceneRuntime, TextureBindings, ViewCamera};

use crate::support::{VIEWPORT, scene, world_from};

fn voxel_world() -> sindri_core::World {
    voxel_world_with_radius(0)
}

fn voxel_world_with_radius(render_radius: u32) -> sindri_core::World {
    let entity = r#",
        { "id": "terrain", "transform_3d": {}, "components": {
          "sindri.voxel_world": {
            "generator": {
              "kind": "layered_terrain", "seed": 7,
              "base_height": 0, "height_variation": 0,
              "surface_voxel": 1, "subsurface_voxel": 2,
              "deep_voxel": 3, "subsurface_depth": 3
            },
            "materials": [
              { "voxel": 1, "top": "surface.png", "side": "soil.png",
                "bottom": "soil.png" },
              { "voxel": 2, "top": "soil.png", "side": "soil.png",
                "bottom": "soil.png" },
              { "voxel": 3, "top": "stone.png", "side": "stone.png",
                "bottom": "stone.png" }
            ],
            "focus": [0, 0, 0], "render_radius": __RENDER_RADIUS__,
            "vertical_radius": 0, "layer": 0
          }
        } }"#
        .replace("__RENDER_RADIUS__", &render_radius.to_string());
    world_from(&scene(&entity))
}

#[test]
fn resident_sections_outside_the_camera_frustum_are_not_submitted() {
    let extractor = SceneExtractor::new().unwrap();
    let world = voxel_world_with_radius(1);
    let frame = extractor
        .extract_animated_with_world_camera(
            &world,
            VIEWPORT,
            ViewCamera {
                view: Mat4::IDENTITY,
                view_projection: Mat4::IDENTITY,
                framed_half_height: 1.0,
            },
            &textures(),
            SceneRuntime::default(),
        )
        .expect("the voxel world extracts through the test camera");
    let submitted = frame
        .passes()
        .iter()
        .filter(|pass| matches!(&pass.command, FrameCommand::CachedTexturedMesh { .. }))
        .count();
    assert!(
        submitted > 0,
        "sections intersecting the frustum remain visible"
    );
    assert!(
        submitted < 9,
        "the 3x3 resident window should not all be submitted through a unit clip volume"
    );
}

fn textures() -> TextureBindings {
    let mut bindings = TextureBindings::new();
    bindings.bind("surface.png", TextureId::new(1));
    bindings.bind("soil.png", TextureId::new(2));
    bindings.bind("stone.png", TextureId::new(3));
    bindings
}

#[test]
fn settled_voxel_world_reuses_its_compiled_section() {
    let extractor = SceneExtractor::new().unwrap();
    let world = voxel_world();
    let textures = textures();

    let first = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .expect("the voxel world extracts");
    assert!(first.passes().iter().any(|pass| matches!(
        &pass.command,
        FrameCommand::CachedTexturedMesh {
            replacement: Some(_),
            ..
        }
    )));

    let settled = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .expect("the settled voxel world extracts");
    let cached = settled
        .passes()
        .iter()
        .filter_map(|pass| match &pass.command {
            FrameCommand::CachedTexturedMesh { replacement, .. } => Some(replacement),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!cached.is_empty(), "settled sections remain drawable");
    assert!(
        cached.iter().all(|replacement| replacement.is_none()),
        "an unchanged resident section must not upload replacement geometry"
    );
}

fn uploads(frame: &sindri_render::PreparedFrame) -> usize {
    frame
        .passes()
        .iter()
        .filter(|pass| {
            matches!(
                &pass.command,
                FrameCommand::CachedTexturedMesh {
                    replacement: Some(_),
                    ..
                }
            )
        })
        .count()
}

/// Binding a texture the world does not use leaves its meshes alone. The
/// bindings' generation moves on any bind in the project, and comparing it
/// rebuilt the whole voxel world whenever a sprite elsewhere loaded.
#[test]
fn an_unrelated_texture_does_not_rebuild_the_world() {
    let extractor = SceneExtractor::new().unwrap();
    let world = voxel_world();
    let mut textures = textures();
    extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .expect("the voxel world extracts");

    textures.bind("somebody-elses-sprite.png", TextureId::new(9));
    let after = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .expect("the voxel world extracts");
    assert_eq!(uploads(&after), 0, "no section was compiled again");
    assert!(
        !after
            .passes()
            .iter()
            .any(|pass| matches!(pass.command, FrameCommand::ReleaseCachedTexturedMesh { .. })),
        "no section was released"
    );
}

/// A texture a face draws with, bound to something else, is a change to what
/// the world looks like, so its sections are compiled again.
#[test]
fn a_texture_the_world_draws_with_rebuilds_it() {
    let extractor = SceneExtractor::new().unwrap();
    let world = voxel_world();
    let mut textures = textures();
    extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .expect("the voxel world extracts");

    textures.bind("surface.png", TextureId::new(7));
    let after = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .expect("the voxel world extracts");
    assert!(
        uploads(&after) > 0,
        "the surface is drawn with the new texture"
    );
}

fn edited(world: &mut sindri_core::World, edits: serde_json::Value) {
    let terrain = world
        .entities()
        .find(|(_, data)| data.components.contains_key("sindri.voxel_world"))
        .map(|(entity, _)| entity)
        .expect("the terrain exists");
    world
        .get_mut(terrain)
        .unwrap()
        .components
        .get_mut("sindri.voxel_world")
        .unwrap()["edits"] = edits;
}

/// How many sections were compiled again: a section uploads one batch per
/// texture it draws with, so batches are counted by where they are drawn.
fn sections_uploaded(frame: &sindri_render::PreparedFrame) -> usize {
    frame
        .passes()
        .iter()
        .filter_map(|pass| match &pass.command {
            FrameCommand::CachedTexturedMesh {
                model,
                replacement: Some(_),
                ..
            } => Some(model.w_axis.to_array().map(f32::to_bits)),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}

fn releases(frame: &sindri_render::PreparedFrame) -> usize {
    frame
        .passes()
        .iter()
        .filter(|pass| matches!(pass.command, FrameCommand::ReleaseCachedTexturedMesh { .. }))
        .count()
}

/// An edit remeshes the one section it is in, and taking it back does the
/// same; the rest of the resident world is left as it was compiled.
#[test]
fn an_edit_rebuilds_only_the_section_it_touches() {
    let extractor = SceneExtractor::new().unwrap();
    let mut world = voxel_world_with_radius(1);
    let textures = textures();
    let first = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .expect("the voxel world extracts");
    assert!(uploads(&first) > 1, "the whole window is compiled once");

    // In the middle of the centre section, so no neighbour shares the face.
    edited(
        &mut world,
        serde_json::json!([{ "at": [8, 1, 8], "block": 3 }]),
    );
    let built = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .expect("the edited world extracts");
    assert_eq!(
        sections_uploaded(&built),
        1,
        "only the edited section is compiled again"
    );
    assert_eq!(releases(&built), 0);

    edited(&mut world, serde_json::json!([]));
    let undone = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .expect("the restored world extracts");
    assert_eq!(
        sections_uploaded(&undone),
        1,
        "taking it back rebuilds the same one"
    );

    let settled = extractor
        .extract(&world, VIEWPORT, CameraView::default(), &textures)
        .expect("the settled world extracts");
    assert_eq!(uploads(&settled), 0);
}

/// A world that follows the camera keeps its window under what the camera is
/// looking at, so looking somewhere else loads the ground there and lets go
/// of what was left behind.
#[test]
fn a_world_following_the_camera_moves_its_window_with_the_view() {
    let extractor = SceneExtractor::new().unwrap();
    let mut world = voxel_world();
    let terrain = world
        .entities()
        .find(|(_, data)| data.components.contains_key("sindri.voxel_world"))
        .map(|(entity, _)| entity)
        .unwrap();
    world
        .get_mut(terrain)
        .unwrap()
        .components
        .get_mut("sindri.voxel_world")
        .unwrap()["follow_camera"] = serde_json::json!(true);
    let textures = textures();
    let looking_at = |x: f32| {
        let target = glam::Vec3::new(x, 0.0, -4.0);
        let view = glam::camera::rh::view::look_at_mat4(
            target + glam::Vec3::new(30.0, 30.0, 30.0),
            target,
            glam::Vec3::Y,
        );
        let projection =
            glam::camera::rh::proj::directx::orthographic(-20.0, 20.0, -20.0, 20.0, 0.1, 200.0);
        ViewCamera {
            view,
            view_projection: projection * view,
            framed_half_height: 20.0,
        }
    };
    let extract = |camera| {
        extractor
            .extract_animated_with_world_camera(
                &world,
                VIEWPORT,
                camera,
                &textures,
                SceneRuntime::default(),
            )
            .expect("the voxel world extracts")
    };
    // The view meets the middle of the section's height eight across from
    // where it looks on the ground, so these look inside one section.
    let here = extract(looking_at(-4.0));
    assert_eq!(
        sections_uploaded(&here),
        1,
        "one resident section, under the view"
    );
    let still = extract(looking_at(-3.0));
    assert_eq!(uploads(&still), 0, "a small look inside it changes nothing");
    let away = extract(looking_at(200.0));
    assert_eq!(sections_uploaded(&away), 1, "the ground there is loaded");
    assert!(releases(&away) > 0, "and the ground left behind let go");
}
