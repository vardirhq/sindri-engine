//! Wreck cargo is physical, selectable and recoverable after setting it down.
mod pilot;

use pilot::Pilot;
use sindri_core::EntityId;
use sindri_decay::ScriptValue;
use sindri_platform::Key;

fn field(pilot: &Pilot, entity: EntityId, name: &str) -> f64 {
    match pilot.run.scripts.field(entity, name) {
        Some(ScriptValue::Number(value)) => *value,
        other => panic!("numeric {name}: {other:?}"),
    }
}

fn crates(pilot: &Pilot) -> Vec<EntityId> {
    pilot
        .run
        .world
        .entities()
        .filter(|(_, data)| data.name.as_deref() == Some("Loose cargo"))
        .map(|(entity, _)| entity)
        .collect()
}

fn stand_at(pilot: &mut Pilot, at: [f32; 2]) {
    pilot
        .run
        .world
        .get_mut(pilot.crew)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [at[0], at[1], 0.3];
    pilot.wait(0.1);
}

fn drop(pilot: &mut Pilot) {
    if pilot.touch() {
        pilot.press("drop-button");
    } else {
        pilot.tap(Key::X);
    }
    pilot.wait(0.1);
}

#[test]
fn choose_wood_set_it_down_pick_it_up_and_stow_once_on_keys_and_touch() {
    for mut pilot in [Pilot::new(), Pilot::on_a_phone()] {
        pilot.walk_deck(&[[6.5, -7.5], [10.0, -7.5]]);
        let wreck = pilot.run.entity("wreck-1").unwrap();
        let contents: Vec<_> = crates(&pilot)
            .into_iter()
            .filter(|entity| pilot.run.world.get(*entity).unwrap().parent == Some(wreck))
            .collect();
        assert_eq!(contents.len(), 6, "actual crates attached to the wreck");
        let wood = contents
            .into_iter()
            .find(|entity| (field(&pilot, *entity, "slot") - 3.0).abs() < 0.01)
            .unwrap();
        let at = pilot.run.position(wood);
        stand_at(&mut pilot, at);
        assert!(
            (pilot.board("focus_kind") - 1.0).abs() < 0.01,
            "wood selected"
        );
        pilot.use_it();
        pilot.wait(0.1);
        assert!((pilot.board("carried_kind") - 1.0).abs() < 0.01);
        assert!((field(&pilot, wreck, "cargo_mask") - 55.0).abs() < 0.01);
        assert!(
            (field(&pilot, wreck, "scrap") - 3.0).abs() < 0.01,
            "scrap untouched"
        );
        assert!(pilot.board("wood").abs() < 0.01, "not yet stowed");
        drop(&mut pilot);
        assert!(pilot.board("carried_kind").abs() < 0.01);
        let loose = crates(&pilot)
            .into_iter()
            .find(|entity| pilot.run.world.get(*entity).unwrap().parent.is_none())
            .unwrap();
        assert_eq!(
            pilot.run.scripts.field(loose, "kind"),
            Some(&ScriptValue::Variant("Resource.Wood".into()))
        );
        pilot.use_it();
        pilot.wait(0.1);
        assert!(
            pilot.run.world.get(loose).is_none(),
            "pickup removes the ground crate"
        );
        assert!(
            (field(&pilot, wreck, "cargo_mask") - 55.0).abs() < 0.01,
            "no second charge to wreck"
        );
        let foot = pilot.deck_to_world([10.5, -7.5]);
        stand_at(&mut pilot, foot);
        let ramp = pilot.deck_to_world([8.4, -7.5]);
        pilot.walk_ashore(ramp, 0.2);
        pilot.wait(0.2);
        assert!(pilot.flag("aboard"));
        assert!((pilot.board("wood") - 1.0).abs() < 0.01);
        assert!((pilot.board("crates") - 1.0).abs() < 0.01);
    }
}

#[test]
fn dropped_cargo_washes_away_once_and_wreck_crates_survive_the_tide() {
    let mut pilot = Pilot::new();
    pilot::salvage::pick_up_at_first_wreck(&mut pilot);
    drop(&mut pilot);
    let loose = crates(&pilot)
        .into_iter()
        .find(|entity| pilot.run.world.get(*entity).unwrap().parent.is_none())
        .unwrap();
    let wreck = pilot.run.entity("wreck-1").unwrap();
    let mask = field(&pilot, wreck, "cargo_mask");
    pilot.run.scripts.blackboard_mut().set("tide_time", 280.0);
    pilot.wait(0.2);
    assert!(pilot.run.world.get(loose).is_none());
    assert!((pilot.board("washed") - 1.0).abs() < 0.01);
    pilot.wait(0.5);
    assert!((pilot.board("washed") - 1.0).abs() < 0.01);
    assert!((field(&pilot, wreck, "cargo_mask") - mask).abs() < 0.01);
}
