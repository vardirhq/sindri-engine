//! The device-independent half of Sindri Mobile's first technical slice.
//!
//! `mobile/first-slice.md` lists the slice; this is the part of it that does not
//! need a phone. A real project — a copy of Lamplighter — is opened and edited
//! through the same checked commands, history, schema check, save and run a
//! mobile host is meant to share with desktop, and the result is read back by
//! the desktop's open path and by the build's.
//!
//! Nothing here touches a renderer, touch input, an Android lifecycle or a
//! document provider. Passing it says the shared seams hold for this edit;
//! it says nothing about whether any of them build or behave on Android.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sindri_core::{
    CommandBuffer, CommandHistory, ComponentSchemaRegistry, EntityId, SceneComponent,
    SceneDocument, SceneEntityId, World, WorldCommand,
};
use sindri_editor::native::{load_world, scene_extractor};
use sindri_editor::scene_file::SceneFile;
use sindri_runtime::{ProjectRun, STEP};
use sindri_scene::Light2dComponent;

/// The entity the slice moves: Wick, who the run also moves.
const MOVED: &str = "wick";
/// The entity whose property the slice edits: one of the field's blue lamps.
const EDITED: &str = "cover-17";
/// How far the touch drag carries Wick, in two pointer moves.
const DRAG: [[f32; 2]; 2] = [[0.75, 0.0], [1.5, 0.5]];

/// A copy of Lamplighter to write to, so the checked-in game is never touched.
fn lamplighter_copy() -> (tempfile::TempDir, PathBuf) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the editor crate sits in the workspace")
        .join("games/lamplighter");
    let dir = tempfile::tempdir().expect("a temporary directory");
    let root = dir.path().join("lamplighter");
    copy_project(&source, &root);
    (dir, root)
}

/// The project's manifest and assets, which is all opening or playing it reads.
fn copy_project(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).expect("the copy's folder");
    std::fs::copy(source.join("sindri.toml"), target.join("sindri.toml")).expect("the manifest");
    copy_tree(&source.join("assets"), &target.join("assets"));
}

fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).expect("a folder in the copy");
    for entry in std::fs::read_dir(source).expect("the folder reads") {
        let entry = entry.expect("an entry");
        let path = entry.path();
        let into = target.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &into);
        } else {
            std::fs::copy(&path, &into).expect("a file copies");
        }
    }
}

/// What an editing session holds: the file, the world, its history and schemas.
struct Authoring {
    file: SceneFile,
    world: World,
    history: CommandHistory,
    components: ComponentSchemaRegistry,
}

impl Authoring {
    /// Opens a scene the way the desktop editor does.
    fn open(path: &Path) -> Self {
        let file = SceneFile::open(path).expect("the scene opens");
        let extractor = scene_extractor();
        let world = load_world(&extractor, &file).expect("the scene loads");
        Self {
            file,
            world,
            history: CommandHistory::default(),
            components: extractor.components().clone(),
        }
    }

    fn entity(&self, id: &str) -> EntityId {
        self.world
            .entity_for_source_id(&SceneEntityId::new(id.to_owned()).expect("a valid ID"))
            .unwrap_or_else(|| panic!("the scene has {id}"))
    }

    fn position(&self, id: &str) -> [f32; 3] {
        self.world
            .get(self.entity(id))
            .and_then(|data| data.transform_3d)
            .expect("the entity has a transform")
            .position
    }

    fn payload(&self, id: &str, type_name: &str) -> Value {
        self.world
            .get(self.entity(id))
            .and_then(|data| data.components.get(type_name))
            .cloned()
            .unwrap_or_else(|| panic!("{id} carries {type_name}"))
    }

    /// The world as it would be written, to compare one state with another.
    fn document(&self) -> SceneDocument {
        self.world
            .to_scene_with_references(self.file.prefabs(), &self.components)
            .expect("the world writes")
    }

    /// A touch drag: one transaction per pointer move, merged into one step,
    /// and the run ended when the finger lifts.
    fn drag(&mut self, id: &str, offsets: &[[f32; 2]]) {
        let entity = self.entity(id);
        let start = self
            .world
            .get(entity)
            .and_then(|data| data.transform_3d)
            .expect("the entity has a transform");
        for offset in offsets {
            let mut moved = start;
            moved.position[0] += offset[0];
            moved.position[1] += offset[1];
            let mut buffer = CommandBuffer::new();
            buffer.push(WorldCommand::SetTransform3D {
                entity,
                transform: Some(moved),
            });
            let transaction = buffer
                .into_transaction("Move")
                .merging(format!("drag:{id}"));
            self.history
                .apply(transaction, &mut self.world)
                .expect("the move applies");
        }
        self.history.break_merge_run();
    }

