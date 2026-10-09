use super::*;

#[test]
fn cached_candidates_do_not_change_when_draft_settings_change() {
    let mut app = ComposerApp::default();
    let original = app.scores.clone();
    app.settings.key = 9;
    app.settings.mood = crate::desktop::Mood::Hopeful;
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
    app.scores[0][2][0].len = crate::desktop::STEPS;
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
    ] {
        let context = egui::Context::default();
        let mut app = ComposerApp::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            ..Default::default()
        };
        let mut output = context.run_ui(input, |ui| {
            app.workspace(ui);
        });
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
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
