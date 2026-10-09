//! Real Orbital scripts retain their filters when the host does the sorting.

use orbital_baked::Run;
use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent, TagsComponent, Transform3D};
use sindri_decay::ScriptComponent;

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run) {
    let notes = run.step(STEP);
    assert!(notes.is_empty(), "{notes:#?}");
}

fn isolated_run(keep: Option<&str>) -> Run {
    let mut run = Run::open().expect("project opens");
    step(&mut run);
    let disable: Vec<_> = run
        .world
        .entities()
        .filter_map(|(id, data)| {
            (data.components.contains_key(ScriptComponent::TYPE_NAME)
                && (keep.is_none() || data.name.as_deref() != keep))
                .then_some(id)
        })
        .collect();
    for id in disable {
        run.world.get_mut(id).expect("entity").disabled = true;
    }
    run.set_board("player_reset", 0.0);
    run.set_board("run_state", 1.0);
    run.set_board("nullified", 1.0);
    run
}

fn enemy(run: &mut Run, position: [f32; 3]) -> EntityId {
    run.world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position,
            scale: [0.2, 0.2, 1.0],
            ..Transform3D::default()
        }),
        components: [(
            TagsComponent::TYPE_NAME.to_owned(),
            json!({ "tags": ["enemy"] }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    })
}

#[test]
fn player_picks_the_nearest_visible_enemy_even_when_offscreen_is_closer() {
    let mut run = isolated_run(Some("Player"));
    let player = run.find("Player").expect("player");
    run.world
        .get_mut(player)
        .expect("entity")
        .transform_3d
        .as_mut()
        .expect("transform")
        .position = [9.0, 0.0, 0.0];
    enemy(&mut run, [10.5, 0.0, 0.0]);
    let visible = enemy(&mut run, [7.0, 0.0, 0.0]);
    let farther = enemy(&mut run, [4.0, 0.0, 0.0]);
    step(&mut run);
    assert!((run.board("target_live") - 1.0).abs() < 1.0e-5);
    assert!((run.board("target_x") - 7.0).abs() < 1.0e-5);
    // Once no acceptable target remains, do not fire at the closest offscreen one.
    run.world.get_mut(visible).expect("entity").disabled = true;
    run.world.get_mut(farther).expect("entity").disabled = true;
    step(&mut run);
    assert!(run.board("target_live").abs() < 1.0e-5);
}

#[test]
fn arc_skips_impact_targets_and_moves_toward_the_next_nearest_enemy() {
    let mut run = isolated_run(None);
    enemy(&mut run, [0.0; 3]);
    enemy(&mut run, [0.2, 0.0, 0.0]);
    enemy(&mut run, [3.0, 0.0, 0.0]);
    enemy(&mut run, [0.0, 4.0, 0.0]);
    let prefab = run
        .session
        .prefabs()
        .get("prefabs/arc.prefab")
        .expect("arc")
        .clone();
    let arc = run.world.spawn_prefab(&prefab).expect("spawn").root;
    step(&mut run);
    step(&mut run);
    let position = run
        .world
        .world_transform(arc)
        .expect("arc still alive")
        .position;
    assert!(
        position[0] > 0.0,
        "arc did not move to the next target: {position:?}"
    );
    assert!(
        position[1].abs() < 1.0e-5,
        "arc chose the farther target: {position:?}"
    );
}

#[test]
fn player_uses_the_nearest_enemy_when_it_is_visible() {
    let mut run = isolated_run(Some("Player"));
    enemy(&mut run, [3.0, 0.0, 0.0]);
    enemy(&mut run, [1.0, 0.0, 0.0]);
    step(&mut run);
    assert!((run.board("target_live") - 1.0).abs() < 1.0e-5);
    assert!((run.board("target_x") - 1.0).abs() < 1.0e-5);
}
