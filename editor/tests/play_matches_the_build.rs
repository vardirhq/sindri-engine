//! Editor Play plays what a build plays.
//!
//! Each project is opened twice: the way the editor opens it — the scene
//! file, the scripts, prefabs and profiles its loader brings in, the
//! project's stylesheets, the scene adopted as the one playing — and the
//! way a build opens it, through `ProjectRun`. Both sessions are stepped
//! with the same scripted input, presented the same way, and must end in
//! the same world. A difference is the editor loading or starting a run
//! differently from a build, which is the bug a play-test in the editor
//! would otherwise hide.
//!
//! Tile sets are read from the project here: the editor binds them as it
//! loads textures, which needs a GPU, and they are data either way.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sindri_assets::{AssetLoadQueueConfig, AssetLoader, FileSystemAssetSource, ModelAssetDecoder};
use sindri_core::TileSetDocument;
use sindri_editor::native::{load_world, scene_extractor};
use sindri_editor::play_session::{self, OpenScene, PlaySources};
use sindri_editor::project::Project;
use sindri_editor::scene_file::SceneFile;
use sindri_editor::scripts::SceneScripts;
use sindri_editor::weave_styles::ProjectStyles;
use sindri_platform::{InputEvent, InputState, Key, MouseButton};
use sindri_runtime::project::files_under;
use sindri_runtime::{ProjectRun, STEP};
use sindri_scene::TileSetBindings;

const SIZE: [f32; 2] = [1280.0, 720.0];
const STEPS: usize = 600;

fn game(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the editor crate sits in the workspace")
        .join("games")
        .join(name)
}

