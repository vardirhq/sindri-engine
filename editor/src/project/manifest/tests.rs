use std::path::Path;

use sindri_core::SceneDocument;

use crate::scene_file::SceneFile;

use super::{FORMAT_VERSION, MANIFEST_NAME, Project, ProjectError, is_project, root_for};

/// A project made the way the welcome window makes one.
fn created(root: &Path, name: &str) -> Project {
    Project::create(root, name, &SceneDocument::default())
        .expect("a fresh directory takes a project")
}

#[test]
fn a_directory_without_a_manifest_is_not_a_project() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(directory.path().join("level.scene"), "{}").expect("a scene file");
    assert!(
        !is_project(directory.path()),
        "a folder with a scene in it is a folder, not a project"
    );
    assert!(matches!(
        Project::open(directory.path()),
        Err(ProjectError::NotAProject { .. })
    ));
}

#[test]
fn creating_a_project_writes_a_manifest_a_scene_and_somewhere_to_put_assets() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path().join("my-game");
    let project = created(&root, "My Game");

    assert!(
        root.join(MANIFEST_NAME).is_file(),
        "the manifest is written"
    );
    assert!(
        root.join("main.scene").is_file(),
        "a project with no scene is a project the editor opens on nothing"
    );
    for expected in ["textures", "scripts", "fonts"] {
        assert!(
            root.join(expected).is_dir(),
            "{expected}/ is where assets resolve from"
        );
    }
    assert_eq!(project.name(), "My Game");
    assert_eq!(project.root(), root);
}

#[test]
fn a_created_project_opens_on_the_scene_it_made() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path().join("my-game");
    let project = created(&root, "My Game");
    assert_eq!(
        project.main_scene(),
        Some(root.join("main.scene")),
        "creating a project nominates its scene, or opening it would ask which"
    );
}

#[test]
fn what_is_written_is_what_comes_back() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path().join("my-game");
    let written = created(&root, "My Game");
    let read = Project::open(&root).expect("a created project opens");
    assert_eq!(read, written);
}

#[test]
fn explicit_asset_includes_are_read_without_inventing_them_for_new_projects() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path();
    std::fs::write(
        root.join(MANIFEST_NAME),
        "format_version = 1\n\n[project]\nname = \"Styled\"\n\n[assets]\ninclude = [\"ui.weave\", \"music/theme.ogg\"]\n",
    )
    .expect("a manifest");
    let project = Project::open(root).expect("the manifest opens");
    assert_eq!(
        project.included_assets(),
        ["ui.weave", "music/theme.ogg"],
        "the editor must agree with export about which files are explicit roots"
    );

    let created_root = directory.path().join("fresh");
    created(&created_root, "Fresh");
    let text = std::fs::read_to_string(created_root.join(MANIFEST_NAME)).expect("the manifest");
    assert!(
        !text.contains("[assets]"),
        "an empty optional table should not churn manifests that do not need it: {text}"
    );
}

#[test]
fn a_project_is_named_by_its_manifest_rather_than_by_its_folder() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path().join("assets");
    let project = created(&root, "Gather");
    assert_eq!(
        Project::open(&root).expect("it opens").name(),
        "Gather",
        "the game is called Gather even though the folder is called `assets`"
    );
    assert_eq!(project.name(), "Gather");
}

#[test]
fn a_project_is_not_created_over_a_project() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path().join("my-game");
    created(&root, "My Game");
    assert!(
        matches!(
            Project::create(&root, "Something Else", &SceneDocument::default()),
            Err(ProjectError::AlreadyAProject { .. })
        ),
        "creating over a project would overwrite a name somebody chose"
    );
    assert_eq!(
        Project::open(&root).expect("it opens").name(),
        "My Game",
        "and the refusal leaves the first project as it was"
    );
}

#[test]
fn a_project_needs_a_name() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path().join("my-game");
    assert!(matches!(
        Project::create(&root, "   ", &SceneDocument::default()),
        Err(ProjectError::Unnamed)
    ));
    assert!(
        !root.exists(),
        "a refused creation leaves nothing behind to open"
    );
}

