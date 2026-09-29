//! What each file operation does to a directory, and what it refuses.

use std::path::{Path, PathBuf};

use super::{AssetOpError, create_folder, delete, duplicate, import, rename, split_name};

fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("level.scene"), "{}").unwrap();
    std::fs::create_dir(root.path().join("textures")).unwrap();
    std::fs::write(root.path().join("textures/orb.png"), b"png").unwrap();
    root
}

fn names(directory: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    found
}

#[test]
fn a_folder_is_made_where_it_was_asked_for() {
    let project = project();
    let made = create_folder(project.path(), project.path(), "audio").unwrap();
    assert!(made.is_dir());
    assert!(names(project.path()).contains(&"audio".to_owned()));
}

#[test]
fn a_name_something_already_has_is_refused() {
    let project = project();
    assert!(matches!(
        create_folder(project.path(), project.path(), "textures"),
        Err(AssetOpError::Exists(_))
    ));
    assert!(matches!(
        rename(project.path(), &project.path().join("textures"), "level.scene"),
        Err(AssetOpError::Exists(_))
    ));
}

#[test]
fn a_name_that_points_somewhere_else_is_not_a_name() {
    let project = project();
    for typed in ["../escape", "sub/dir", "", "   ", ".."] {
        assert!(
            matches!(
                create_folder(project.path(), project.path(), typed),
                Err(AssetOpError::EmptyName | AssetOpError::NotAName(_))
            ),
            "{typed:?} must not be treated as a file name"
        );
    }
}

#[test]
fn a_target_outside_the_project_is_refused() {
    let project = project();
    let elsewhere = tempfile::tempdir().unwrap();
    let stranger = elsewhere.path().join("other.png");
    std::fs::write(&stranger, b"png").unwrap();

    assert!(matches!(
        delete(project.path(), &stranger),
        Err(AssetOpError::OutsideProject)
    ));
    assert!(matches!(
        rename(project.path(), &stranger, "renamed.png"),
        Err(AssetOpError::OutsideProject)
    ));
    assert!(stranger.exists(), "and it is still there");
}

#[test]
fn renaming_keeps_the_file_in_its_own_folder() {
    let project = project();
    let renamed = rename(
        project.path(),
        &project.path().join("textures/orb.png"),
        "pip.png",
    )
    .unwrap();
    assert_eq!(renamed, project.path().join("textures/pip.png"));
    assert_eq!(names(&project.path().join("textures")), vec!["pip.png"]);
}

#[test]
fn a_copy_is_still_the_same_kind_of_file() {
    let project = project();
    let copy = duplicate(project.path(), &project.path().join("level.scene")).unwrap();
    assert_eq!(copy.file_name().unwrap(), "level copy.scene");

    let again = duplicate(project.path(), &project.path().join("level.scene")).unwrap();
    assert_eq!(again.file_name().unwrap(), "level copy 2.scene");
}

#[test]
fn a_legacy_scene_copy_keeps_its_legacy_suffix() {
    let project = project();
    let legacy = project.path().join("legacy.scene.json");
    std::fs::write(&legacy, "{}").unwrap();
    let copy = duplicate(project.path(), &legacy).unwrap();
    assert_eq!(copy.file_name().unwrap(), "legacy copy.scene.json");
}

#[test]
fn splitting_a_name_keeps_the_whole_suffix() {
    let cases = [
        ("level.scene", ("level", ".scene")),
        ("legacy.scene.json", ("legacy", ".scene.json")),
        ("tiles.sheet.json", ("tiles", ".sheet.json")),
        ("orb.png", ("orb", ".png")),
        ("README", ("README", "")),
        (".gitignore", (".gitignore", "")),
    ];
    for (name, (stem, suffix)) in cases {
        assert_eq!(
            split_name(&PathBuf::from(name)),
            (stem.to_owned(), suffix.to_owned()),
            "{name} split wrongly"
        );
    }
}

#[test]
fn duplicating_a_folder_takes_its_contents() {
    let project = project();
    let copy = duplicate(project.path(), &project.path().join("textures")).unwrap();
    assert_eq!(copy.file_name().unwrap(), "textures copy");
    assert_eq!(names(&copy), vec!["orb.png"]);
}

#[test]
fn deleting_a_folder_removes_what_is_under_it() {
    let project = project();
    delete(project.path(), &project.path().join("textures")).unwrap();
    assert_eq!(names(project.path()), vec!["level.scene"]);
}

#[test]
fn an_import_brings_in_what_it_can_and_names_what_it_could_not() {
    let project = project();
    let elsewhere = tempfile::tempdir().unwrap();
    let fresh = elsewhere.path().join("pip.png");
    let clashing = elsewhere.path().join("level.scene");
    std::fs::write(&fresh, b"png").unwrap();
    std::fs::write(&clashing, "{}").unwrap();

    let (arrived, refused) = import(
        project.path(),
        project.path(),
        &[fresh.clone(), clashing.clone()],
    );

    assert_eq!(arrived, vec![project.path().join("pip.png")]);
    assert_eq!(refused.len(), 1);
    assert!(matches!(refused[0], AssetOpError::Exists(_)));
    assert_eq!(
        std::fs::read_to_string(project.path().join("level.scene")).unwrap(),
        "{}",
        "and the file it would have replaced is untouched"
    );
}
