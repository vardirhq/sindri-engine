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
use super::prefab_writes::PendingWrite;
use super::unsaved::Discarding;

impl EditorApp {
    /// Writes each of `entities`, and everything under it, as a prefab, and
    /// replaces it with an instance of that prefab, all as one undoable step.
    ///
    /// Each is written under `prefabs/` beside the scene, named after it.
    /// Undo takes the files away again along with the instances, as long as
    /// nobody has changed them since.
    pub(super) fn make_prefabs(&mut self, entities: &[EntityId]) {
        if !self.authoring_enabled() {
            return;
        }
        let entities = crate::selection::topmost(&self.world, entities);
        let mut made = Vec::new();
        for entity in entities {
            // Written one at a time, so the next one's name sees this file and
            // two entities called the same get two files.
            let outcome = self.prefab_of(entity).and_then(|(path, source, document)| {
                let write = PendingWrite::write(&path, &source, &document)?;
                Ok((entity, source, document, write))
            });
            match outcome {
                Ok(one) => made.push(one),
                Err(error) => {
                    self.report(format!("No prefab made: {error}"));
                    return;
                }
            }
        }
        self.replace_with_instances(made);
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
        let document = prefab::subtree_prefab(
            &self.world,
            entity,
            self.file.prefabs(),
            self.scene.components(),
        )?;
        let stem = slug(data.name.as_deref().unwrap_or(root_id.as_str()));
        let path = unused_path(&folder.join("prefabs"), &stem);
        let source = self
            .file
            .prefabs()
            .id_for(&path)
            .ok_or("the prefab would be outside the scene's folder")?;
        Ok((path, source, document))
    }

    /// Swaps each subtree for an instance of the prefab just written from it.
    fn replace_with_instances(
        &mut self,
        made: Vec<(EntityId, String, PrefabDocument, PendingWrite)>,
    ) {
        let mut rehearsal = self.world.clone();
        let mut buffer = CommandBuffer::new();
        let mut roots = Vec::new();
        let mut writes = Vec::new();
        let mut label = "Make prefabs".to_owned();
        for (entity, source, document, write) in made {
            self.file.prefabs_mut().replace(&source, document);
            let Some(data) = self.world.get(entity) else {
                continue;
            };
            let (parent, transform) = (data.parent, data.transform_3d);
            let _ = rehearsal.despawn_recursive(entity);
            buffer.push(WorldCommand::Despawn { entity });
            match prefab::spawn_instance(
                &mut rehearsal,
                &source,
                self.file.prefabs(),
                parent,
                transform,
                &mut buffer,
            ) {
                Ok(root) => roots.push(root),
                Err(error) => {
                    self.report(error);
                    return;
                }
            }
            label = format!("Make {}", super::instances::file_name(&source));
            self.console.info(format!("Made {source}"));
            writes.push(write);
        }
        if writes.len() > 1 {
            label = format!("Make {} prefabs", writes.len());
        }
        self.history.break_merge_run();
        let before = self.history.revision();
        if let Err(error) = self
            .history
            .apply(buffer.into_transaction(label), &mut self.world)
        {
            self.report(error.to_string());
            return;
        }
        self.record_prefab_writes(writes, before);
        self.refresh_project();
        self.selection.clear();
        for root in roots {
            self.selection.toggle(root);
        }
    }

    /// Opens the prefab at `path` as the document being edited, remembering
    /// the scene to come back to.
    pub(super) fn edit_prefab(&mut self, path: &Path, context: &egui::Context) {
        if !self.file.is_prefab() {
            self.prefab_session.returning_to = self.file.path().map(Path::to_path_buf);
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
        let back = self.prefab_session.returning_to.clone();
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
                self.prefab_session.returning_to = None;
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
