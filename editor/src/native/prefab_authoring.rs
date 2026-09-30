//! Making a prefab out of something in the scene, and opening one to edit it.
//!
//! Authoring a prefab used to mean writing its JSON by hand, and the editor
//! could only place what already existed. Now a subtree becomes a prefab where
//! it stands — written beside the scene and put back as an instance of itself —
//! and a prefab opens as the document being edited, in the same panels a scene
//! does, with the way back to the scene it was opened from.

use std::path::{Path, PathBuf};

use eframe::egui;
use sindri_core::{CommandBuffer, EntityId, PrefabDocument, WorldCommand};

use crate::prefab;
use crate::ui::icons;
use crate::ui::widgets::button::{self, Intent};
use crate::ui::widgets::{panel, section};

use super::EditorApp;
use super::unsaved::Discarding;

impl EditorApp {
    /// Writes `entity` and everything under it as a prefab, and replaces it
    /// with an instance of that prefab, as one undoable step.
    ///
    /// The file is written under `prefabs/` beside the scene and is not taken
    /// back by Undo, which returns the entities as they were.
    pub(super) fn make_prefab(&mut self, entity: EntityId) {
        if !self.authoring_enabled() {
            return;
        }
        match self.prefab_of(entity) {
            Ok((path, source, document)) => {
                self.replace_with_instance(entity, &path, &source, document);
            }
            Err(error) => self.report(format!("No prefab made: {error}")),
        }
    }

    /// The prefab `entity` would become, and where it would be written.
    fn prefab_of(&self, entity: EntityId) -> Result<(PathBuf, String, PrefabDocument), String> {
        let data = self.world.get(entity).ok_or("that entity is gone")?;
        if data.prefab.as_ref().is_some_and(|link| !link.root)
            && self.world.instance_root(entity).is_some()
        {
            return Err("it is part of another prefab's instance; unpack that first".to_owned());
        }
        let root_id = data.source_id.clone().ok_or("it has no stable ID")?;
        let folder = self
            .file
            .prefabs()
            .root()
            .ok_or("save the scene first, so the prefab has a folder to go in")?
            .to_path_buf();
        let document = prefab::subtree_prefab(&self.world, entity, self.file.prefabs())?;
        let stem = slug(data.name.as_deref().unwrap_or(root_id.as_str()));
        let path = unused_path(&folder.join("prefabs"), &stem);
        let source = self
            .file
            .prefabs()
            .id_for(&path)
            .ok_or("the prefab would be outside the scene's folder")?;
        Ok((path, source, document))
    }

    /// Writes the prefab and swaps the subtree for an instance of it.
    fn replace_with_instance(
        &mut self,
        entity: EntityId,
        path: &Path,
        source: &str,
        document: PrefabDocument,
    ) {
        let written = document
            .to_canonical_json()
            .map_err(|error| error.to_string())
            .and_then(|json| {
                std::fs::create_dir_all(path.parent().unwrap_or(path))
                    .and_then(|()| std::fs::write(path, json))
                    .map_err(|error| error.to_string())
            });
        if let Err(error) = written {
            self.report(format!("{} was not written: {error}", path.display()));
            return;
        }
        self.file.prefabs_mut().replace(source, document);
        let Some(data) = self.world.get(entity) else {
            return;
        };
        let (parent, transform) = (data.parent, data.transform_3d);
        let mut rehearsal = self.world.clone();
        let _ = rehearsal.despawn_recursive(entity);
        let mut buffer = CommandBuffer::new();
        buffer.push(WorldCommand::Despawn { entity });
        let root = match prefab::spawn_instance(
            &mut rehearsal,
            source,
            self.file.prefabs(),
            parent,
            transform,
            &mut buffer,
        ) {
            Ok(root) => root,
            Err(error) => {
                self.report(error);
                return;
            }
        };
        self.history.break_merge_run();
        let label = format!("Make {}", super::instances::file_name(source));
        if let Err(error) = self
            .history
            .apply(buffer.into_transaction(label), &mut self.world)
        {
            self.report(error.to_string());
            return;
        }
        self.console.info(format!("Made {source}"));
        self.refresh_project();
        self.select(Some(root));
    }

    /// Opens the prefab at `path` as the document being edited, remembering
    /// the scene to come back to.
    pub(super) fn edit_prefab(&mut self, path: &Path, context: &egui::Context) {
        if !self.file.is_prefab() {
            self.returning_to = self.file.path().map(Path::to_path_buf);
        }
        self.discard_or_confirm(Discarding::OpenPath(path.to_path_buf()), context);
    }

    /// While a prefab is the open document: which one, what saving it does,
    /// and the way back to the scene it was opened from.
    pub(super) fn prefab_banner(&mut self, ui: &mut egui::Ui) {
        if !self.file.is_prefab() {
            return;
        }
        section::group(ui, icons::PREFAB, &format!("Editing {}", self.file.label()));
        panel::note(
            ui,
            "Saving writes the prefab. Every scene that places it picks the change up, \
             keeping what each instance overrides.",
        );
        let back = self.returning_to.clone();
        if let Some(scene) = back {
            let label = scene.file_name().map_or_else(
                || "the scene".to_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
            if button::labelled(
                ui,
                &format!("Back to {label}"),
                Intent::Primary,
                "Open the scene again",
            )
            .clicked()
            {
                self.returning_to = None;
                self.discard_or_confirm(Discarding::OpenPath(scene), ui.ctx());
            }
        }
        panel::rule(ui);
    }
}

/// A file name made from what an entity is called.
fn slug(name: &str) -> String {
    let slug: String = name
        .trim()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-').to_owned();
    if slug.is_empty() {
        "prefab".to_owned()
    } else {
        slug
    }
}

/// `folder/<stem>.prefab`, or a numbered one when that is taken.
fn unused_path(folder: &Path, stem: &str) -> PathBuf {
    let first = folder.join(format!("{stem}{}", sindri_core::PREFAB_SUFFIX));
    if !first.exists() {
        return first;
    }
    let mut suffix = 2_u32;
    loop {
        let path = folder.join(format!("{stem}-{suffix}{}", sindri_core::PREFAB_SUFFIX));
        if !path.exists() {
            return path;
        }
        suffix += 1;
    }
}
