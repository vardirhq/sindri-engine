use super::*;

#[test]
fn cached_candidates_do_not_change_when_draft_settings_change() {
    let mut app = ComposerApp::default();
    let original = app.scores.clone();
    app.settings.key = 9;
    app.settings.mood = crate::lab::Mood::Hopeful;
    app.settings.seed = 177;
    assert_eq!(app.scores, original);
    assert_eq!(app.selected_settings().key, 2);
    app.generate();
    assert_ne!(app.scores, original);
    assert_eq!(app.selected_settings().key, 9);
}

#[test]
fn generate_keeps_locked_tracks_for_each_candidate() {
    let mut app = ComposerApp::default();
    let bass_a = app.scores[0][0].clone();
    let arp_b = app.scores[1][1].clone();
    app.track_locks[0][0] = true;
    app.track_locks[1][1] = true;
    app.settings.seed = app.settings.seed.wrapping_add(81);
    app.settings.key = 9;
    app.settings.mood = crate::lab::Mood::Hopeful;
    app.generate();
    assert_eq!(app.scores[0][0], bass_a);
    assert_eq!(app.scores[1][1], arp_b);
    assert_eq!(app.generated.key, 2);
    assert_eq!(app.generated.mood, crate::lab::Mood::Mysterious);
    let scale = crate::lab::synth::scale(app.generated.mood);
    assert!(
        app.scores
            .iter()
            .flatten()
            .flatten()
            .all(|note| { scale.contains(&(note.pitch - app.generated.key).rem_euclid(12)) })
    );
    assert!(app.status.contains("locked tracks were kept"));
}

#[test]
fn regenerating_one_track_keeps_the_rest_and_can_be_undone() {
    let mut app = ComposerApp {
        selected: 1,
        edit_track: 2,
        ..ComposerApp::default()
    };
    let before = app.scores[1].clone();
    app.regenerate_track();
    assert_ne!(app.scores[1][2], before[2]);
    assert_eq!(app.scores[1][0], before[0]);
    assert_eq!(app.scores[1][1], before[1]);
    app.undo_edit();
    assert_eq!(app.scores[1], before);
}

#[test]
fn a_locked_track_cannot_be_regenerated() {
    let mut app = ComposerApp::default();
    app.track_locks[0][2] = true;
    let before = app.scores[0].clone();
    app.regenerate_track();
    assert_eq!(app.scores[0], before);
    assert!(app.status.contains("Unlock this track"));
}

#[test]
fn regenerate_shortcut_replaces_the_selected_music_track() {
    let context = egui::Context::default();
    theme::install(&context);
    let mut app = ComposerApp {
        edit_track: 2,
        ..ComposerApp::default()
    };
    let before = app.scores[0][2].clone();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1360.0, 820.0),
        )),
        events: vec![egui::Event::Key {
            key: egui::Key::R,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        }],
        ..Default::default()
    };
    context
        .run_ui(input, |ui| app.workspace(ui))
        .drop_without_applying_deltas();
    assert_ne!(app.scores[0][2], before);
}

#[test]
fn vary_shortcut_changes_the_selected_lead() {
    let context = egui::Context::default();
    theme::install(&context);
    let mut app = ComposerApp {
        edit_track: 2,
        ..ComposerApp::default()
    };
    let before = app.scores[0][2].clone();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1360.0, 820.0),
        )),
        events: vec![egui::Event::Key {
            key: egui::Key::V,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        }],
        ..Default::default()
    };
    context
        .run_ui(input, |ui| app.workspace(ui))
        .drop_without_applying_deltas();
    assert_ne!(app.scores[0][2], before);
}

#[test]
fn varying_the_lead_is_undoable_and_leaves_other_tracks_alone() {
    let mut app = ComposerApp::default();
    let before = app.scores[0].clone();
    app.vary_melody();
    assert_ne!(app.scores[0][2], before[2]);
    assert_eq!(app.scores[0][0], before[0]);
    assert_eq!(app.scores[0][1], before[1]);
    app.undo_edit();
    assert_eq!(app.scores[0], before);
}

#[test]
fn a_locked_lead_cannot_be_varied() {
    let mut app = ComposerApp::default();
    app.track_locks[0][2] = true;
    let before = app.scores[0].clone();
    app.vary_melody();
    assert_eq!(app.scores[0], before);
    assert!(app.status.contains("Unlock Lead"));
}

#[test]
fn undo_and_redo_restore_the_edited_candidate() {
    let mut app = ComposerApp {
        selected: 1,
        ..ComposerApp::default()
    };
    let before = app.scores[1].clone();
    app.save_undo();
    app.scores[1][2][0].pitch += 1;
    let edited = app.scores[1].clone();
    app.finish_edit();
    app.selected = 0;
    app.undo_edit();
    assert_eq!(app.selected, 1);
    assert_eq!(app.scores[1], before);
    app.redo_edit();
    assert_eq!(app.scores[1], edited);
}

