//! Controlled terrain for playing the gathering-to-construction loop.
use super::{Pilot, column_row};
use sindri_scene::{VoxelBlock, VoxelEdit, VoxelWorldComponent};

pub fn patch(pilot: &mut Pilot, block: &str) {
    let basin = pilot.run.entity("basin").unwrap();
    let (cx, cz) = column_row(pilot.crew_world());
    let mut terrain = pilot
        .run
        .components
        .get::<VoxelWorldComponent>(&pilot.run.world, basin)
        .unwrap()
        .unwrap();
    for z in cz - 1..=cz + 1 {
        for x in cx - 1..=cx + 1 {
            terrain.edits.push(VoxelEdit {
                at: [x, 10, z],
                block: VoxelBlock::Named("mud".into()),
            });
        }
    }
    terrain.edits.push(VoxelEdit {
        at: [cx, 10, cz],
        block: VoxelBlock::Named(block.into()),
    });
    pilot.run.world.get_mut(basin).unwrap().components.insert(
        "sindri.voxel_world".into(),
        serde_json::to_value(terrain).unwrap(),
    );
    pilot.wait(0.1);
}

pub fn home(pilot: &mut Pilot) {
    let foot = pilot.deck_to_world([10.5, -7.5]);
    pilot.walk_ashore(foot, 0.3);
    let ramp = pilot.deck_to_world([8.4, -7.5]);
    pilot.walk_ashore(ramp, 0.2);
    pilot.wait(0.1);
    assert!(pilot.flag("aboard"));
}

pub fn gather(pilot: &mut Pilot, block: &str) {
    pilot.walk_deck(&[[5.5, -7.5], [6.5, -7.5], [10.0, -7.5]]);
    assert!(!pilot.flag("aboard"));
    patch(pilot, block);
    pilot.use_it();
    let carried = pilot.run.entity("carried").unwrap();
    assert!(pilot.run.world.is_active(carried));
    home(pilot);
}

pub fn bench(pilot: &mut Pilot) {
    pilot.walk_deck(&[[5.5, -7.5], [5.5, -6.5]]);
    pilot.use_it();
    pilot.wait(0.1);
    assert!(pilot.flag("building"));
}

pub fn next(pilot: &mut Pilot) {
    if pilot.touch() {
        pilot.press("next-button");
    } else {
        pilot.tap(sindri_platform::Key::Q);
    }
    pilot.wait(0.1);
}

pub fn close(pilot: &mut Pilot) {
    if pilot.touch() {
        pilot.press("drop-button");
    } else {
        pilot.tap(sindri_platform::Key::X);
    }
    pilot.wait(0.1);
    assert!(!pilot.flag("building"));
}
