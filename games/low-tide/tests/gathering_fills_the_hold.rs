//! Harvesting changes the Basin and carries typed, weighted cargo home.
mod pilot;

use pilot::{Pilot, column_row};
use sindri_scene::{TilemapComponent, VoxelBlock, VoxelEdit, VoxelWorldComponent};

fn patch(pilot: &mut Pilot, block: &str, level: i32) -> [i32; 3] {
    let basin = pilot.run.entity("basin").unwrap();
    let (column, row) = column_row(pilot.crew_world());
    let mut terrain = pilot
        .run
        .components
        .get::<VoxelWorldComponent>(&pilot.run.world, basin)
        .unwrap()
        .unwrap();
    // Isolate the target from other harvestable surfaces. High edits exercise
    // removal back to generated air as well as removal of natural blocks.
    for z in row - 1..=row + 1 {
        for x in column - 1..=column + 1 {
            for y in level - 1..=level {
                terrain.edits.push(VoxelEdit {
                    at: [x, y, z],
                    block: VoxelBlock::Named("mud".into()),
                });
            }
        }
    }
    let at = [column, level, row];
    terrain.edits.push(VoxelEdit {
        at,
        block: VoxelBlock::Named(block.into()),
    });
    pilot.run.world.get_mut(basin).unwrap().components.insert(
        "sindri.voxel_world".into(),
        serde_json::to_value(terrain).unwrap(),
    );
    pilot.wait(0.1);
    at
}

fn ashore(pilot: &mut Pilot) {
    pilot.walk_deck(&[[6.5, -7.5], [10.0, -7.5]]);
    assert!(!pilot.flag("aboard"));
}

fn bring_home(pilot: &mut Pilot) {
    let foot = pilot.deck_to_world([10.5, -7.5]);
    pilot.walk_ashore(foot, 0.3);
    let ramp = pilot.deck_to_world([8.4, -7.5]);
    pilot.walk_ashore(ramp, 0.2);
    pilot.wait(0.2);
    assert!(pilot.flag("aboard"));
}

fn harvest(mut pilot: Pilot, block: &str, kind: &str, tile: u32) {
    ashore(&mut pilot);
    let at = patch(&mut pilot, block, 10);
    assert_eq!(pilot.ground().block(at), block);
    pilot.use_it();
    let carried = pilot.run.entity("carried").unwrap();
    assert!(pilot.run.world.is_active(carried), "carrying {kind}");
    assert_eq!(pilot.ground().block(at), "", "the harvested block is gone");
    assert!(pilot.board(kind).abs() < 0.01, "not yet stowed");
    // Holding a bundle refuses a second harvest rather than erasing extra terrain.
    let after = pilot.ground().surface(at[0], at[2]);
    pilot.use_it();
    assert_eq!(pilot.ground().surface(at[0], at[2]), after);
    bring_home(&mut pilot);
    assert!(!pilot.run.world.is_active(carried));
    assert!((pilot.board(kind) - 1.0).abs() < 0.01);
    assert!((pilot.board("crates") - 1.0).abs() < 0.01);
    let deck = pilot.run.entity("deck").unwrap();
    let hold = pilot
        .run
        .components
        .get::<TilemapComponent>(&pilot.run.world, deck)
        .unwrap()
        .unwrap();
    assert_eq!(hold.tile(4, 10), Some(tile));
    assert!(
        pilot.board("top_speed") < 4.2 * pilot.board("going") - 0.2,
        "cargo has weight"
    );
    pilot::salvage::jettison_it(&mut pilot);
    assert!(
        pilot.board(kind).abs() < 0.01,
        "the right inventory count is reduced"
    );
}

#[test]
fn biome_resources_are_harvested_stowed_and_jettisoned_with_keys_and_touch() {
    for (block, kind, tile) in [
        ("trunk", "wood", 16),
        ("reef", "stone", 17),
        ("kelp", "fibre", 19),
        ("salt", "salt", 20),
    ] {
        harvest(Pilot::new(), block, kind, tile);
        harvest(Pilot::on_a_phone(), block, kind, tile);
    }
}

#[test]
fn full_holds_and_flooded_resources_refuse_harvesting() {
    let mut pilot = Pilot::new();
    // Raise this fixture's high-water mark above the authored resource patch.
    // Configure before Tide starts, as an editor-authored export would.
    let basin = pilot.run.entity("basin").unwrap();
    pilot
        .run
        .world
        .get_mut(basin)
        .unwrap()
        .components
        .get_mut("sindri.script")
        .unwrap()["properties"]["high_level"] = serde_json::json!(50.0);
    ashore(&mut pilot);
    let at = patch(&mut pilot, "salt", 10);
    assert_eq!(pilot.surface_under(pilot.crew_world()), "salt");
    pilot.run.scripts.blackboard_mut().set("crates", 4.0);
    pilot.use_it();
    assert_eq!(pilot.ground().block(at), "salt");
    let carried = pilot.run.entity("carried").unwrap();
    assert!(!pilot.run.world.is_active(carried));
    pilot.run.scripts.blackboard_mut().set("crates", 0.0);
    pilot.run.scripts.blackboard_mut().set("tide_time", 280.0);
    pilot.wait(0.1);
    pilot.use_it();
    assert_eq!(pilot.ground().block(at), "salt");
    assert!(!pilot.run.world.is_active(carried));
}

#[test]
fn rock_can_supply_ore_with_keys_and_touch() {
    for mut pilot in [Pilot::new(), Pilot::on_a_phone()] {
        ashore(&mut pilot);
        let mut found = false;
        for _ in 0..16 {
            patch(&mut pilot, "rock", 10);
            // Resource.Ore is the third declared variant; observe the game,
            // rather than duplicate the script's procedural resource formula.
            if (pilot.board("gather_kind") - 3.0).abs() < 0.01 {
                found = true;
                break;
            }
            let at = pilot.crew_world();
            pilot.walk_ashore([at[0] + 1.0, at[1]], 0.05);
        }
        assert!(found, "an ore-bearing column is reachable");
        pilot.use_it();
        bring_home(&mut pilot);
        assert!((pilot.board("ore") - 1.0).abs() < 0.01);
        assert!(pilot.board("stone").abs() < 0.01);
        let deck = pilot.run.entity("deck").unwrap();
        let hold = pilot
            .run
            .components
            .get::<TilemapComponent>(&pilot.run.world, deck)
            .unwrap()
            .unwrap();
        assert_eq!(hold.tile(4, 10), Some(18));
    }
}

#[test]
fn a_flood_washes_the_resource_that_was_stowed() {
    let mut pilot = Pilot::new();
    ashore(&mut pilot);
    patch(&mut pilot, "trunk", 10);
    pilot.use_it();
    bring_home(&mut pilot);
    assert!((pilot.board("wood") - 1.0).abs() < 0.01);
    pilot.run.scripts.blackboard_mut().set("tide_time", 280.0);
    pilot.wait(0.2);
    assert!(pilot.flag("flooded"));
    assert!(pilot.board("wood").abs() < 0.01);
    assert!(pilot.board("crates").abs() < 0.01);
    assert!((pilot.board("washed") - 1.0).abs() < 0.01);
    assert!(pilot.board("scrap").abs() < 0.01);
}
