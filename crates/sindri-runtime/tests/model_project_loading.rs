//! Headless loading follows authored collision references in both project layouts.
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use sindri_runtime::{ProjectRun, STEP};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new(assets_dir: bool) -> Self {
        let path = std::env::temp_dir().join(format!(
            "sindri-model-run-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        let assets = if assets_dir {
            path.join("assets")
        } else {
            path.clone()
        };
        fs::create_dir_all(assets.join("models")).unwrap();
        let main = if assets_dir {
            "assets/main.scene"
        } else {
            "main.scene"
        };
        fs::write(
            path.join("sindri.toml"),
            format!("format_version=1\n[project]\nname='Collision'\nmain_scene='{main}'\n"),
        )
        .unwrap();
        Self(path)
    }
    fn assets(&self) -> PathBuf {
        sindri_runtime::project::assets_root(&self.0)
    }
    fn scene(&self, id: &str, disabled: bool) {
        fs::write(self.assets().join("main.scene"), serde_json::json!({"format_version":10,"entities":[{"id":"level","disabled":disabled,"components":{"sindri.physics3d.model_collider":{"asset":id}}}]}).to_string()).unwrap();
    }
    fn model(&self, id: &str) {
        fs::write(
            self.assets().join(id),
            include_bytes!("../../sindri-assets/tests/fixtures/models/static-model.glb"),
        )
        .unwrap();
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn headless_collision_only_models_load_in_both_asset_layouts() {
    for assets_dir in [false, true] {
        let project = Scratch::new(assets_dir);
        project.scene("models/level.glb", false);
        project.model("models/level.glb");
        // Unreferenced/render-only bytes are never prepared as collision.
        fs::write(
            project.assets().join("models/unrelated.glb"),
            b"not a model",
        )
        .unwrap();
        let mut run = ProjectRun::open(&project.0, [640.0, 360.0]).unwrap();
        assert!(run.step(STEP).unwrap().notes().is_empty());
    }
}

#[test]
fn headless_missing_and_malformed_collision_assets_name_the_id_even_when_inactive() {
    for assets_dir in [false, true] {
        let project = Scratch::new(assets_dir);
        project.scene("models/broken.glb", true);
        let error = ProjectRun::open(&project.0, [640.0, 360.0]).err().unwrap();
        assert!(error.contains("models/broken.glb"), "{error}");
        fs::write(project.assets().join("models/broken.glb"), b"broken").unwrap();
        let error = ProjectRun::open(&project.0, [640.0, 360.0]).err().unwrap();
        assert!(error.contains("models/broken.glb"), "{error}");
    }
}

#[test]
fn headless_loads_later_scenes_and_spawnable_nested_prefab_overrides() {
    let project = Scratch::new(true);
    fs::write(
        project.assets().join("main.scene"),
        r#"{"format_version":10,"entities":[]}"#,
    )
    .unwrap();
    fs::write(project.0.join("sindri.toml"), "format_version=1\n[project]\nname='Collision'\nmain_scene='assets/main.scene'\nscenes=['assets/later.scene']\n").unwrap();
    fs::write(project.assets().join("later.scene"), r#"{"format_version":10,"entities":[{"id":"level","prefab":{"source":"nested.prefab","overrides":{"child":{"components":{"sindri.physics3d.model_collider":{"asset":"models/override.glb"}}}}}}]}"#).unwrap();
    fs::write(project.assets().join("part.prefab"), r#"{"format_version":1,"entities":[{"id":"root","components":{"sindri.physics3d.model_collider":{"asset":"models/base.glb"}}}]}"#).unwrap();
    fs::write(
        project.assets().join("nested.prefab"),
        r#"{"format_version":1,"entities":[{"id":"child","prefab":{"source":"part.prefab"}}]}"#,
    )
    .unwrap();
    project.model("models/base.glb");
    let error = ProjectRun::open(&project.0, [640.0, 360.0]).err().unwrap();
    assert!(error.contains("models/override.glb"), "{error}");
    project.model("models/override.glb");
    let mut run = ProjectRun::open_scene(&project.0, "later.scene", [640.0, 360.0]).unwrap();
    assert!(run.step(STEP).unwrap().notes().is_empty());
    fs::remove_file(project.assets().join("models/base.glb")).unwrap();
    let error = ProjectRun::open(&project.0, [640.0, 360.0]).err().unwrap();
    assert!(
        error.contains("models/base.glb"),
        "spawnable prefab: {error}"
    );
}

#[test]
fn convex_collision_only_models_load_in_both_layouts_and_reject_planar_replacements() {
    for assets_dir in [false, true] {
        let project = Scratch::new(assets_dir);
        fs::write(project.assets().join("main.scene"), r#"{"format_version":10,"entities":[{"id":"prop","components":{"sindri.physics3d.model_convex_collider":{"asset":"models/prop.glb"}}}]}"#).unwrap();
        fs::write(
            project.assets().join("models/prop.glb"),
            include_bytes!("../../../examples/model-collision/assets/models/prop.glb"),
        )
        .unwrap();
        let mut run = ProjectRun::open(&project.0, [640.0, 360.0]).unwrap();
        assert!(run.step(STEP).unwrap().notes().is_empty());
        project.model("models/prop.glb");
        let error = ProjectRun::open(&project.0, [640.0, 360.0]).err().unwrap();
        assert!(error.contains("models/prop.glb"), "{error}");
        assert!(error.contains("volumetric"), "{error}");
    }
}