#[test]
fn a_project_from_a_newer_editor_is_refused_rather_than_guessed_at() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path();
    std::fs::write(
        root.join(MANIFEST_NAME),
        format!(
            "format_version = {}\n\n[project]\nname = \"From The Future\"\n",
            FORMAT_VERSION + 1
        ),
    )
    .expect("a manifest");
    assert!(matches!(
        Project::open(root),
        Err(ProjectError::FromTheFuture { .. })
    ));
}

#[test]
fn a_manifest_that_is_not_a_manifest_says_so() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path();
    std::fs::write(root.join(MANIFEST_NAME), "this is not toml =").expect("a manifest");
    assert!(matches!(
        Project::open(root),
        Err(ProjectError::Malformed { .. })
    ));
}

#[test]
fn a_nominated_scene_that_is_gone_opens_nothing_rather_than_something_else() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path().join("my-game");
    let project = created(&root, "My Game");
    std::fs::remove_file(root.join("main.scene")).expect("the scene is removed");
    std::fs::write(root.join("other.scene"), "{}").expect("another scene");
    assert_eq!(
        project.main_scene(),
        None,
        "standing another scene in for the named one would read as though it loaded"
    );
}

#[test]
fn nominating_a_scene_stores_it_relative_to_the_project() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path().join("my-game");
    let mut project = created(&root, "My Game");
    let scene = root.join("levels").join("two.scene");
    std::fs::create_dir_all(scene.parent().expect("a parent")).expect("a levels folder");
    std::fs::write(&scene, "{}").expect("a scene");

    project
        .set_main_scene(&scene)
        .expect("it is inside the project");
    assert_eq!(project.main_scene(), Some(scene));
    let text = std::fs::read_to_string(root.join(MANIFEST_NAME)).expect("the manifest");
    assert!(
        text.contains("levels/two.scene"),
        "the path is stored project-relative and with forward slashes, so a \
         project checked out on another platform still finds it: {text}"
    );
}

#[test]
fn a_scene_outside_the_project_cannot_be_nominated() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path().join("my-game");
    let mut project = created(&root, "My Game");
    let outside = directory.path().join("elsewhere.scene");
    std::fs::write(&outside, "{}").expect("a scene");
    assert!(
        project.set_main_scene(&outside).is_err(),
        "the field is relative to the root, so a path escaping it names nothing"
    );
}

#[test]
fn a_scene_deep_inside_a_project_still_finds_it() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path().join("my-game");
    created(&root, "My Game");
    let scene = root.join("levels").join("act-one").join("two.scene");
    std::fs::create_dir_all(scene.parent().expect("a parent")).expect("the folders");
    std::fs::write(&scene, "{}").expect("a scene");

    assert_eq!(
        root_for(&scene).as_deref(),
        Some(root.as_path()),
        "opening a scene from the command line opens it as its project"
    );
}

#[test]
fn a_scene_in_no_project_belongs_to_no_project() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let scene = directory.path().join("loose.scene");
    std::fs::write(&scene, "{}").expect("a scene");
    assert_eq!(root_for(&scene), None);
}

/// A field the editor does not use must still survive the editor saving.
///
/// `ProjectSection` serializes by named field, so anything it did not model
/// would be dropped the first time the editor rewrote the manifest — and a
/// person would find the scenes their game reaches missing from the next build,
/// with nothing to point at. Asserted through the real save path rather than by
/// reading the struct, because it is the write that loses a field.
#[test]
fn saving_a_project_keeps_the_scenes_it_did_not_open() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path().join("two-places");
    let project = created(&root, "Two Places");

    // Written as a person or another tool would write it.
    let manifest = root.join(MANIFEST_NAME);
    let text = std::fs::read_to_string(&manifest).expect("the manifest reads");
    std::fs::write(
        &manifest,
        text.replace("[project]", "[project]\nscenes = [\"assets/house.scene\"]"),
    )
    .expect("the manifest writes");
    drop(project);

    let mut project = Project::open(&root).expect("the project reopens");
    assert_eq!(
        project.manifest.project.scenes,
        vec!["assets/house.scene".to_owned()],
        "the field is read"
    );

    // Anything that rewrites the manifest, here the ordinary act of nominating
    // a different opening scene.
    let other = root.join("other.scene");
    SceneFile::create(&other, &SceneDocument::default()).expect("a second scene file");
    project
        .set_main_scene(&other)
        .expect("the scene can be nominated");

    let after = std::fs::read_to_string(&manifest).expect("the manifest still reads");
    assert!(
        after.contains("assets/house.scene"),
        "saving dropped the scenes the editor does not use:\n{after}"
    );
}

