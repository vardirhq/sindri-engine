//! The hero is lit through a normal map the editor's Normal Map Studio made.
//!
//! `textures/hero_n.png` was not drawn: the studio derived it from
//! `hero.png`, cell by cell, and saved the settings it used beside it as
//! `hero_n.normalmap`. Made again from those settings and today's art, it is
//! the file the game ships, so the map can never drift from the sprite
//! without this failing; when the art changes, opening the settings file in
//! the studio and saving makes it again.

use platformer::{Run, project};
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
fn the_hero_names_the_studio_made_map() {
    let run = Run::open().expect("the project opens");
    let hero = run.entity("hero").expect("the hero");
    let sprite = &run.world.get(hero).expect("the hero exists").components["sindri.sprite"];
    assert_eq!(sprite["normal_map"], "textures/hero_n.png");
}
