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
    let position = egui::pos2(240.0, 17.0);
    let input = |events| egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 100.0),
        )),
        events,
        ..Default::default()
    };
    let button = |pressed| egui::Event::PointerButton {
        pos: position,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    context
        .run_ui(input(Vec::new()), |ui| app.section_headers(ui))
        .drop_without_applying_deltas();
    context
        .run_ui(
            input(vec![egui::Event::PointerMoved(position), button(true)]),
            |ui| app.section_headers(ui),
        )
        .drop_without_applying_deltas();
    context
        .run_ui(input(vec![button(false)]), |ui| app.section_headers(ui))
        .drop_without_applying_deltas();
    assert_eq!(app.section, 2);
}

#[test]
fn clicking_an_arrangement_track_selects_its_piano_roll() {
    let context = egui::Context::default();
    theme::install(&context);
    let mut app = ComposerApp::default();
    for layer in 0..5 {
        let position = egui::pos2(24.0, 39.0 + 33.0 * f32::from(u8::try_from(layer).unwrap()));
        let mut draw = |events| {
            context
                .run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(400.0, 260.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| app.track_headers(ui),
                )
                .drop_without_applying_deltas();
        };
        let button = |pressed| egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        draw(Vec::new());
        draw(vec![egui::Event::PointerMoved(position), button(true)]);
        draw(vec![button(false)]);
        assert_eq!(app.selected_layer, layer);
        assert_eq!(app.edit_track, layer.min(2));
        assert_eq!(app.inspector_tab, InspectorTab::Track);
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

#[test]
fn generated_layers_are_visible_and_cannot_regenerate_the_previous_melody() {
    let mut app = ComposerApp::default();
    let original = app.scores.clone();
    for layer in [3, 4] {
        app.select_layer(layer);
        assert!(!app.layer_notes(layer).is_empty());
        app.regenerate_track();
        app.vary_melody();
        assert_eq!(app.scores, original);
        assert!(app.selected_note.is_none());
    }
}

#[test]
fn chord_preview_tracks_bass_edits_without_changing_the_song_file() {
    let mut app = ComposerApp::default();
    let original = app.song_file().to_json();
    let before = app.layer_notes(4).to_vec();
    app.refresh_arrangement();
    assert_eq!(app.song_file().to_json(), original);
    for note in &mut app.scores[0][0] {
        note.pitch += 2;
    }
    app.refresh_arrangement();
    assert_ne!(app.layer_notes(4), before);
}

#[test]
fn fitted_roll_contains_the_visible_notes_on_both_phone_pages() {
    let mut app = ComposerApp {
        compact: true,
        ..ComposerApp::default()
    };
    for layer in 0..5 {
        for page in 0..2 {
            app.roll_page = page;
            app.select_layer(layer);
            let begin = page * 64;
            for note in app
                .layer_notes(layer)
                .iter()
                .filter(|n| n.start < begin + 64 && n.start + n.len > begin)
            {
                assert!(note.pitch >= app.roll_base);
                assert!(note.pitch < app.roll_base + i32::try_from(app.roll_span).unwrap());
            }
        }
    }
}

#[test]
fn phone_selection_prefers_a_populated_page_and_song_generation_works_from_drums() {
    let mut app = ComposerApp {
        compact: true,
        ..ComposerApp::default()
    };
    app.scores[0][2] = vec![crate::lab::Note {
        start: 80,
        pitch: 64,
        len: 4,
        velocity: 90,
    }];
    app.select_layer(2);
    assert_eq!(app.roll_page, 1);
    app.select_layer(3);
    let before = app.scores.clone();
    app.settings.seed += 2;
    app.generate();
    assert_ne!(app.scores, before);
}

#[test]
fn generation_preserves_sounds_notes_and_participation_independently() {
    let mut app = ComposerApp::default();
    app.settings.rack.tracks[0].preset = crate::lab::instruments::Preset::Organ;
    app.settings.rack.tracks[0].volume = 0.23;
    app.settings.rack.arrangement.sections = [1, 3, 7, 31];
    let before = app.scores[0][0].clone();
    app.track_locks[0][0] = true;
    app.sound_locks[0][0] = true;
    app.arrangement_locks[0][1] = true;
    app.settings.seed = 900;
    app.generate();
    assert_eq!(app.scores[0][0], before);
    assert_eq!(
        app.settings.rack.tracks[0].preset,
        crate::lab::instruments::Preset::Organ
    );
    assert!((app.settings.rack.tracks[0].volume - 0.23).abs() < f32::EPSILON);
    for section in 0..4 {
        assert_eq!(
            app.settings.rack.arrangement.active(section, 1),
            [false, true, true, true][section]
        );
    }
    let expected = crate::lab::ensemble::generate(&Settings {
        seed: 900,
        ..Settings::default()
    });
    assert_eq!(
        app.settings.rack.tracks[2].preset,
        expected.tracks[2].preset
    );
}

#[test]
fn candidates_keep_their_own_mixes_and_manual_generation_mode() {
    let mut app = ComposerApp::default();
    app.settings.rack.tracks[0].volume = 0.12;
    app.choose_candidate(1);
    app.settings.rack.tracks[0].volume = 0.89;
    app.choose_candidate(0);
    assert!((app.settings.rack.tracks[0].volume - 0.12).abs() < f32::EPSILON);
    app.choose_candidate(1);
    assert!((app.settings.rack.tracks[0].volume - 0.89).abs() < f32::EPSILON);
    let rack = app.settings.rack.clone();
    app.ensemble_options.sounds = false;
    app.ensemble_options.arrangement = false;
    app.settings.seed = 900;
    app.generate();
    app.choose_candidate(1);
    assert_eq!(app.settings.rack, rack);
}

#[test]
fn phrase_seeds_save_reopen_and_randomization_only_changes_the_draft() {
    let mut app = ComposerApp::default();
    let notes = app.scores.clone();
    app.seed_input = "forest at dawn".into();
    app.edit_seed();
    assert!(app.seed_error.is_none());
    assert_eq!(app.scores, notes);
    app.generate();
    let song = app.song_file();
    assert_eq!(song.seed_text.as_deref(), Some("forest at dawn"));
    let reopened = crate::lab::song::SongFile::from_json(&song.to_json()).unwrap();
    app.load_song(reopened);
    assert_eq!(app.seed_input, "forest at dawn");
    let saved = app.song_file().to_json();
    let old_seed = app.settings.seed;
    app.randomize_seed();
    assert_ne!(app.settings.seed, old_seed);
    assert!(app.settings.seed_text.is_none());
    assert_eq!(app.song_file().to_json(), saved);
    assert_eq!(app.seed_input.parse::<u64>().unwrap(), app.settings.seed);
    app.choose_candidate(1);
    assert!(app.song_file().seed_text.is_none());
}

#[test]
fn invalid_seed_input_keeps_the_last_valid_seed() {
    let mut app = ComposerApp::default();
    let seed = app.settings.seed;
    for invalid in ["", "18446744073709551616"] {
        app.seed_input = invalid.into();
        app.edit_seed();
        assert!(app.seed_error.is_some());
        assert_eq!(app.settings.seed, seed);
    }
}
