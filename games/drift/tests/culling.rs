//! What the frame draws: the hull plates, fittings and wreckage out of the
//! diver's view are culled, so a ship of a few hundred pieces draws only what
//! is in front of the eye.

use drift::Run;
use sindri_assets::{AssetBytes, AssetDecoder, ModelAssetDecoder};
use sindri_core::{AssetId, SceneDocument, TileSetDocument, World};
use sindri_render::{FrameCommand, ModelIndexData, RenderModel, Viewport};
use sindri_scene::{
    CameraView, SceneExtractor, SceneRuntime, TextureBindings, TileSetBindings, ViewCamera,
    referenced_models, tile_set_models,
};

/// Loads every model the scene names and binds it for drawing, as a host does
/// before its first frame; a headless run only loads what collides.
fn bind_models(scene: &mut SceneExtractor, world: &World, blocks: &TileSetDocument) {
    let mut names = referenced_models(world);
    names.extend(tile_set_models(blocks).map(ToString::to_string));
    for name in names {
        let id = AssetId::new(name.clone()).expect("a model path");
        let bytes =
            std::fs::read(drift::project().join("assets").join(&name)).expect("the model reads");
        let asset = ModelAssetDecoder
            .decode(AssetBytes::new(id.clone(), bytes))
            .expect("the model decodes");
        let prepared = sindri::model::prepare(asset).expect("the model prepares");
        scene.bind_model(id, prepared.resource);
    }
}

fn bind_assets(scene: &mut SceneExtractor, world: &World) -> TileSetBindings {
    let blocks = std::fs::read_to_string(drift::project().join("assets/blocks.tileset")).unwrap();
    let blocks = TileSetDocument::from_json(&blocks).unwrap();
    bind_models(scene, world, &blocks);
    let mut tile_sets = TileSetBindings::new();
    tile_sets.bind("blocks.tileset", blocks).unwrap();
    tile_sets
}

/// How many cubes and models one frame draws, and how many the scene has.
fn drawn(game: &mut Run) -> (usize, usize) {
    let tile_sets = bind_assets(&mut game.run.scene, &game.run.world);
    let authored = game
        .run
        .world
        .entities()
        .filter(|(_, data)| {
            data.components.contains_key("sindri.mesh")
                || data.components.contains_key("sindri.model")
        })
        .count();
    let passes = game
        .run
        .draw_with(|scene, world, _| {
            let frame = scene
                .extract_animated(
                    world,
                    Viewport::new(1280, 720),
                    CameraView::default(),
                    &TextureBindings::new(),
                    SceneRuntime::default().with_tile_sets(&tile_sets),
                )
                .map_err(|error| error.to_string())?;
            let drawn = frame
                .passes()
                .iter()
                .filter(|pass| {
                    !pass.shadow_only
                        && matches!(
                            pass.command,
                            FrameCommand::TexturedCube { .. } | FrameCommand::Model { .. }
                        )
                })
                .count();
            Ok((drawn, sindri_scene::UiTextSizes::new()))
        })
        .expect("the frame extracts");
    (passes, authored)
}

#[test]
fn facing_aft_in_the_cabin_draws_neither_the_hold_nor_the_wreckage() {
    let mut game = Run::open().expect("DRIFT opens");
    game.wait(0.3);
    let (forward, authored) = drawn(&mut game);
    assert!(
        authored > 150,
        "the ship and the wreckage: {authored} pieces"
    );
    // Culling is by the view's frustum, not by what walls hide: looking
    // forward down the cabin, the hold and the wreckage are straight ahead
    // behind the bulkheads, and are drawn.
    assert!(forward * 10 > authored * 8, "{forward} of {authored}");
    // Turned to face aft, everything forward of the eye drops out.
    let eye = game.run.entity("eye").expect("the eye");
    game.run
        .world
        .get_mut(eye)
        .and_then(|data| data.transform_3d.as_mut())
        .expect("the eye has a transform")
        .rotation = [
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
    ];
    let (aft, _) = drawn(&mut game);
    assert!(
        aft * 4 < authored,
        "facing aft draws the cabin's end: {aft} of {authored}"
    );
}

#[test]
fn looking_out_of_the_hatch_draws_the_wreckage_ahead_and_not_the_ship_behind() {
    let mut game = Run::open_at([5.5, -0.6, 0.0]).expect("DRIFT opens");
    game.wait(0.3);
    let (out, authored) = drawn(&mut game);
    assert!(
        out * 2 < authored,
        "the cabin and airlock behind are not drawn: {out} of {authored}"
    );
}

#[test]
fn shuttle_vents_share_material_draws_per_section() {
    use glam::Vec3;
    use sindri_render::{look_at, orthographic_projection};

    let scene = std::fs::read_to_string(drift::project().join("assets/hold.scene")).unwrap();
    let mut document = SceneDocument::from_json(&scene).unwrap();
    document
        .entities
        .retain(|entity| entity.id.as_str() == "shuttle");
    let world = World::from_scene(&document).unwrap().world;
    let mut extractor = SceneExtractor::new().unwrap();
    let sets = bind_assets(&mut extractor, &world);
    let view = look_at(
        Vec3::new(20.0, 2.0, -4.0),
        Vec3::new(20.0, 0.0, -15.0),
        Vec3::Y,
    );
    let frame = extractor
        .extract_animated_with_world_camera(
            &world,
            Viewport::new(1280, 720),
            ViewCamera {
                view,
                view_projection: orthographic_projection(-10.0, 10.0, -6.0, 6.0, 0.1, 50.0) * view,
                framed_half_height: 6.0,
            },
            &TextureBindings::new(),
            SceneRuntime::default().with_tile_sets(&sets),
        )
        .unwrap();
    let batches: Vec<&RenderModel> = frame
        .passes()
        .iter()
        .filter_map(|pass| match &pass.command {
            FrameCommand::Model { asset, .. } => Some(asset.as_ref()),
            _ => None,
        })
        .collect();
    assert_eq!(batches.len(), 2, "six vents span two sections");
    let primitives: Vec<_> = batches
        .iter()
        .flat_map(|asset| &asset.data().meshes[0])
        .collect();
    assert_eq!(primitives.len(), 8, "one draw per material in each section");
    let triangles: usize = primitives
        .iter()
        .map(|primitive| match &primitive.indices {
            ModelIndexData::U16(indices) => indices.len() / 3,
            ModelIndexData::U32(indices) => indices.len() / 3,
        })
        .sum();
    assert_eq!(
        triangles,
        6 * 788,
        "every placed vent is present in the batches"
    );
}