/// And a project that names none does not grow an empty list, so manifests do
/// not change shape just by being opened.
#[test]
fn a_project_with_no_extra_scenes_writes_no_list() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path().join("one-place");
    let project = created(&root, "One Place");
    drop(project);
    let text = std::fs::read_to_string(root.join(MANIFEST_NAME)).expect("the manifest reads");
    assert!(!text.contains("scenes"), "{text}");
}

/// A manifest written by hand keeps what this build does not model — a web
/// splash, the comment saying why an asset is included — through the editor
/// changing a field it does.
#[test]
fn saving_keeps_tables_and_comments_it_does_not_model() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path();
    for scene in ["assets/title.scene", "assets/level.scene"] {
        std::fs::create_dir_all(root.join("assets")).expect("a folder");
        SceneFile::create(&root.join(scene), &SceneDocument::default()).expect("a scene");
    }
    let written = "format_version = 1\n\n[project]\nname = \"Doors\"\nmain_scene = \"assets/title.scene\"\n\n# Played by a script, so no scene names it.\n[assets]\ninclude = [\"audio/door.wav\"]\n\n[web.splash]\nimage = \"assets/logo.png\"\nseconds = 1.2\n";
    std::fs::write(root.join(MANIFEST_NAME), written).expect("the manifest writes");

    let mut project = Project::open(root).expect("the project opens");
    project
        .add_scene(&root.join("assets/level.scene"))
        .expect("the scene is added");
    let after = std::fs::read_to_string(root.join(MANIFEST_NAME)).expect("it reads");
    assert!(
        after.contains("[web.splash]") && after.contains("seconds = 1.2"),
        "{after}"
    );
    assert!(after.contains("# Played by a script"), "{after}");
    assert!(
        after.contains("scenes = [\"assets/level.scene\"]"),
        "{after}"
    );
    assert_eq!(
        Project::open(root).expect("it reopens").scenes(),
        vec![
            root.join("assets/title.scene"),
            root.join("assets/level.scene")
        ]
    );
}

/// Nominating a listed scene swaps it with the main one, so the scene the
/// project opened on before is still one it carries; the main scene cannot
/// be taken off the list, and a listed one can.
#[test]
fn the_scene_list_is_edited_without_losing_a_scene() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = directory.path().join("doors");
    let mut project = created(&root, "Doors");
    let main = root.join("main.scene");
    let hall = root.join("hall.scene");
    let yard = root.join("yard.scene");
    for scene in [&hall, &yard] {
        SceneFile::create(scene, &SceneDocument::default()).expect("a scene");
    }
    project.add_scene(&hall).expect("added");
    project.add_scene(&yard).expect("added");
    project.add_scene(&hall).expect("again changes nothing");
    assert_eq!(
        project.scenes(),
        vec![main.clone(), hall.clone(), yard.clone()]
    );

    project.set_main_scene(&yard).expect("nominated");
    assert_eq!(
        project.scenes(),
        vec![yard.clone(), main.clone(), hall.clone()]
    );
    assert!(matches!(
        project.remove_scene(&yard),
        Err(ProjectError::RemovingMainScene { .. })
    ));
    project.move_scene(&hall, 0).expect("moved");
    assert_eq!(
        project.scenes(),
        vec![yard.clone(), hall.clone(), main.clone()]
    );
    project.remove_scene(&main).expect("removed");

    let reopened = Project::open(&root).expect("it reopens");
    assert_eq!(reopened.scenes(), vec![yard, hall]);
    assert!(
        main.is_file(),
        "taking a scene off the list leaves its file"
    );
}
