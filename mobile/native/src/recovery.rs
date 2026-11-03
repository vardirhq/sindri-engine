//! Durable, app-private recovery for a scene whose save did not finish.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};
use sindri_core::{SceneDocument, SceneMigrator, UnknownComponentPolicy, World};

use crate::OpenedScene;

/// Recovery is kept outside the project so it survives a project write failure.
pub struct Recovery {
    directory: PathBuf,
    scene_path: PathBuf,
    base: String,
}

impl Recovery {
    /// Opens the recovery store for a scene. Existing recovery is never replaced
    /// until it has been restored, discarded, or confirmed already saved.
    pub fn new(directory: &Path, opened: &OpenedScene) -> Result<Self, String> {
        fs::create_dir_all(directory).map_err(|e| format!("recovery folder: {e}"))?;
        let scene_path = opened.file.path().ok_or("scene has no path")?.to_path_buf();
        let base = fs::read_to_string(&scene_path).map_err(|e| format!("scene: {e}"))?;
        Ok(Self {
            directory: directory.to_path_buf(),
            scene_path,
            base,
        })
    }

    fn journal(&self) -> PathBuf {
        self.directory.join("recovery.json")
    }

    fn read(&self) -> Result<Option<Value>, String> {
        let text = match fs::read_to_string(self.journal()) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("recovery read: {e}")),
        };
        let value: Value =
            serde_json::from_str(&text).map_err(|e| format!("recovery is invalid: {e}"))?;
        let expected_path = self.scene_path.to_str().ok_or("invalid scene path")?;
        if value.get("path").and_then(Value::as_str) != Some(expected_path)
            || value.get("base").and_then(Value::as_str).is_none()
            || value.get("scene").and_then(Value::as_str).is_none()
        {
            return Err("recovery belongs to another scene or is invalid".to_owned());
        }
        Ok(Some(value))
    }

    /// Returns whether a recovery exists and whether the scene changed since it was made.
    pub fn pending(&self) -> Result<Option<bool>, String> {
        let Some(value) = self.read()? else {
            return Ok(None);
        };
        let scene = value["scene"].as_str().ok_or("invalid recovery scene")?;
        if self.base == scene {
            self.clear()?;
            return Ok(None);
        }
        Ok(Some(value["base"].as_str() != Some(self.base.as_str())))
    }

    /// Write the current edit and its original scene as one atomic journal.
    pub fn checkpoint(&self, opened: &OpenedScene) -> Result<(), String> {
        let scratch = self.directory.join("snapshot.scene");
        let mut file = opened.file.clone();
        file.save_as(&scratch, &opened.world, opened.extractor.components())
            .map_err(|e| format!("recovery snapshot: {e}"))?;
        let scene = fs::read_to_string(&scratch).map_err(|e| format!("recovery snapshot: {e}"))?;
        let _ = fs::remove_file(scratch);
        let data = json!({"path": self.scene_path.to_str().ok_or("invalid scene path")?, "base": self.base, "scene": scene}).to_string();
        let temporary = self.directory.join("recovery.tmp");
        let mut output =
            fs::File::create(&temporary).map_err(|e| format!("recovery write: {e}"))?;
        output
            .write_all(data.as_bytes())
            .and_then(|()| output.sync_all())
            .map_err(|e| format!("recovery write: {e}"))?;
        drop(output);
        fs::rename(temporary, self.journal()).map_err(|e| format!("recovery replace: {e}"))
    }

    /// Restore into memory, leaving the project file and recovery copy intact.
    pub fn restore(&self, opened: &mut OpenedScene) -> Result<(), String> {
        let value = self.read()?.ok_or("no recovery exists")?;
        let text = value["scene"].as_str().ok_or("invalid recovery scene")?;
        let document = SceneDocument::from_json_migrated(text, &SceneMigrator::builtin())
            .map_err(|e| e.to_string())?;
        let expanded = document
            .expanded(opened.file.prefabs())
            .map_err(|e| e.to_string())?;
        opened
            .extractor
            .validate(&expanded, UnknownComponentPolicy::Preserve)
            .map_err(|e| e.to_string())?;
        opened.world = World::from_scene_with(&document, opened.file.prefabs())
            .map_err(|e| e.to_string())?
            .world;
        Ok(())
    }

    /// Refuse to replace a scene modified by another editor since it was opened.
    pub fn check_original(&self) -> Result<(), String> {
        let current =
            fs::read_to_string(&self.scene_path).map_err(|e| format!("scene read: {e}"))?;
        if current != self.base {
            return Err("The scene changed outside the editor. Reopen to choose a version; recovery was kept.".to_owned());
        }
        Ok(())
    }

    /// Update the baseline only after a successful save, then remove the journal.
    pub fn saved(&mut self) -> Result<(), String> {
        self.base = fs::read_to_string(&self.scene_path).map_err(|e| format!("scene read: {e}"))?;
        self.clear()
    }

    /// Explicitly discard the stored recovery.
    pub fn clear(&self) -> Result<(), String> {
        match fs::remove_file(self.journal()) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("recovery removal: {e}")),
        }
    }
}
