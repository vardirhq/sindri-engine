//! The old hero sprite's normal map remains a checked Normal Map Studio
//! artifact. Dusk Road now uses Wick's baked sprite and the meadow's generated
//! ground normal map, both checked in the playable scene.

use lamplighter::{Run, project};
use sindri_assets::normal_map::remake;

#[test]
fn the_heros_normal_map_is_the_one_its_settings_make() {
    let textures = project().join("assets/textures");
    let remade = remake(&textures.join("hero_n.normalmap")).expect("the settings make a map");
    assert_eq!(remade.map_path, textures.join("hero_n.png"));
    let shipped = std::fs::read(&remade.map_path).expect("the map is saved");
    assert!(
        remade.png == shipped,
        "hero_n.png is stale: open hero_n.normalmap in the studio and save"
    );
}

#[test]
fn the_playable_level_uses_wick_and_its_ground_normal_map() {
    let run = Run::open().expect("the project opens");
    let wick = run.entity("wick-sprite").expect("Wick's sprite");
    let sprite = &run.world.get(wick).expect("Wick exists").components["sindri.sprite"];
    assert_eq!(sprite["texture"], "textures/wick-pixellab.png#idle");
    let level = run.entity("level").expect("the ground");
    let tilemap = &run.world.get(level).expect("the level exists").components["sindri.tilemap"];
    assert_eq!(tilemap["normal_map"], "textures/ground_n.png");
}