/// The project as editor Play starts it.
fn in_the_editor(root: &Path) -> ProjectRun {
    let project = Project::open(root).expect("the project opens");
    let scene = project.main_scene().expect("the project names its scene");
    let file = SceneFile::open(&scene).expect("the scene opens");
    let extractor = scene_extractor();
    let components = extractor.components().clone();
    let mut world = load_world(&extractor, &file).expect("the scene loads");

    // Turned until everything the project holds has arrived and compiles,
    // as the editor's frames turn its loader.
    let mut scripts = SceneScripts::for_scene(Some(&scene));
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        scripts.request(&world, &components);
        scripts.poll();
        let compiled = scripts.compile(&world, &components).is_empty();
        if (compiled && !scripts.loading()) || Instant::now() > deadline {
            assert!(compiled, "the editor could not compile {}", root.display());
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    let mut tile_sets = TileSetBindings::new();
    for (id, bytes) in files_under(&root.join("assets"), ".tileset") {
        let document = TileSetDocument::from_json(&String::from_utf8(bytes).expect("utf-8"))
            .expect("the tile set parses");
        tile_sets.bind(&id, document).expect("the tile set binds");
    }
    let styles = ProjectStyles::load(&project).expect("the stylesheets compose");
    let models = collision_models(
        root,
        &scene,
        &components,
        &scripts,
        &tile_sets,
        styles.sheets(),
        &world,
    );
    let session = play_session::start(
        &PlaySources {
            components: &components,
            scripts: &scripts,
            tile_sets: &tile_sets,
            sheets: styles.sheets(),
            models: Some(&models),
        },
        &mut world,
        OpenScene {
            project: Some(root),
            file: Some(&scene),
        },
        weave::Viewport {
            width: SIZE[0],
            height: SIZE[1],
        },
    )
    .expect("Play starts");
    ProjectRun {
        session,
        world,
        input: InputState::default(),
        size: SIZE,
        scene: extractor,
    }
}

fn collision_models(
    root: &Path,
    scene: &Path,
    components: &sindri_core::ComponentSchemaRegistry,
    scripts: &SceneScripts,
    tile_sets: &TileSetBindings,
    sheets: &[weave::Stylesheet],
    world: &sindri_core::World,
) -> AssetLoader<ModelAssetDecoder> {
    let open = OpenScene {
        project: Some(root),
        file: Some(scene),
    };
    let wanted = play_session::collision_references(
        &PlaySources {
            components,
            scripts,
            tile_sets,
            sheets,
            models: None,
        },
        world,
        open,
    )
    .unwrap();
    let mut models = AssetLoader::new(
        FileSystemAssetSource::new(sindri_runtime::project::assets_root(root)),
        AssetLoadQueueConfig::unbounded(2),
        ModelAssetDecoder,
    )
    .unwrap();
    for id in wanted.into_keys() {
        models.request(id).unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let outcomes = models.poll();
        assert!(
            !outcomes
                .iter()
                .any(|outcome| matches!(outcome, sindri_assets::AssetLoadOutcome::Failed(_))),
            "{outcomes:?}"
        );
        if models.outstanding() == 0 {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    models
}

/// The same input for both: a click in the middle of the screen, which
/// starts most title screens, then keys held and let go on a schedule.
fn scripted(step: usize, run: &mut ProjectRun) {
    const KEYS: [Key; 8] = [
        Key::Enter,
        Key::ArrowRight,
        Key::Space,
        Key::ArrowUp,
        Key::ArrowLeft,
        Key::D,
        Key::W,
        Key::A,
    ];
    match step {
        30 => {
            run.input.apply(InputEvent::PointerMoved {
                x: SIZE[0] / 2.0,
                y: SIZE[1] / 2.0,
            });
            run.input
                .apply(InputEvent::ButtonPressed(MouseButton::Left));
        }
        32 => run
            .input
            .apply(InputEvent::ButtonReleased(MouseButton::Left)),
        _ => {}
    }
    if step >= 60 && step.is_multiple_of(20) {
        let key = KEYS[(step / 20) % KEYS.len()];
        let down = !(step / 20).is_multiple_of(3);
        run.key(key, down);
    }
}

/// What is comparable between the two worlds: every entity a scene placed,
/// by its stable ID, and every one spawned since, by name and place.
fn state(run: &ProjectRun) -> (BTreeMap<String, String>, Vec<String>) {
    let mut placed = BTreeMap::new();
    let mut spawned = Vec::new();
    for (entity, data) in run.world.entities() {
        let position = run
            .world
            .world_transform(entity)
            .map(|transform| transform.position);
        let described = format!(
            "active {} at {position:?} {}",
            run.world.is_active(entity),
            serde_json::to_string(&data.components).expect("components serialize"),
        );
        match &data.source_id {
            Some(id) => {
                placed.insert(id.as_str().to_owned(), described);
            }
            None if run.world.is_active(entity) && data.name.is_some() => {
                spawned.push(format!("{:?} {described}", data.name));
            }
            None => {}
        }
    }
    spawned.sort();
    (placed, spawned)
}

fn plays_the_same(name: &str) {
    let root = game(name);
    let mut editor = in_the_editor(&root);
    let mut build = ProjectRun::open(&root, SIZE).expect("the build opens");
    for step in 0..STEPS {
        scripted(step, &mut editor);
        scripted(step, &mut build);
        let ours = editor.step(STEP).expect("the editor steps");
        let theirs = build.step(STEP).expect("the build steps");
        assert_eq!(ours.notes(), theirs.notes(), "{name}, step {step}: notes");
    }
    let (editor_placed, editor_spawned) = state(&editor);
    let (build_placed, build_spawned) = state(&build);
    let differing: Vec<_> = build_placed
        .iter()
        .filter(|(id, theirs)| editor_placed.get(*id) != Some(theirs))
        .map(|(id, theirs)| {
            format!(
                "{id}\n  build:  {theirs}\n  editor: {:?}",
                editor_placed.get(id)
            )
        })
        .take(5)
        .collect();
    assert!(
        differing.is_empty() && editor_placed.len() == build_placed.len(),
        "{name}: {} placed in the editor, {} in the build; differing:\n{}",
        editor_placed.len(),
        build_placed.len(),
        differing.join("\n")
    );
    assert_eq!(editor_spawned, build_spawned, "{name}: what was spawned");
}

#[test]
fn the_platformer_plays_the_same_in_the_editor_and_the_build() {
    plays_the_same("platformer");
}

#[test]
fn scorchball_plays_the_same_in_the_editor_and_the_build() {
    plays_the_same("scorchball");
}

#[test]
fn low_tide_plays_the_same_in_the_editor_and_the_build() {
    plays_the_same("low-tide");
}

#[test]
fn orbital_plays_the_same_in_the_editor_and_the_build() {
    plays_the_same("orbital-baked");
}

#[test]
fn model_collision_lab_blocks_and_reaches_the_same_goal_in_editor_and_headless() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/model-collision");
    let mut editor = in_the_editor(&root);
    let mut build = ProjectRun::open(&root, SIZE).unwrap();
    let mut said = Vec::new();
    let initial = state(&editor);
    let checkpoint = editor.session.checkpoint();
    let snapshot = editor.world.clone();
    for (key, frames) in [(Key::D, 90), (Key::S, 45), (Key::D, 48), (Key::W, 45)] {
        editor.key(key, true);
        build.key(key, true);
        for _ in 0..frames {
            let ours = editor.step(STEP).unwrap();
            let theirs = build.step(STEP).unwrap();
            assert!(ours.notes().is_empty(), "{ours:?}");
            assert!(theirs.notes().is_empty(), "{theirs:?}");
            assert_eq!(state(&editor), state(&build));
            said.extend(ours.scripts.printed.into_iter().map(|line| line.message));
        }
        editor.key(key, false);
        build.key(key, false);
    }
    assert!(
        said.iter().any(|line| line == "Model wall blocked"),
        "{said:?}"
    );
    assert!(
        said.iter().any(|line| line == "Model goal reached"),
        "{said:?}"
    );
    assert!(
        said.iter().any(|line| line == "Model prop goal reached"),
        "{said:?}"
    );
    assert!(
        said.iter()
            .any(|line| line == "Model spawn controls queued"),
        "{said:?}"
    );
    editor.session.restore(&checkpoint);
    editor.world = snapshot;
    assert_eq!(state(&editor), initial, "Stop restores the authored world");
    editor.key(Key::D, true);
    for _ in 0..90 {
        assert!(editor.step(STEP).unwrap().notes().is_empty());
    }
    let walker = editor.entity("walker").unwrap();
    assert!(
        editor.world.world_transform(walker).unwrap().position[0] < -0.29,
        "replay retains wall collision"
    );
}
