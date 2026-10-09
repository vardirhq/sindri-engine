//! Combat Lab proves attack primitives outside any boss script.
//! A boss should compose these entities, not reimplement their behavior.

use orbital_baked::Run;
use sindri_core::SceneDocument;

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run) {
    let notes = run.step(STEP);
    assert!(notes.is_empty(), "{notes:#?}");
}

fn lab_run() -> Run {
    let mut run = Run::open().expect("the project opens");
    for _ in 0..6 {
        step(&mut run);
    }
    run.click("TitleStart");
    step(&mut run);

    // The lab owns the arena while a test is using it. Normal spawning would
    // make counts nondeterministic and, more importantly, hide whether an attack
    // works on its own.
    let director = run.find("Director").expect("the director exists");
    run.world
        .get_mut(director)
        .expect("director remains")
        .disabled = true;

    let document = run
        .session
        .prefabs()
        .get("prefabs/combat-lab.prefab")
        .expect("the combat lab prefab ships")
        .clone();
    run.world.spawn_prefab(&document).expect("lab spawns");
    step(&mut run);
    run.set_board("lab_autoplay", 0.0);
    run.set_board("lab_clear", 1.0);
    step(&mut run);
    run
}

#[test]
fn the_standalone_lab_scene_is_a_real_scene() {
    let text = std::fs::read_to_string(orbital_baked::project().join("assets/combat-lab.scene"))
        .expect("the lab scene reads");
    let scene: SceneDocument = serde_json::from_str(&text).expect("the lab scene parses");
    scene.validate().expect("the lab scene validates");
}

#[test]
fn every_attack_can_be_spawned_independently() {
    let mut run = lab_run();
    for (kind, tag) in [
        (1.0, "attack_gas"),
        (2.0, "attack_mine"),
        (3.0, "attack_shockwave"),
        (4.0, "attack_gravity"),
    ] {
        run.set_board("lab_spawn", kind);
        step(&mut run);
        assert_eq!(run.count(tag), 1, "attack kind {kind} did not spawn");
        run.set_board("lab_clear", 1.0);
        step(&mut run);
        assert_eq!(run.count("lab_attack"), 0, "clear left attack kind {kind}");
    }
}

#[test]
fn combo_presets_use_the_same_attack_entities() {
    let mut run = lab_run();

    run.set_board("lab_combo", 1.0);
    step(&mut run);
    assert_eq!(run.count("attack_gas"), 1);
    assert_eq!(run.count("attack_gravity"), 1);

    run.set_board("lab_combo", 2.0);
    step(&mut run);
    assert_eq!(run.count("attack_mine"), 3);
    assert_eq!(run.count("attack_shockwave"), 1);

    run.set_board("lab_combo", 3.0);
    step(&mut run);
    assert_eq!(run.count("attack_gas"), 1);
    assert_eq!(run.count("attack_gravity"), 1);
    assert_eq!(run.count("attack_mine"), 2);
    assert_eq!(run.count("attack_shockwave"), 1);
    assert_eq!(run.count("lab_attack"), 5);

    // Let the combination actually run. A preset that merely spawns but starts
    // throwing script/physics errors a frame later is not a usable playground.
    for _ in 0..120 {
        step(&mut run);
    }
}

#[test]
fn a_shockwave_triggers_the_minefield() {
    let mut run = lab_run();
    run.set_board("lab_combo", 2.0);
    step(&mut run);
    assert_eq!(run.count("attack_mine"), 3);

    for _ in 0..100 {
        step(&mut run);
    }

    assert_eq!(
        run.count("attack_mine"),
        0,
        "the wave crossed the mines without triggering the chain"
    );
}

#[test]
fn the_stress_preset_has_real_cross_attack_reactions() {
    let mut run = lab_run();
    run.set_board("lab_combo", 3.0);
    step(&mut run);
    assert_eq!(run.count("attack_gas"), 1);
    assert_eq!(run.count("attack_mine"), 2);
    assert_eq!(run.count("lab_target"), 4);

    for _ in 0..110 {
        step(&mut run);
    }

    assert_eq!(
        run.count("attack_mine"),
        0,
        "the wave did not start the chain"
    );
    assert_eq!(
        run.count("attack_gas"),
        0,
        "the mine blast did not ignite the gas"
    );
    assert!(
        run.count("lab_target") < 4,
        "composed attacks did not affect their reactive environment"
    );

    run.set_board("lab_clear", 1.0);
    step(&mut run);
    assert_eq!(
        run.count("lab_target"),
        4,
        "clearing the lab did not restore its destructible targets"
    );
}

/// A mine's blast reaches what its circle overlaps, collider and all: a target
/// whose centre is just outside the blast but whose body is inside it is hit.
#[test]
fn a_mine_blast_catches_a_target_by_its_edge() {
    let mut run = lab_run();
    let document = run
        .session
        .prefabs()
        .get("prefabs/attack-hostile-mine.prefab")
        .expect("the mine prefab ships")
        .clone();
    let spawned = run.world.spawn_prefab(&document).expect("mine spawns");
    let mine = spawned.root;
    let data = run.world.get_mut(mine).expect("the mine");
    // 1.95 from the target at (-1.35, -0.25): outside a 1.75 blast by its
    // centre, inside it by the target's 0.32 radius.
    data.transform_3d.as_mut().expect("placed").position = [0.6, -0.25, 0.0];
    data.components.get_mut("sindri.script").expect("scripted")["properties"]["lifetime"] =
        serde_json::json!(0.0);

    let target = run
        .world
        .entities()
        .find(|(_, data)| {
            data.transform_3d.is_some_and(|t| {
                (t.position[0] + 1.35).abs() < 1.0e-3 && (t.position[1] + 0.25).abs() < 1.0e-3
            })
        })
        .map(|(entity, _)| entity)
        .expect("the near-left target");
    step(&mut run);
    step(&mut run);
    step(&mut run);
    let hp = run
        .session
        .scripts()
        .field(target, "hp")
        .and_then(|value| match value {
            sindri_decay::ScriptValue::Number(hp) => Some(*hp),
            _ => None,
        })
        .expect("a target with health");
    assert!(hp < 2.4, "the blast's edge missed a target it overlapped");
}
