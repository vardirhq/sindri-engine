use super::super::*;
use crate::lab::{instruments::Preset, song::SongFile};

#[test]
fn ensemble_audition_changes_only_unlocked_presets_and_is_undoable() {
    let mut app = ComposerApp::default();
    app.settings.rack.tracks[0].preset = Preset::Organ;
    app.sound_locks[0][0] = true;
    app.settings.rack.tracks[1].volume = 0.12;
    app.settings.rack.tracks[2].octave = 1;
    app.settings.rack.arrangement.sections = [1, 3, 7, 31];
    let before = app.song_file();
    let b = app.candidate_racks[1].clone();
    app.try_ensemble();
    let after = app.song_file();
    assert_eq!(after.tracks, before.tracks);
    assert_eq!(after.seed, before.seed);
    assert_eq!(after.rack.arrangement, before.rack.arrangement);
    for layer in 0..5 {
        let mut sound = *after.rack.channel(layer);
        sound.preset = before.rack.channel(layer).preset;
        assert_eq!(sound, *before.rack.channel(layer));
    }
    assert_eq!(after.rack.tracks[0].preset, Preset::Organ);
    assert_ne!(after.rack, before.rack);
    assert_eq!(app.candidate_racks[1], b);
    app.undo_edit();
    assert_eq!(app.song_file(), before);
    app.redo_edit();
    assert_eq!(app.song_file(), after);
}

#[test]
fn locks_block_phrase_and_sound_changes_and_phrase_undo_restores_notes() {
    let mut app = ComposerApp::default();
    let before = app.song_file();
    let b = app.scores[1].clone();
    app.track_locks[0][2] = true;
    app.regenerate_phrase();
    assert_eq!(app.song_file(), before);
    app.track_locks[0][2] = false;
    app.regenerate_phrase();
    assert_ne!(app.scores[0][2], before.tracks[2]);
    assert_eq!(app.scores[1], b);
    assert_eq!(app.scores[0][0], before.tracks[0]);
    app.undo_edit();
    assert_eq!(app.song_file(), before);
    app.sound_locks[0] = [true; 5];
    app.try_ensemble();
    assert_eq!(app.song_file(), before);
}

#[test]
fn named_snapshots_travel_with_the_song_and_restore_both_candidates_on_undo() {
    let mut app = ComposerApp::default();
    app.choose_candidate(1);
    app.tools.snapshot_name = "Quiet theme".into();
    let original = app.song_file();
    app.save_named_snapshot();
    app.settings.seed = 900;
    app.generate();
    app.settings.rack.pad.volume = 0.2;
    let before_restore = app.song_file();
    let b = app.scores[1].clone();
    app.restore_named_snapshot(0);
    assert_eq!(app.scores[0], original.tracks);
    assert_eq!(app.selected_settings().seed, original.seed);
    assert_eq!(app.settings.rack, original.rack);
    assert_eq!(app.tools.snapshots.len(), 1);
    app.undo_edit();
    assert_eq!(app.song_file(), before_restore);
    assert_eq!(app.scores[1], b);
    app.redo_edit();
    assert_eq!(app.scores[0], original.tracks);
    let reopened = SongFile::from_json(&app.song_file().to_json()).unwrap();
    app.load_song(reopened);
    assert_eq!(app.tools.snapshots[0].name, "Quiet theme");
    assert_eq!(app.tools.snapshots[0].song.tracks, original.tracks);
}

#[test]
fn malformed_or_nested_snapshots_are_rejected_and_old_songs_still_load() {
    let app = ComposerApp::default();
    let song = app.song_file();
    assert!(!song.to_json().contains("snapshots"));
    SongFile::from_json(&song.to_json()).unwrap();
    let take = crate::lab::song::NamedSnapshot {
        name: "Take".into(),
        song: song.clone(),
    };
    let mut broken = song.clone();
    broken.snapshots.push(take.clone());
    SongFile::from_json(&broken.to_json()).unwrap();
    broken.snapshots[0].song.snapshots.push(take.clone());
    assert!(SongFile::from_json(&broken.to_json()).is_err());
    broken.snapshots = vec![take.clone(); 17];
    assert!(SongFile::from_json(&broken.to_json()).is_err());
    broken.snapshots = vec![take];
    broken.snapshots[0].song.bpm = 999.0;
    assert!(SongFile::from_json(&broken.to_json()).is_err());
}

#[test]
fn targeted_undo_preserves_later_manual_mixing() {
    let mut app = ComposerApp::default();
    app.regenerate_phrase();
    app.settings.rack.pad.volume = 0.17;
    app.undo_edit();
    assert!((app.settings.rack.pad.volume - 0.17).abs() < f32::EPSILON);
    let preset = app.settings.rack.tracks[2].preset;
    app.try_ensemble();
    app.settings.rack.tracks[2].volume = 0.23;
    app.undo_edit();
    assert_eq!(app.settings.rack.tracks[2].preset, preset);
    assert!((app.settings.rack.tracks[2].volume - 0.23).abs() < f32::EPSILON);
}