    /// An inspector edit of one field, refused before it reaches the world
    /// when the result would not decode as the component.
    fn set_field(&mut self, id: &str, type_name: &str, field: &str, value: Value) -> bool {
        let entity = self.entity(id);
        let mut payload = self.payload(id, type_name);
        payload[field] = value;
        if self
            .components
            .validate_payload(type_name, &payload)
            .is_err()
        {
            return false;
        }
        let mut buffer = CommandBuffer::new();
        buffer.push(WorldCommand::SetComponent {
            entity,
            type_name: type_name.to_owned(),
            payload,
        });
        self.history
            .apply(
                buffer.into_transaction(format!("Edit {field}")),
                &mut self.world,
            )
            .expect("the edit applies");
        true
    }

    fn undo(&mut self) {
        self.history
            .undo(&mut self.world)
            .expect("undo applies")
            .expect("there is a step to undo");
    }

    fn redo(&mut self) {
        self.history
            .redo(&mut self.world)
            .expect("redo applies")
            .expect("there is a step to redo");
    }
}

fn intensity(payload: &Value) -> f64 {
    payload["intensity"].as_f64().expect("a numeric intensity")
}

fn assert_near(actual: [f32; 3], expected: [f32; 3], what: &str) {
    let apart = (0..3).map(|axis| (actual[axis] - expected[axis]).abs());
    assert!(
        apart.fold(0.0, f32::max) < 1e-5,
        "{what}: {actual:?}, expected {expected:?}"
    );
}

#[test]
fn a_touch_edit_undoes_saves_reopens_and_plays_as_desktop_and_build_read_it() {
    let (_dir, root) = lamplighter_copy();
    let scene = root.join("assets/world.scene");
    let mut authoring = Authoring::open(&scene);
    let authored = authoring.document();
    let start = authoring.position(MOVED);
    let light = Light2dComponent::TYPE_NAME;
    let start_intensity = intensity(&authoring.payload(EDITED, light));
    let moved = [start[0] + DRAG[1][0], start[1] + DRAG[1][1], start[2]];

    // Slice step 2: move with touch, edit one schema-checked property.
    authoring.drag(MOVED, &DRAG);
    assert_near(authoring.position(MOVED), moved, "the drag carries Wick");
    assert_eq!(
        authoring.history.undo_steps().len(),
        1,
        "the drag is one undo step"
    );
    let before_refusal = authoring.history.revision();
    assert!(
        !authoring.set_field(EDITED, light, "intensity", json!("bright")),
        "a value the component cannot hold is refused"
    );
    assert_eq!(
        authoring.history.revision(),
        before_refusal,
        "a refused edit leaves no trace in history"
    );
    assert!(authoring.set_field(EDITED, light, "intensity", json!(1.25)));
    let edited = authoring.document();

    // Slice step 3: undo and redo.
    authoring.undo();
    authoring.undo();
    assert_eq!(authoring.document(), authored, "undo returns to the file");
    authoring.redo();
    authoring.redo();
    assert_eq!(authoring.document(), edited, "redo returns to the edit");

    // Save, then drop everything, as an app that is closed and opened again.
    authoring
        .file
        .save(&authoring.world, &authoring.components)
        .expect("the scene saves");
    drop(authoring);

    let reopened = Authoring::open(&scene);
    assert_eq!(
        reopened.document(),
        edited,
        "the reopened scene is the edit"
    );
    assert_near(
        reopened.position(MOVED),
        moved,
        "Wick stays where he was put",
    );
    let reopened_intensity = intensity(&reopened.payload(EDITED, light));
    assert!((reopened_intensity - 1.25).abs() < 1e-9);
    assert!((reopened_intensity - start_intensity).abs() > 0.1);

    // Slice step 4: start a real run of the saved project, as a build opens it.
    let mut run =
        ProjectRun::open_scene(&root, "world.scene", [1280.0, 720.0]).expect("the run opens");
    let wick = run.entity(MOVED).expect("the run has Wick");
    let first = run.position(wick);
    assert!(
        (first[0] - moved[0]).abs() < 1e-4 && (first[1] - moved[1]).abs() < 1e-4,
        "the run starts from the edit: {first:?}"
    );
    for _ in 0..120 {
        run.step(STEP).expect("the run steps");
    }
    drop(run);

    // Stopping leaves the authored scene and its file as they were. Keeping or
    // discarding edits made during Play is the desktop's run-edit review, which
    // is still editor-only and not exercised here.
    assert_eq!(
        reopened.document(),
        edited,
        "the run changed a copy, not the authoring world"
    );
    let after = Authoring::open(&scene);
    assert_eq!(after.document(), edited, "playing does not write the scene");
}
