//! Every advertised build system has a payoff, and no payoff leaks into replay.
use orbital_baked::Run;
use sindri_core::{SaveStore, SaveValue};
use sindri_platform::MemorySaves;

const STEP: f32 = 1.0 / 60.0;
fn settle(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "{notes:#?}");
    }
}
fn playing() -> Run {
    let mut run = Run::open().unwrap();
    settle(&mut run, 6);
    run.click("TitleStart");
    settle(&mut run, 4);
    run
}
fn install(run: &mut Run, module: f32) {
    run.set_board("run_state", 2.0);
    settle(run, 3);
    // Select a known catalogue entry through the real first card's install path.
    run.set_board("offer_0", module);
    run.click("Guidance Kernel");
    settle(run, 4);
}

#[test]
fn legacy_room_and_shop_modules_now_give_real_combat_benefits() {
    let mut run = playing();
    install(&mut run, 140.0); // Rusted Key: XP and collection, not absent doors.
    assert!(run.board("xp_gain") > 1.19);
    assert!(run.board("magnet") > 2.5);
    install(&mut run, 142.0); // Warm Seat: hull and regeneration, not absent shops.
    assert!(run.board("max_hp") > 5.5);
    assert!(run.board("regen") > 0.02);
    assert!(run.session.saves().flag("module_140", false));
}

#[test]
fn every_recipe_has_an_explicit_bonus_and_the_complete_build_activates_all_nineteen() {
    let catalog: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            orbital_baked::project().join("assets/profiles/synergy-catalog.profile"),
        )
        .unwrap(),
    )
    .unwrap();
    let recipes = catalog["values"]["synergies"].as_array().unwrap();
    assert_eq!(recipes.len(), 19);
    for recipe in recipes {
        assert!(
            [
                "damage_bonus",
                "fire_bonus",
                "armor_bonus",
                "magnet_bonus",
                "regen_bonus"
            ]
            .iter()
            .any(|key| recipe[key].as_f64().unwrap_or(0.0) > 0.0)
        );
        assert!(!recipe["requirements_text"].as_str().unwrap().is_empty());
    }
    let mut run = playing();
    for key in [
        "missile",
        "arc",
        "nova",
        "beam",
        "mines",
        "comp_ember",
        "comp_wisp",
        "comp_shield",
        "comp_blade",
        "comp_drone",
        "transform_storm",
        "transform_critical_mass",
        "transform_rail_prism",
        "transform_recursive",
        "transform_horizon",
        "transform_thunder_choir",
        "transform_razor_prism",
        "transform_guardian_swarm",
        "transform_singularity_court",
    ] {
        run.set_board(key, 1.0);
    }
    run.set_board("shots_add", 1.0);
    run.set_board("pierce_add", 1.0);
    run.set_board("bullet_size_mul", 1.2);
    settle(&mut run, 5);
    assert!((run.board("synergy_count") - 19.0).abs() < 1.0e-5);
    assert!(run.board("damage") > 1.0 && run.board("fire_rate") > 2.38);
    assert!(run.board("armor") > 0.0 && run.board("regen") > 0.0);
    let damage = run.board("damage");
    settle(&mut run, 20);
    assert!(
        (run.board("damage") - damage).abs() < 1.0e-5,
        "bonuses must not compound each frame"
    );
    run.set_board("hp", 0.0);
    settle(&mut run, 4);
    run.click("ResultAgain");
    settle(&mut run, 5);
    assert!(run.board("synergy_count").abs() < 1.0e-5);
    assert!((run.board("damage") - 1.0).abs() < 1.0e-5);
    assert!(run.board("armor").abs() < 1.0e-5);
}

#[test]
fn difficulty_changes_real_incoming_damage_and_population_pressure() {
    for (difficulty, pressure, damage) in [(0.0, 0.78, 0.65), (1.0, 1.0, 1.0), (2.0, 1.22, 1.25)] {
        let mut saves = SaveStore::default();
        saves.set("difficulty", SaveValue::Number(difficulty));
        let mut run = Run::open().unwrap();
        run.session
            .keep_saves_in(Box::new(MemorySaves::holding(saves.to_document())));
        settle(&mut run, 6);
        run.click("TitleStart");
        settle(&mut run, 4);
        assert!((run.board("run_pressure") - pressure).abs() < 1.0e-5);
        let before = run.board("hp");
        run.set_board("player_damage_taken", 1.0);
        settle(&mut run, 1);
        assert!((before - run.board("hp") - damage).abs() < 1.0e-5);
    }
}

#[test]
fn the_black_sun_is_an_actual_authored_area_attack() {
    let mut run = playing();
    run.set_board("special_black_sun", 1.0);
    settle(&mut run, 42);
    let pulse = run.find("Black Sun Pulse").expect("an area pulse is alive");
    let data = run.world.get(pulse).unwrap();
    assert!(
        data.components["sindri.script"]["properties"]["damage"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    assert!(
        (data.components["sindri.physics2d.collider"]["shape"]["radius"]
            .as_f64()
            .unwrap()
            - 2.5)
            .abs()
            < 1.0e-5
    );
}
