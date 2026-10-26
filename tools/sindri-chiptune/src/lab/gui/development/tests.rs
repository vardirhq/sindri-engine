use super::super::*;
use crate::lab::song::SongFile;

#[test]
fn motif_expression_and_dialogue_are_undoable_and_respect_note_locks() {
    for action in [
        ComposerApp::apply_motif,
        ComposerApp::apply_feel,
        ComposerApp::apply_dialogue,
    ] {
        let mut app = ComposerApp::default();
        let before = app.song_file();
        let b = app.scores[1].clone();
        app.track_locks[0] = [true; 3];
        action(&mut app);
        assert_eq!(app.song_file(), before);
        app.track_locks[0] = [false; 3];
        action(&mut app);
        let changed = app.song_file();
        assert_ne!(changed.tracks, before.tracks);
        assert_eq!(changed.rack, before.rack);
        assert_eq!(app.scores[1], b);
        app.undo_edit();
        assert_eq!(app.song_file(), before);
        app.redo_edit();
        assert_eq!(app.song_file(), changed);
    }
}

#[test]
fn family_is_atomic_named_persistent_and_does_not_replace_the_current_song() {
    let mut app = ComposerApp::default();
    let before = app.song_file();
    app.create_soundtrack_family();
    assert_eq!(app.scores[0], before.tracks);
    assert_eq!(app.settings.rack, before.rack);
    assert_eq!(app.tools.snapshots.len(), 3);
    assert_eq!(app.tools.snapshots[1].name, "Theme · Danger");
    app.create_soundtrack_family();
    assert_eq!(app.tools.snapshots.len(), 3);
    let reopened = SongFile::from_json(&app.song_file().to_json()).unwrap();
    app.load_song(reopened);
    app.restore_named_snapshot(2);
    assert_eq!(app.generated.mood, crate::lab::Mood::Hopeful);
    assert_eq!(app.tools.snapshots.len(), 3);
    let take = app.tools.snapshots[0].clone();
    app.tools.snapshots = vec![take; 14];
    app.development.family_name = "Another family".into();
    app.create_soundtrack_family();
    assert_eq!(app.tools.snapshots.len(), 14);
}