#[test]
fn overlapping_edits_roll_back_instead_of_hiding_notes() {
    let mut app = ComposerApp::default();
    let before = app.scores[0].clone();
    app.save_undo();
    app.scores[0][2][0].len = crate::lab::STEPS;
    app.finish_edit();
    assert_eq!(app.scores[0], before);
    assert!(app.status.contains("32-bar"));
}

#[test]
fn workstation_draws_at_wide_and_compact_window_sizes() {
    for size in [
        egui::vec2(1360.0, 820.0),
        egui::vec2(900.0, 650.0),
        egui::vec2(760.0, 560.0),
        // A phone in portrait, and one on its side.
        egui::vec2(390.0, 844.0),
        egui::vec2(700.0, 360.0),
    ] {
        for mode in [Mode::Music, Mode::Sfx] {
            let tabs = if mode == Mode::Music {
                vec![InspectorTab::Song, InspectorTab::Track, InspectorTab::Mixer]
            } else {
                vec![InspectorTab::Track]
            };
            for inspector_tab in tabs {
                let context = egui::Context::default();
                theme::install(&context);
                let mut app = ComposerApp {
                    mode,
                    inspector_tab,
                    ..ComposerApp::default()
                };
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    ..Default::default()
                };
                let mut output = context.run_ui(input, |ui| {
                    app.workspace(ui);
                });
                assert!(
                    !output.shapes.is_empty(),
                    "{mode:?} with {inspector_tab:?} at {size:?}"
                );
                output.textures_delta.clear();
            }
        }
    }
}

#[test]
fn clicking_a_section_header_selects_that_section() {
    let context = egui::Context::default();
    theme::install(&context);
    let mut app = ComposerApp::default();
    let position = egui::pos2(312.0, 17.0);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 100.0),
        )),
        events: vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            },
        ],
        ..Default::default()
    };
    context
        .run_ui(input, |ui| app.section_headers(ui))
        .drop_without_applying_deltas();
    assert_eq!(app.section, 2);
}

#[test]
fn clicking_an_arrangement_track_selects_its_piano_roll() {
    let context = egui::Context::default();
    theme::install(&context);
    let mut app = ComposerApp::default();
    let position = egui::pos2(24.0, 12.0);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 100.0),
        )),
        events: vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            },
        ],
        ..Default::default()
    };
    context
        .run_ui(input, |ui| app.track_headers(ui))
        .drop_without_applying_deltas();
    assert_eq!(app.edit_track, 0);
}

#[test]
fn undo_and_redo_restore_imported_tempo_with_notes() {
    let mut app = ComposerApp::default();
    let before = app.settings.bpm;
    app.save_undo();
    app.settings.bpm = 160.0;
    app.undo_edit();
    assert!((app.settings.bpm - before).abs() < f64::EPSILON);
    app.redo_edit();
    assert!((app.settings.bpm - 160.0).abs() < f64::EPSILON);
}

#[test]
fn send_delivers_the_current_effect_and_song_into_the_project() {
    let dir = tempfile::tempdir().expect("temp");
    std::fs::write(
        dir.path().join("sindri.toml"),
        "[project]\nname = \"Demo\"\n",
    )
    .expect("manifest");
    std::fs::create_dir(dir.path().join("assets")).expect("assets");
    let mut app = ComposerApp {
        mode: Mode::Sfx,
        project: Some(dir.path().to_path_buf()),
        ..ComposerApp::default()
    };
    app.sfx.generate(crate::lab::sfx::Kind::Laser);
    app.send_to_project();
    assert_eq!(
        app.sfx.snippet.as_deref(),
        Some("Audio.play(\"audio/laser.wav\", 0.5);")
    );
    let source =
        std::fs::read_to_string(dir.path().join("assets/audio/laser.sindrisfx")).expect("source");
    assert_eq!(
        crate::lab::sfx::SfxFile::from_json(&source).expect("valid"),
        app.sfx.file
    );

    app.mode = Mode::Music;
    app.song_name = "Title Theme".into();
    app.send_to_project();
    assert_eq!(
        app.song_snippet.as_deref(),
        Some("Audio.loop(\"audio/title-theme.ogg\", 0.6);")
    );
    // A song goes in compressed; an effect stays WAV.
    let song = std::fs::read(dir.path().join("assets/audio/title-theme.ogg")).expect("the song");
    assert_eq!(&song[..4], b"OggS");
    let manifest = std::fs::read_to_string(dir.path().join("sindri.toml")).expect("manifest");
    assert!(manifest.contains("audio/laser.wav") && manifest.contains("audio/title-theme.ogg"));
}

#[test]
fn effect_history_keeps_recent_sounds_newest_first() {
    let mut app = ComposerApp::default();
    app.sfx.generate(crate::lab::sfx::Kind::Jump);
    app.sfx.mutate();
    assert_eq!(app.sfx.history[0], app.sfx.file);
    assert_eq!(app.sfx.history[1].kind, crate::lab::sfx::Kind::Jump);
    assert!(app.sfx.history.len() >= 3);
    for _ in 0..30 {
        app.sfx.generate(crate::lab::sfx::Kind::Blip);
    }
    assert!(app.sfx.history.len() <= 14);
}
