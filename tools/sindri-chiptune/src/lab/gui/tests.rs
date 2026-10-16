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
            let context = egui::Context::default();
            theme::install(&context);
            let mut app = ComposerApp {
                mode,
                ..ComposerApp::default()
            };
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            };
            let mut output = context.run_ui(input, |ui| {
                app.workspace(ui);
            });
            assert!(!output.shapes.is_empty(), "{mode:?} at {size:?}");
            output.textures_delta.clear();
        }
    }
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
        Some("Audio.loop(\"audio/title-theme.wav\", 0.6);")
    );
    let manifest = std::fs::read_to_string(dir.path().join("sindri.toml")).expect("manifest");
    assert!(manifest.contains("audio/laser.wav") && manifest.contains("audio/title-theme.wav"));
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
