use std::collections::BTreeMap;

use serde_json::{Value, json};
use sindri_core::{TerrainDocument, TerrainKind};

use super::{TerrainPaintError, paint_terrain};

fn ground() -> BTreeMap<String, TerrainDocument> {
    BTreeMap::from([(
        "ground".to_owned(),
        TerrainDocument {
            kind: TerrainKind::Edges,
            sprites: (0..16).map(|mask| format!("g{mask}")).collect(),
        },
    )])
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

/// Two cells side by side join: the left one's east side and the right
/// one's west side are the same terrain.
#[test]
fn neighbors_join() {
    let mut map = empty(5, 3);
    paint_terrain(&mut map, &ground(), 1, 1, Some("ground")).unwrap();
    assert_eq!(sprite(&map, 1, 1).as_deref(), Some("g0"), "alone");
    paint_terrain(&mut map, &ground(), 2, 1, Some("ground")).unwrap();
    assert_eq!(sprite(&map, 1, 1).as_deref(), Some("g2"), "east joined");
    assert_eq!(sprite(&map, 2, 1).as_deref(), Some("g8"), "west joined");
}

/// Ground painted to the map's edge carries on past it.
#[test]
fn the_map_edge_is_more_of_the_same() {
    let mut map = empty(2, 2);
    paint_terrain(&mut map, &ground(), 0, 1, Some("ground")).unwrap();
    // Open to the north and east, where the map has cells; joined west and
    // south, where it has none.
    assert_eq!(sprite(&map, 0, 1).as_deref(), Some("g12"));
}

/// Emptying a cell redraws the neighbors that leaned on it.
#[test]
fn erasing_redraws_the_neighbors() {
    let mut map = empty(3, 5);
    for row in 1..4 {
        paint_terrain(&mut map, &ground(), 1, row, Some("ground")).unwrap();
    }
    assert_eq!(sprite(&map, 1, 2).as_deref(), Some("g5"), "north and south");
    assert!(paint_terrain(&mut map, &ground(), 1, 3, None).unwrap());
    assert_eq!(sprite(&map, 1, 2).as_deref(), Some("g1"), "only north now");
    assert!(!paint_terrain(&mut map, &ground(), 1, 3, None).unwrap());
}

#[test]
fn an_unknown_terrain_is_refused() {
    let mut map = empty(1, 1);
    assert_eq!(
        paint_terrain(&mut map, &ground(), 0, 0, Some("lava")),
        Err(TerrainPaintError::Unknown("lava".to_owned()))
    );
}
