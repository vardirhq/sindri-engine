//! Streaming creates deterministic wrecks and preserves salvage on revisits.
mod pilot;

use pilot::{Pilot, column_row, distance};
use sindri_core::EntityId;
use sindri_decay::ScriptValue;
use sindri_scene::{VoxelBlock, VoxelEdit, VoxelWorldComponent};

fn wrecks(pilot: &Pilot) -> Vec<EntityId> {
    pilot
        .run
        .world
        .entities()
        .filter(|(_, data)| data.name.as_deref() == Some("Basin wreck"))
        .map(|(entity, _)| entity)
        .collect()
}

fn travel(pilot: &mut Pilot, at: [f32; 2]) {
    // Move the test host's observer between distant regions; actual harvesting
    // and ramp traversal are separately played with keys and touch.
    pilot
        .run
        .world
        .get_mut(pilot.crew)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [at[0], at[1], 0.3];
    pilot.wait(1.2);
}

fn exploring() -> Pilot {
    let mut pilot = Pilot::new();
    pilot.walk_deck(&[[6.5, -7.5], [10.0, -7.5]]);
    assert!(!pilot.flag("aboard"));
    travel(&mut pilot, [400.5, 400.5]);
    pilot
}

#[test]
fn procedural_wrecks_are_stable_dry_and_bounded() {
    let mut first = exploring();
    let second = exploring();
    let positions = |pilot: &Pilot| {
        wrecks(pilot)
            .iter()
            .map(|entity| pilot.run.position(*entity))
            .collect::<Vec<_>>()
    };
    let sites = positions(&first);
    assert!(!sites.is_empty(), "wrecks exist far beyond the start");
    assert_eq!(
        sites,
        positions(&second),
        "the same regions give the same sites"
    );
    for at in &sites {
        assert_ne!(first.surface_under(*at), "brine", "on dry ground");
    }
    for at in [[800.5, 400.5], [800.5, 800.5], [-400.5, -400.5]] {
        travel(&mut first, at);
        let nearby = wrecks(&first);
        assert!(
            nearby.len() <= 9,
            "no more than the surrounding nine regions"
        );
        for wreck in nearby {
            assert!(
                distance(first.run.position(wreck), at) < 120.0,
                "old wrecks unload"
            );
        }
    }
}

#[test]
fn returning_to_a_streamed_wreck_does_not_refill_scrap() {
    let mut pilot = exploring();
    let wreck = wrecks(&pilot)[0];
    let site = pilot.run.position(wreck);
    travel(&mut pilot, site);
    pilot.use_it();
    assert_eq!(
        pilot.run.scripts.field(wreck, "scrap"),
        Some(&ScriptValue::Number(2.0))
    );
    let carried = pilot.run.entity("carried").unwrap();
    assert!(pilot.run.world.is_active(carried));
    travel(&mut pilot, [site[0] + 300.0, site[1]]);
    assert!(pilot.run.world.get(wreck).is_none(), "unloaded");
    // Harvesting/building edits must not reroll a visited region's wreck site.
    let basin = pilot.run.entity("basin").unwrap();
    let mut terrain = pilot
        .run
        .components
        .get::<VoxelWorldComponent>(&pilot.run.world, basin)
        .unwrap()
        .unwrap();
    let (column, row) = column_row(site);
    terrain.edits.push(VoxelEdit {
        at: [column, 100, row],
        block: VoxelBlock::Named("rock".into()),
    });
    pilot.run.world.get_mut(basin).unwrap().components.insert(
        "sindri.voxel_world".into(),
        serde_json::to_value(terrain).unwrap(),
    );
    travel(&mut pilot, site);
    let returned = wrecks(&pilot)
        .into_iter()
        .find(|entity| distance(pilot.run.position(*entity), site) < 0.01)
        .expect("the same wreck returns");
    assert_eq!(
        pilot.run.scripts.field(returned, "scrap"),
        Some(&ScriptValue::Number(2.0)),
        "no free refill"
    );
}
