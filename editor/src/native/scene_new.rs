//! Making a scene, and putting one somewhere it has not been.
//!
//! Apart from `scene_io` because they answer a different question. That module
//! is about the scene the editor has open: reading it, writing it back,
//! re-reading it. This is about the two moments a scene file does not exist
//! yet — New Scene and Save As — which is what the editor could not do at all.

use std::path::{Path, PathBuf};

use sindri_core::SceneComponent;
use sindri_core::{SCENE_SUFFIX, SceneDocument, SceneEntity, SceneEntityId, SceneMetadata, Transform3D};
use sindri_scene::{LightComponent, SceneExtractor, default_sun_transform};

use crate::scene_file::{SceneFile, scene_path};
use super::CAMERA_COMPONENT;
use super::EditorApp;
use super::hierarchy::row::humanize;
use super::runtime::PLAYING_TIP;

pub(super) fn blank_scene(scene: &SceneExtractor, path: &Path) -> SceneDocument {
    let mut camera = SceneEntity::new(SceneEntityId::new("world-camera").expect("a literal ID is not empty"));
    camera.name = Some("World Camera".to_owned());
    camera.transform_3d = Some(Transform3D { position: [0.0, 0.0, 9.0], ..Transform3D::default() });
    if let Some(payload) = scene.components().default_payload(CAMERA_COMPONENT) {
        camera.components.insert(CAMERA_COMPONENT.to_owned(), payload.clone());
    }
    let mut sun = SceneEntity::new(SceneEntityId::new("sun").expect("a literal ID is not empty"));
    sun.name = Some("Sun".to_owned());
    sun.transform_3d = Some(default_sun_transform());
    if let Some(payload) = scene.components().default_payload(LightComponent::TYPE_NAME) {
        sun.components.insert(LightComponent::TYPE_NAME.to_owned(), payload.clone());
    }
    SceneDocument {
        metadata: SceneMetadata { name: Some(scene_name(path)), ..SceneMetadata::default() },
        entities: vec![sun, camera],
        ..SceneDocument::default()
    }
}

fn scene_name(path: &Path) -> String {
    let file = path.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let lower = file.to_lowercase();
    let stem = if lower.ends_with(SCENE_SUFFIX) {
        &file[..file.len() - SCENE_SUFFIX.len()]
    } else if lower.ends_with(".scene.json") {
        &file[..file.len() - ".scene.json".len()]
    } else if lower.ends_with(".json") {
        &file[..file.len() - ".json".len()]
    } else {
        &file
    };
    humanize(stem)
}

impl EditorApp {
    pub(super) fn save_as(&mut self) {
        if !self.authoring_enabled() {
            self.report(format!("Not saved. {PLAYING_TIP}"));
            return;
        }
        let Some(path) = self.ask_for_scene_path(&self.file.label()) else { return; };
        if let Err(error) = self.file.save_as(&path, &self.world) {
            self.report(error.to_string());
            return;
        }
        self.saved_revision = self.history.revision();
        self.notice = None;
        self.console.info(format!("Saved {}", self.file.label()));
        self.refresh_project();
        self.remember_open_scene();
        self.reload_textures();
        self.reload_scripts();
    }

    pub(super) fn new_scene(&mut self) {
        let Some(path) = self.ask_for_scene_path("untitled.scene") else { return; };
        let document = blank_scene(&self.scene, &path);
        if let Err(error) = SceneFile::create(&path, &document) {
            self.report(error.to_string());
            return;
        }
        self.open_path(&path);
        self.nominate_if_unclaimed(&path);
    }

    fn ask_for_scene_path(&self, suggested: &str) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Sindri scene", &["scene"])
            .set_directory(self.scene_directory())
            .set_file_name(suggested)
            .save_file()
            .map(|chosen| scene_path(&chosen))
    }
}
