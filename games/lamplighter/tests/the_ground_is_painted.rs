//! The ground is grass until `ground.decay` paints a dirt path on it when the
//! world opens, one corner at a time on the sheet's `dirt` corners terrain.

use lamplighter::Run;

const STEP: f32 = 1.0 / 60.0;

/// The sprite a cell of the ground draws, by column and row from its top-left.
fn sprite(run: &Run, column: usize, row: usize) -> String {
    let ground = run.entity("ground").expect("the world has ground");
    let data = run.world.get(ground).expect("it exists");
    let map = &data.components["sindri.tilemap"];
    let columns = usize::try_from(map["columns"].as_u64().expect("columns")).expect("fits");
    let tile = usize::try_from(
        map["tiles"][row * columns + column]
            .as_u64()
            .expect("a tile"),
    )
    .expect("fits");
    map["palette"][tile].as_str().expect("a sprite").to_owned()
}

#[test]
fn the_path_is_dirt_and_the_far_fields_stay_grass() {
    let mut run = Run::open().expect("the project opens");
    for _ in 0..5 {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
    // The map's top-left is (-26, 18); Wick starts at the origin, on a path
    // plaza, which is column 26 and row 18.
    assert_eq!(sprite(&run, 26, 18), "ground-15", "dirt where Wick starts");
    assert_eq!(sprite(&run, 2, 2), "ground-0", "grass in the far corner");
    // Beside the path the tiles are edges, neither grass nor all dirt.
    let beside: Vec<String> = (0..36).map(|row| sprite(&run, 26, row)).collect();
    assert!(
        beside
            .iter()
            .any(|name| name != "ground-0" && name != "ground-15"),
        "the path has edges: {beside:?}"
    );
}

/// The sprite a cell of the paving layer draws, if any.
fn paving(run: &Run, column: usize, row: usize) -> Option<String> {
    let road = run.entity("road").expect("the world has a paving layer");
    let data = run.world.get(road).expect("it exists");
    let map = &data.components["sindri.tilemap"];
    let columns = usize::try_from(map["columns"].as_u64().expect("columns")).expect("fits");
    let tile = usize::try_from(map["tiles"][row * columns + column].as_u64()?).ok()?;
    map["palette"][tile].as_str().map(str::to_owned)
}

#[test]
fn each_lamp_stands_on_paving_and_the_far_fields_have_none() {
    let mut run = Run::open().expect("the project opens");
    for _ in 0..5 {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
    // The map's top-left is (-26, 18). The beacon lamp is at (22, 8), so the tile
    // whose corners are all within its pad is column 47, row 9; the first lamp at
    // (8, 0) has its own, at column 33, row 17.
    assert_eq!(
        paving(&run, 47, 9).as_deref(),
        Some("road-15"),
        "the beacon's pad"
    );
    assert_eq!(
        paving(&run, 33, 17).as_deref(),
        Some("road-15"),
        "the first lamp's pad"
    );
    assert_eq!(paving(&run, 2, 2), None, "no paving in the far corner");
    assert_eq!(
        paving(&run, 26, 18),
        None,
        "none where Wick starts, which has no lamp"
    );
}
