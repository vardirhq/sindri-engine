use std::collections::BTreeMap;

use serde_json::{Value, json};
use sindri_core::{TerrainDocument, TerrainKind};

use super::{TerrainPaintError, paint_corner};
use crate::paint_terrain;

fn dirt() -> BTreeMap<String, TerrainDocument> {
    BTreeMap::from([
        (
            "dirt".to_owned(),
            TerrainDocument {
                kind: TerrainKind::Corners,
                sprites: (0..16).map(|mask| format!("d{mask}")).collect(),
            },
        ),
        (
            "wall".to_owned(),
            TerrainDocument {
                kind: TerrainKind::Edges,
                sprites: (0..16).map(|mask| format!("w{mask}")).collect(),
            },
        ),
    ])
}

fn empty(columns: u32, rows: u32) -> Value {
    json!({
        "texture": "tiles.png",
        "palette": [],
        "columns": columns,
        "rows": rows,
        "tiles": vec![Value::Null; (columns * rows) as usize],
    })
}

fn sprite(map: &Value, column: usize, row: usize) -> Option<String> {
    let columns = usize::try_from(map["columns"].as_u64()?).ok()?;
    let index = usize::try_from(map["tiles"][row * columns + column].as_u64()?).ok()?;
    map["palette"][index].as_str().map(str::to_owned)
}

/// One corner touches four tiles, each of which sees it as a different one of
/// its own: the south-east of the tile north-west of it, and so on.
#[test]
fn a_corner_is_a_different_corner_of_each_tile_around_it() {
    let mut map = empty(2, 2);
    assert!(paint_corner(&mut map, &dirt(), 1, 1, "dirt", true).unwrap());
    assert_eq!(sprite(&map, 0, 0).as_deref(), Some("d1"), "south-east");
    assert_eq!(sprite(&map, 1, 0).as_deref(), Some("d2"), "south-west");
    assert_eq!(sprite(&map, 0, 1).as_deref(), Some("d4"), "north-east");
    assert_eq!(sprite(&map, 1, 1).as_deref(), Some("d8"), "north-west");
}

/// Painting a second corner joins onto what the first left in each tile.
#[test]
fn corners_accumulate_in_a_tile() {
    let mut map = empty(2, 1);
    paint_corner(&mut map, &dirt(), 1, 0, "dirt", true).unwrap();
    paint_corner(&mut map, &dirt(), 1, 1, "dirt", true).unwrap();
    assert_eq!(sprite(&map, 0, 0).as_deref(), Some("d5"), "its east side");
    assert_eq!(sprite(&map, 1, 0).as_deref(), Some("d10"), "its west side");
}

#[test]
fn clearing_a_corner_undoes_it_and_a_repeat_changes_nothing() {
    let mut map = empty(2, 2);
    paint_corner(&mut map, &dirt(), 1, 1, "dirt", true).unwrap();
    assert!(!paint_corner(&mut map, &dirt(), 1, 1, "dirt", true).unwrap());
    assert!(paint_corner(&mut map, &dirt(), 1, 1, "dirt", false).unwrap());
    assert_eq!(
        sprite(&map, 0, 0).as_deref(),
        Some("d0"),
        "back to the ground"
    );
    assert!(!paint_corner(&mut map, &dirt(), 0, 0, "dirt", false).unwrap());
}

/// A corner on the map's own edge reaches only the tiles that exist.
#[test]
fn a_corner_on_the_edge_reaches_one_tile() {
    let mut map = empty(2, 2);
    paint_corner(&mut map, &dirt(), 0, 0, "dirt", true).unwrap();
    assert_eq!(sprite(&map, 0, 0).as_deref(), Some("d8"));
    assert_eq!(sprite(&map, 1, 0), None);
    paint_corner(&mut map, &dirt(), 2, 2, "dirt", true).unwrap();
    assert_eq!(sprite(&map, 1, 1).as_deref(), Some("d1"));
}

#[test]
fn another_terrain_keeps_its_tiles() {
    let mut map = empty(2, 1);
    paint_terrain(&mut map, &dirt(), 0, 0, Some("wall")).unwrap();
    paint_corner(&mut map, &dirt(), 1, 0, "dirt", true).unwrap();
    assert_eq!(
        sprite(&map, 0, 0).as_deref(),
        Some("w13"),
        "the wall is untouched"
    );
    assert_eq!(sprite(&map, 1, 0).as_deref(), Some("d8"));
}

#[test]
fn the_wrong_kind_and_the_wrong_place_are_refused() {
    let mut map = empty(2, 2);
    assert_eq!(
        paint_corner(&mut map, &dirt(), 0, 0, "wall", true),
        Err(TerrainPaintError::ByCell("wall".to_owned()))
    );
    assert_eq!(
        paint_terrain(&mut map, &dirt(), 0, 0, Some("dirt")),
        Err(TerrainPaintError::ByCorner("dirt".to_owned()))
    );
    assert_eq!(
        paint_corner(&mut map, &dirt(), 0, 0, "lava", true),
        Err(TerrainPaintError::Unknown("lava".to_owned()))
    );
    assert!(matches!(
        paint_corner(&mut map, &dirt(), 3, 0, "dirt", true),
        Err(TerrainPaintError::Outside { .. })
    ));
}
