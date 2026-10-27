//! The art scenes open: they parse, every picture they draw exists and has the
//! sprite they name, every normal map is laid out like its picture, and the
//! autotiled ground of each is what its terrain would paint.
//!
//! `art/dusk/build.py` writes the scenes and everything they draw; CI re-runs
//! it and fails if the checked-in files move, so this holds the other half: what
//! the script wrote is something the engine can use.

use serde_json::Value;
use sindri_core::{SceneComponent, SceneDocument, SpriteSheetDocument};
use sindri_scene::{TilemapComponent, paint_terrain};
use std::path::PathBuf;

const SCENES: [(&str, &str); 2] = [("art-test.scene", "ground"), ("art-harbor.scene", "quay")];

fn assets() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
}

fn sheet(texture: &str) -> SpriteSheetDocument {
    let path = assets().join(texture).with_extension("sheet");
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{texture} has no sheet: {error}"));
    SpriteSheetDocument::from_json(&json).unwrap_or_else(|error| panic!("{texture}: {error}"))
}

fn size_of(texture: &str) -> (u32, u32) {
    let bytes = std::fs::read(assets().join(texture))
        .unwrap_or_else(|error| panic!("{texture} is missing: {error}"));
    assert_eq!(&bytes[1..4], b"PNG", "{texture} is not a PNG");
    let at = |from: usize| u32::from_be_bytes(bytes[from..from + 4].try_into().expect("a header"));
    (at(16), at(20))
}

fn scene(name: &str) -> Value {
    let json = std::fs::read_to_string(assets().join(name)).expect("the scene is checked in");
    SceneDocument::from_json(&json).unwrap_or_else(|error| panic!("{name} does not open: {error}"));
    serde_json::from_str(&json).expect("it is JSON")
}

fn components<'a>(scene: &'a Value, kind: &'a str) -> impl Iterator<Item = &'a Value> {
    scene["entities"]
        .as_array()
        .expect("entities")
        .iter()
        .filter_map(move |entity| entity["components"].get(kind))
}

#[test]
fn every_picture_an_art_scene_draws_exists_with_the_sprite_it_names() {
    for (name, _) in SCENES {
        let scene = scene(name);
        let mut sprites = 0;
        for sprite in components(&scene, "sindri.sprite") {
            let reference = sprite["texture"].as_str().expect("a texture");
            let (texture, part) = reference.split_once('#').unwrap_or((reference, ""));
            size_of(texture);
            if !part.is_empty() {
                let rects = sheet(texture).rects().expect("the sheet cuts");
                assert!(
                    rects.contains_key(part),
                    "{name}: {texture} has no `{part}`"
                );
            }
            sprites += 1;
        }
        let mut maps = 0;
        for map in components(&scene, "sindri.tilemap") {
            let map: TilemapComponent = serde_json::from_value(map.clone()).expect("a tilemap");
            let rects = sheet(&map.texture).rects().expect("the sheet cuts");
            for sprite in &map.palette {
                assert!(
                    rects.contains_key(sprite),
                    "{name}: {} has no `{sprite}`",
                    map.texture
                );
            }
            let normal = &map.normal_map;
            if !normal.is_empty() {
                assert_eq!(
                    size_of(normal),
                    size_of(&map.texture),
                    "{name}: {normal} is not laid out like {}",
                    map.texture
                );
            }
            maps += 1;
        }
        assert!(
            sprites >= 5 && maps >= 3,
            "{name}: {sprites} sprites, {maps} maps"
        );
    }
}

#[test]
fn the_autotiled_ground_is_what_its_terrain_paints() {
    for (name, terrain) in SCENES {
        let scene = scene(name);
        let level = scene["entities"]
            .as_array()
            .expect("entities")
            .iter()
            .find(|entity| entity["id"] == "level")
            .expect("a level");
        let payload = level["components"][TilemapComponent::TYPE_NAME].clone();
        let map: TilemapComponent = serde_json::from_value(payload.clone()).expect("it reads");
        let terrains = sheet(&map.texture).terrains;
        assert!(
            terrains[terrain].owns(&map.palette[0]),
            "{name}: its ground is the `{terrain}` terrain"
        );
        let mut painted = 0;
        for row in 0..map.rows {
            for column in 0..map.columns {
                if map.tiles[(row * map.columns + column) as usize].is_none() {
                    continue;
                }
                let mut repainted = payload.clone();
                let changed = paint_terrain(&mut repainted, &terrains, column, row, Some(terrain))
                    .expect("the terrain paints");
                assert!(
                    !changed,
                    "{name}: cell {column},{row} is not what it paints"
                );
                painted += 1;
            }
        }
        assert!(painted > 20, "{name}: {painted} ground cells");
    }
}

#[test]
fn the_playable_level_uses_the_dusk_art() {
    let level = scene("level-1.scene");
    let ground = level["entities"]
        .as_array()
        .expect("entities")
        .iter()
        .find(|entity| entity["id"] == "level")
        .expect("the playable ground");
    let map = &ground["components"]["sindri.tilemap"];
    assert_eq!(map["texture"], "textures/ground.png");
    assert_eq!(map["normal_map"], "textures/ground_n.png");

    let entities = level["entities"].as_array().expect("entities");
    let wick = entities
        .iter()
        .find(|entity| entity["id"] == "wick-sprite")
        .expect("Wick's visible sprite");
    assert_eq!(wick["parent"], "hero");
    assert_eq!(
        wick["components"]["sindri.sprite"]["texture"],
        "textures/wick-pixellab.png#idle"
    );

    let backdrop_sprites = components(&level, "sindri.sprite")
        .filter(|sprite| {
            let texture = sprite["texture"].as_str().unwrap_or("");
            texture == "textures/sky.png"
                || texture == "textures/hills-far.png"
                || texture == "textures/hills-near.png"
        })
        .count();
    assert_eq!(backdrop_sprites, 18, "six sprites for each parallax layer");
    for entity in entities.iter().filter(|entity| {
        let id = entity["id"].as_str().unwrap_or("");
        id.starts_with("dusk-sky-")
            || id.starts_with("dusk-hills-far-")
            || id.starts_with("dusk-hills-near-")
    }) {
        assert_eq!(
            entity["name"], entity["id"],
            "the backdrop script finds each layer by name"
        );
    }
}

#[test]
fn playable_wick_uses_a_foot_anchored_sheet_with_all_walk_frames() {
    let texture = "textures/wick-pixellab.png";
    assert_eq!(size_of(texture), (432, 48));
    let sheet_json = std::fs::read_to_string(assets().join(texture).with_extension("sheet"))
        .expect("Wick's sheet exists");
    let metadata: Value = serde_json::from_str(&sheet_json).expect("valid sheet JSON");
    assert_eq!(metadata["anchor"], "bottom");
    let rects = sheet(texture).rects().expect("the sheet cuts");
    assert!(rects.contains_key("idle"));
    for i in 0..8 {
        assert!(rects.contains_key(&format!("walk-{i}")));
    }
    let level = scene("level-1.scene");
    let wick = level["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entity| entity["id"] == "wick-sprite")
        .unwrap();
    let frames = wick["components"]["sindri.animation.sprite"]["clips"]["run"]["frames"]
        .as_array()
        .expect("the walk clip has frames");
    assert_eq!(frames.len(), 8);
    for frame in frames {
        assert!(rects.contains_key(frame.as_str().expect("a sprite name")));
    }
}
