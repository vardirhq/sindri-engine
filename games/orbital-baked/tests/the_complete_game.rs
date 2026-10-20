//! Real player flows through mission control, a sortie and persistent career.
use orbital_baked::Run;
use sindri_platform::{InputEvent, Key, MemorySaves};

const STEP: f32 = 1.0 / 60.0;
fn settle(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "{notes:#?}");
    }
}
fn title() -> Run {
    let mut run = Run::open().expect("the complete project opens");
    settle(&mut run, 6);
    run
}
fn press(run: &mut Run, key: Key) {
    run.hold(key);
    settle(run, 1);
    run.let_go(key);
    settle(run, 2);
}
fn text(run: &Run, name: &str) -> String {
    run.text(run.find(name).expect("named label"))
        .expect("text")
}

#[test]
fn a_ship_is_unlocked_equipped_flown_and_remembered() {
    let mut run = title();
    run.click("TitleHangar");
    run.click("PortShip1");
    run.click("PortEquip");
    assert!(text(&run, "PortShipStatus").contains("NEED 80"));
    assert!(!run.session.saves().flag("ship_1", false));
    let mut funded = run.session.saves().to_document();
    funded
        .values
        .insert("credits".into(), sindri_core::SaveValue::Number(100.0));
    run.session
        .keep_saves_in(Box::new(MemorySaves::holding(funded)));
    run.click("PortEquip");
    assert!(run.session.saves().flag("ship_1", false));
    assert!((run.session.saves().number("credits", 0.0) - 20.0).abs() < 1.0e-5);
    run.click("PortEquip");
    assert!(
        (run.session.saves().number("credits", 0.0) - 20.0).abs() < 1.0e-5,
        "equipping twice does not charge twice"
    );
    run.click("PortBack");
    run.click("TitleStart");
    settle(&mut run, 5);
    assert!((run.board("max_hp") - 7.0).abs() < 1.0e-5);
    assert!((run.board("move_speed") - 4.42).abs() < 1.0e-4);
    let saves = run.session.saves().to_document();
    let mut reopened = Run::open().unwrap();
    reopened
        .session
        .keep_saves_in(Box::new(MemorySaves::holding(saves)));
    settle(&mut reopened, 6);
    assert!(text(&reopened, "TitleFleetText").contains("BASTION"));
}

#[test]
fn a_finished_sortie_settles_rewards_records_and_returns_to_the_title() {
    let mut run = title();
    run.click("TitleStart");
    settle(&mut run, 5);
    // Earned state at a deterministic late-flight checkpoint, then real death.
    run.set_board("elapsed", 120.0);
    run.set_board("kills", 100.0);
    run.set_board("bosses_defeated", 1.0);
    run.set_board("hp", 0.0);
    settle(&mut run, 5);
    assert!((run.board("run_state") - 4.0).abs() < 1.0e-5);
    assert!(run.session.saves().number("credits", 0.0) > 60.0);
    assert!(run.session.saves().flag("medal_hunter", false));
    assert!(run.session.saves().text("record_0", "").contains("STRIDER"));
    let earned = run.session.saves().number("credits", 0.0);
    settle(&mut run, 30);
    assert!((run.session.saves().number("credits", 0.0) - earned).abs() < 1.0e-5);
    run.click("ResultRush");
    settle(&mut run, 4);
    assert!(run.world.is_active(run.find("TitleStart").unwrap()));
    run.click("TitleRecords");
    assert!(text(&run, "PortRecord0").contains("100 KILLS"));
    run.click("PortBack");
    run.click("TitleStart");
    settle(&mut run, 4);
    assert!(run.board("kills").abs() < 1.0e-5);
    assert!(run.board("bosses_defeated").abs() < 1.0e-5);
}

#[test]
fn dash_has_a_cooldown_and_pause_preserves_it() {
    let mut run = title();
    run.click("TitleStart");
    settle(&mut run, 5);
    press(&mut run, Key::ShiftLeft);
    assert!(run.board("dash_left") > 3.0);
    let used = run.board("dash_left");
    press(&mut run, Key::ShiftLeft);
    assert!(
        run.board("dash_left") < used,
        "a second press cannot reset cooldown"
    );
    run.click("FlightPause");
    let paused = run.board("dash_left");
    settle(&mut run, 60);
    assert!((run.board("dash_left") - paused).abs() < 1.0e-5);
    run.click("PauseResume");
    settle(&mut run, 240);
    assert!(run.board("dash_left").abs() < 1.0e-5);
}

#[test]
fn the_callsign_accessibility_and_mixer_persist_through_a_new_session() {
    let mut run = title();
    run.click("TitleCallsign");
    for c in "Nova 星".chars() {
        run.input.apply(InputEvent::TextInput(c));
    }
    settle(&mut run, 3);
    press(&mut run, Key::Escape);
    run.click("TitleSettings");
    run.click("PortMotion");
    run.click("PortFlashes");
    let saves = run.session.saves().to_document();
    let mut reopened = Run::open().unwrap();
    reopened
        .session
        .keep_saves_in(Box::new(MemorySaves::holding(saves)));
    settle(&mut reopened, 6);
    assert_eq!(text(&reopened, "TitleCallsignText"), "Nova 星");
    assert!(reopened.session.saves().flag("reduced_motion", false));
    assert!(reopened.session.saves().flag("reduced_flashes", false));
}

