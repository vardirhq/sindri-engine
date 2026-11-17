//! What stands in the wilds: Wick goes behind the ground cover as he goes
//! behind anything else that stands, and what blocks him is fitted to where
//! each thing meets the ground (`art/fit.py`), not its whole picture.

use std::path::Path;

fn prefab(name: &str) -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/prefabs/wilds")
        .join(format!("{name}.prefab"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{name} is a prefab"));
    let doc: serde_json::Value = serde_json::from_str(&text).expect("it reads");
    doc["entities"][0]["components"].clone()
}

#[test]
fn ground_cover_stands_with_wick_so_he_goes_behind_it() {
    let wick_layer = 5;
    for cover in ["tufts", "flowers", "pebbles", "mushrooms"] {
        assert_eq!(
            prefab(cover)["sindri.sprite"]["layer"],
            wick_layer,
            "{cover} is ordered against Wick by distance"
        );
    }
}

#[test]
fn what_stands_is_blocked_where_it_meets_the_ground() {
    for name in [
        "oak-1",
        "tree-1",
        "bush-2",
        "dead-tree-3",
        "boulder-1",
        "hamlet/well",
        "hamlet/bakery",
    ] {
        let parts = prefab(name);
        let collider = &parts["sindri.physics2d.collider"];
        let shape = &collider["shape"];
        let kind = shape["shape"].as_str().expect("a shape");
        assert!(
            kind == "capsule" || kind == "circle",
            "{name}: a footprint, not a box: {shape}"
        );
        let radius = shape["radius"].as_f64().expect("a radius");
        let across = radius + shape["half_height"].as_f64().unwrap_or(0.0);
        // A footprint lies on the ground: no deeper than it is wide, and its
        // middle low on the picture, near where the thing stands.
        assert!(radius <= across + 1e-6, "{name}: shallower than it is wide");
        let up = collider["offset"][1].as_f64().expect("an offset");
        assert!(
            up.abs() < 0.12,
            "{name}: at its foot, not up its picture: {up}"
        );
        assert!(
            parts["sindri.light2d.occluder"]["points"]
                .as_array()
                .map(Vec::len)
                == Some(12),
            "{name}: its shadow is cast from the same footprint"
        );
    }
}
