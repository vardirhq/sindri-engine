//! The level's ground is painted with the `ground` terrain from
//! `textures/ground.sheet`: every cell of it is the sprite the terrain picks for
//! its neighbors, so repainting any of them changes nothing.

use platformer::Run;
use sindri_core::{SceneComponent, SpriteSheetDocument};
use sindri_scene::{TilemapComponent, paint_terrain};

#[test]
fn every_ground_cell_is_what_the_terrain_would_paint() {
    let sheet = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/textures/ground.sheet"
    ))
    .expect("the tiles have a sheet");
    let sheet = SpriteSheetDocument::from_json(&sheet).expect("the sheet parses");
    let ground = &sheet.terrains["ground"];

    let run = Run::open().expect("the project opens");
    let level = run
        .world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("Level"))
        .and_then(|(_, data)| data.components.get(TilemapComponent::TYPE_NAME))
        .expect("the level is a tilemap")
        .clone();
    let map: TilemapComponent = serde_json::from_value(level.clone()).expect("it reads");
    let mut painted = 0;
    for row in 0..map.rows {
        for column in 0..map.columns {
            let cell = (row * map.columns + column) as usize;
            let Some(sprite) = map.tiles[cell].map(|index| &map.palette[index as usize]) else {
                continue;
            };
            if !ground.owns(sprite) {
                continue;
            }
            let mut repainted = level.clone();
            let changed =
                paint_terrain(&mut repainted, &sheet.terrains, column, row, Some("ground"))
                    .expect("the terrain paints");
            assert!(
                !changed,
                "cell {column},{row} is not what the terrain paints"
            );
            painted += 1;
        }
    }
    assert!(
        painted > 100,
        "the level's ground is the terrain ({painted} cells)"
    );
}