#[test]
fn the_archive_browses_real_catalog_content_and_rerolls_are_bounded() {
    let mut run = title();
    run.click("TitleArchive");
    let first = text(&run, "PortArchiveName");
    run.click("PortArchiveNext");
    assert_ne!(text(&run, "PortArchiveName"), first);
    run.click("PortArchiveMode1");
    assert_eq!(text(&run, "PortArchiveName"), "SCOUT");
    run.click("PortArchiveMode3");
    assert_eq!(text(&run, "PortArchiveName"), "First Light");
    run.click("PortBack");
    run.click("TitleStart");
    settle(&mut run, 4);
    run.set_board("cores", 35.0);
    settle(&mut run, 3);
    run.click("UpgradeReroll");
    assert!((run.board("rerolls_left") - 1.0).abs() < 1.0e-5);
    run.click("UpgradeReroll");
    run.click("UpgradeReroll");
    assert!(run.board("rerolls_left").abs() < 1.0e-5);
    assert!(run.board("modules_installed").abs() < 1.0e-5);
}

#[test]
fn a_burst_key_is_rebound_persisted_and_used_by_the_next_sortie() {
    let mut run = title();
    run.click("TitleSettings");
    run.click("PortRebind");
    press(&mut run, Key::Q);
    assert_eq!(run.session.saves().text("burst_binding", ""), "key.Q");
    let saves = run.session.saves().to_document();
    let mut reopened = Run::open().unwrap();
    reopened
        .session
        .keep_saves_in(Box::new(MemorySaves::holding(saves)));
    settle(&mut reopened, 6);
    reopened.click("TitleStart");
    settle(&mut reopened, 4);
    press(&mut reopened, Key::ShiftLeft);
    assert!(reopened.board("dash_left").abs() < 1.0e-5);
    press(&mut reopened, Key::Q);
    assert!(reopened.board("dash_left") > 3.0);
}

#[test]
fn a_controller_steers_and_bursts_without_a_keyboard() {
    use sindri_platform::{GamepadAxis, GamepadButton, PadId};
    let mut run = title();
    run.click("TitleStart");
    settle(&mut run, 4);
    let pad = PadId(7);
    run.input.apply(InputEvent::GamepadConnected(pad));
    run.input.apply(InputEvent::GamepadAxisMoved {
        pad,
        axis: GamepadAxis::LeftX,
        value: 0.8,
    });
    settle(&mut run, 30);
    let player = run.find("Player").unwrap();
    assert!(
        run.world
            .get(player)
            .unwrap()
            .transform_3d
            .unwrap()
            .position[0]
            > 1.0
    );
    run.input.apply(InputEvent::GamepadPressed {
        pad,
        button: GamepadButton::RightBumper,
    });
    settle(&mut run, 2);
    assert!(run.board("dash_left") > 3.0);
}

#[test]
fn a_campaign_reaches_its_goal_and_replays_with_clean_state() {
    let mut run = Run::open().unwrap();
    // Shorten the authored mission for a deterministic goal test; production is
    // the director's 600-second default. No win state is written by the test.
    let director = run.find("Director").unwrap();
    run.world
        .get_mut(director)
        .unwrap()
        .components
        .get_mut("sindri.script")
        .unwrap()["properties"]["run_limit"] = serde_json::json!(2.0);
    settle(&mut run, 6);
    for sortie in 1..=2 {
        run.click("TitleStart");
        settle(&mut run, 140);
        assert!((run.board("run_state") - 4.0).abs() < 1.0e-5);
        assert!((run.board("victory") - 1.0).abs() < 1.0e-5);
        assert!((run.session.saves().number("victories", 0.0) - f64::from(sortie)).abs() < 1.0e-5);
        assert!(run.session.saves().text("record_0", "").contains("CLEAR"));
        run.click("ResultRush");
        settle(&mut run, 4);
    }
}

#[test]
fn every_hull_flies_its_advertised_sidegrade() {
    for (ship, hull, speed, fire, damage, research, arc) in [
        (0_u16, 5.0, 5.2, 2.38, 1.0, 1.0, 0.0),
        (1, 7.0, 4.42, 2.142, 1.0, 1.0, 0.0),
        (2, 4.0, 6.24, 2.856, 0.85, 1.0, 0.0),
        (3, 4.0, 5.2, 2.38, 1.1, 1.2, 1.0),
    ] {
        let mut run = title();
        let mut saves = run.session.saves().to_document();
        saves
            .values
            .insert("credits".into(), sindri_core::SaveValue::Number(500.0));
        run.session
            .keep_saves_in(Box::new(MemorySaves::holding(saves)));
        run.click("TitleHangar");
        run.click(&format!("PortShip{ship}"));
        run.click("PortEquip");
        run.click("PortBack");
        run.click("TitleStart");
        settle(&mut run, 5);
        for (key, expected) in [
            ("max_hp", hull),
            ("move_speed", speed),
            ("fire_rate", fire),
            ("damage", damage),
            ("xp_gain", research),
            ("arc", arc),
        ] {
            assert!(
                (run.board(key) - expected).abs() < 1.0e-4,
                "ship {ship}: {key}"
            );
        }
    }
}
