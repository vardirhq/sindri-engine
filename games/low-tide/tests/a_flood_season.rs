//! The sea returns and leaves again; it costs cargo and time, never a life.
mod pilot;

use pilot::{Pilot, distance};
use sindri_scene::{VoxelGround, VoxelWorldComponent};

fn time(pilot: &mut Pilot, seconds: f64) {
    pilot.run.scripts.blackboard_mut().set("tide_time", seconds);
    pilot.wait(0.1);
}

fn level(pilot: &Pilot) -> f32 {
    let basin = pilot.run.entity("basin").unwrap();
    pilot
        .run
        .components
        .get::<VoxelWorldComponent>(&pilot.run.world, basin)
        .unwrap()
        .unwrap()
        .map_flood
        .unwrap()
        .level
}

#[test]
fn the_season_rises_holds_recedes_and_repeats_without_changing_the_basin() {
    let mut pilot = Pilot::new();
    pilot.wait(0.1);
    let basin = pilot.run.entity("basin").unwrap();
    let before = pilot.run.world.get(basin).unwrap().components["sindri.voxel_world"].clone();
    let start = pilot.crawler_world();
    let surface = pilot.surface_under(start);
    assert!((level(&pilot) + 17.0).abs() < 0.01);
    time(&mut pilot, 225.0);
    assert!((level(&pilot) + 10.5).abs() < 0.05, "half risen");
    time(&mut pilot, 280.0);
    assert!((level(&pilot) + 4.0).abs() < 0.01, "high water");
    assert!(pilot.flag("flooded"));
    assert!(pilot.flag("blocked"));
    assert!(distance(start, pilot.crawler_world()) < 0.01);
    time(&mut pilot, 360.0);
    assert!((level(&pilot) + 10.5).abs() < 0.05, "half fallen");
    time(&mut pilot, 405.0);
    assert!(!pilot.flag("flooded"), "dry again");
    assert!(pilot.flag("aboard"), "alive and aboard");
    assert_eq!(surface, pilot.surface_under(start));
    let after = &pilot.run.world.get(basin).unwrap().components["sindri.voxel_world"];
    assert_eq!(before["generator"], after["generator"]);
    assert_eq!(before["edits"], after["edits"]);
    time(&mut pilot, 685.0);
    assert!(pilot.flag("flooded"), "the next season returns");
}

#[test]
fn flooding_washes_one_crate_per_season_and_the_crawler_can_drive_after_the_ebb() {
    let mut pilot = Pilot::new();
    pilot::salvage::salvage_the_first_wreck(&mut pilot);
    let before = pilot.crawler_world();
    time(&mut pilot, 280.0);
    assert!(pilot.flag("flooded"));
    assert!(pilot.board("scrap").abs() < 0.01);
    assert!(pilot.board("crates").abs() < 0.01);
    assert!((pilot.board("washed") - 1.0).abs() < 0.01);
    pilot.take_the_helm();
    pilot.set_throttle(1.0);
    pilot.wait(3.0);
    assert!(
        distance(before, pilot.crawler_world()) < 0.01,
        "trapped until the ebb"
    );
    assert!(
        (pilot.board("washed") - 1.0).abs() < 0.01,
        "not charged every frame"
    );
    time(&mut pilot, 405.0);
    pilot.wait(3.0);
    assert!(
        distance(before, pilot.crawler_world()) > 1.0,
        "drives again"
    );
}

fn caught_ashore(mut pilot: Pilot) {
    pilot.walk_deck(&[[7.5, -6.5], [12.0, -6.5]]);
    assert!(!pilot.flag("aboard"));
    let from = pilot.crew_world();
    time(&mut pilot, 280.0);
    pilot.push([1.0, 0.0]);
    for _ in 0..60 {
        pilot.step();
    }
    pilot.release();
    let moved = distance(from, pilot.crew_world());
    assert!(moved > 0.5 && moved < 2.0, "can wade slowly out: {moved}");
    time(&mut pilot, 405.0);
    let foot = pilot.deck_to_world([12.5, -6.5]);
    pilot.walk_ashore(foot, 0.3);
    let ramp = pilot.deck_to_world([10.4, -6.5]);
    pilot.walk_ashore(ramp, 0.2);
    assert!(pilot.flag("aboard"), "can come home after the flood");
}

#[test]
fn caught_crew_can_escape_and_come_home() {
    caught_ashore(Pilot::new());
}

#[test]
fn caught_crew_can_escape_on_a_phone() {
    caught_ashore(Pilot::on_a_phone());
}

#[test]
fn the_same_overlay_leaves_builtin_overland_blocks_intact() {
    let mut component: VoxelWorldComponent = serde_json::from_str("{}").unwrap();
    component.generator = sindri_scene::VoxelGeneratorDocument::NaturalTerrain(
        sindri_scene::NaturalTerrainDocument::with_builtin_blocks(),
    );
    component.blocks = Some("builtin:blocks".into());
    component.view = sindri_scene::VoxelView::Map;
    let mut sets = sindri_scene::TileSetBindings::new();
    sets.bind(
        "builtin:blocks",
        sindri_core::TileSetDocument::from_json(include_str!(
            "../../../crates/sindri-assets/builtin/blocks/blocks.tileset"
        ))
        .unwrap(),
    )
    .unwrap();
    let before = VoxelGround::of(&component, Some(&sets)).unwrap();
    component.map_flood = Some(sindri_scene::VoxelMapFlood {
        level: 100.0,
        block: sindri_scene::VoxelBlock::Named("water".into()),
    });
    let after = VoxelGround::of(&component, Some(&sets)).unwrap();
    assert_eq!(before.surface(0, 0), after.surface(0, 0));
    assert_eq!(before.block([0, 0, 0]), after.block([0, 0, 0]));
}

#[test]
fn carried_salvage_washes_away_and_flooded_wrecks_cannot_be_harvested() {
    let mut pilot = Pilot::new();
    pilot::salvage::pick_up_at_first_wreck(&mut pilot);
    let carried = pilot.run.entity("carried").unwrap();
    assert!(pilot.run.world.is_active(carried));
    time(&mut pilot, 280.0);
    assert!(
        !pilot.run.world.is_active(carried),
        "caught salvage washes away"
    );
    assert!((pilot.board("washed") - 1.0).abs() < 0.01);
    pilot.use_it();
    assert!(
        !pilot.run.world.is_active(carried),
        "no harvesting underwater"
    );
    pilot.wait(1.0);
    assert!(
        (pilot.board("washed") - 1.0).abs() < 0.01,
        "only the carried crate lost"
    );
    time(&mut pilot, 405.0);
    pilot.use_it();
    assert!(
        pilot.run.world.is_active(carried),
        "harvesting returns with low tide"
    );
}
