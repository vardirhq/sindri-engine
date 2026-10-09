//! Copying entities and pasting them elsewhere.
//!
//! Duplicate puts a copy beside the original at once; copy and paste let the
//! copy land somewhere else, later: under another entity, at the top level,
//! or at a point in the Scene view. The clipboard keeps the copied subtrees as
//! they were when Copy was pressed, in a world of their own, so what is pasted
//! is what was copied even if the original has since been changed or deleted,
//! and a prefab instance copied whole pastes as a new instance of the same
//! prefab, as a duplicate does (`duplicate.rs`).
//!
//! The clipboard is the editor's own, not the system's: an entity is a
//! subtree of components that only this editor reads. The system's is given
//! the copied entities' names, which is what pasting them anywhere else
//! should give, and which is also how Ctrl+V reaches the editor at all: egui
//! hears a paste only when the system clipboard holds text. A paste whose
//! text is no longer those names is of something copied since, elsewhere,
//! and is left alone.

use eframe::egui;
use glam::Vec3;
use sindri_core::{CommandBuffer, EntityId, Transform3D, World};

use super::duplicate::copy_under;
use crate::native::EditorApp;
use crate::selection;

/// What Copy took.
pub(in crate::native) struct Clipboard {
    /// The copied subtrees, at their own handles, each root standing where
    /// it stood in the world and under nothing.
    world: World,
    /// The topmost entities copied, in the order they were listed, each with
    /// where it stood under its parent.
    roots: Vec<(EntityId, Option<Transform3D>)>,
    /// Their names, as given to the system clipboard.
    text: String,
    /// Whether the system clipboard has been given `text` yet: a menu copies
    /// without the context that writes it, so the next frame does.
    written: bool,
}

impl EditorApp {
    /// Copies the selection to the editor's clipboard.
    pub(in crate::native) fn copy_selection(&mut self) {
        let selected = self.selection.clone();
        self.copy_entities(selected.all());
    }

    /// Copies the entities named, and everything under them.
    pub(in crate::native) fn copy_entities(&mut self, entities: &[EntityId]) {
        let roots = selection::topmost(&self.world, entities);
        if roots.is_empty() {
            return;
        }
        let said = if roots.len() == 1 {
            format!("Copied {}", self.entity_label(roots[0]))
        } else {
            format!("Copied {} entities", roots.len())
        };
        let text = roots
            .iter()
            .map(|&root| self.entity_label(root))
            .collect::<Vec<_>>()
            .join("\n");
        let mut kept = World::default();
        let mut placed = Vec::with_capacity(roots.len());
        for &root in &roots {
            let Ok(subtree) = self.world.capture_subtree(root) else {
                continue;
            };
            let local = self.world.get(root).and_then(|data| data.transform_3d);
            let in_world = self.world.world_transform(root);
            for (entity, mut data) in subtree {
                if entity == root {
                    data.parent = None;
                    data.transform_3d = in_world.or(data.transform_3d);
                }
                // At the handles they had, which no two subtrees share.
                let _ = kept.spawn_at(entity, data);
            }
            placed.push((root, local));
        }
        self.clipboard = Some(Clipboard {
            world: kept,
            roots: placed,
            text,
            written: false,
        });
        self.console.info(said);
    }

    /// Acts on Ctrl+C and Ctrl+V while the hierarchy or a view has the
    /// selection, and hands a copy's names to the system clipboard.
    pub(in crate::native) fn clipboard_keys(
        &mut self,
        context: &egui::Context,
        copied: bool,
        pasted: Option<&str>,
    ) {
        let entities = self.focus == crate::native::Focus::Hierarchy && self.world_editable();
        if entities && copied && !self.selection.is_empty() {
            self.copy_selection();
        }
        if let Some(clipboard) = self.clipboard.as_mut()
            && !clipboard.written
        {
            context.copy_text(clipboard.text.clone());
            clipboard.written = true;
        }
        if entities
            && let Some(text) = pasted
            && self
                .clipboard
                .as_ref()
                .is_some_and(|clipboard| clipboard.text == text.replace("\r\n", "\n"))
        {
            self.paste(None, None);
        }
    }

    /// Whether there is anything to paste.
    pub(in crate::native) const fn can_paste(&self) -> bool {
        self.clipboard.is_some()
    }

    /// Pastes what was copied under `parent`, or at the top level, and selects
    /// it. At the top level each copy keeps where it was in the world; under
    /// an entity it keeps where it was under its own parent. Given `at`, the
    /// first copy lands over that point, at its own depth, and the rest keep
    /// their places around it.
    pub(in crate::native) fn paste(&mut self, parent: Option<EntityId>, at: Option<Vec3>) {
        let Some(clipboard) = self.clipboard.as_ref() else {
            return;
        };
        let origin = clipboard
            .roots
            .first()
            .and_then(|&(root, _)| clipboard.world.world_transform(root))
            .map(|transform| Vec3::from_array(transform.position));
        let mut buffer = CommandBuffer::new();
        let mut rehearsal = self.world.clone();
        let mut pasted = Vec::new();
        for &(root, local) in &clipboard.roots {
            let placed = if parent.is_some() {
                local
            } else {
                clipboard.world.world_transform(root).map(|mut transform| {
                    if let (Some(at), Some(origin)) = (at, origin) {
                        // Moved across the plane the point was picked in,
                        // keeping its depth: a sprite drawn above the
                        // background is pasted above it.
                        let at = Vec3::new(at.x, at.y, origin.z);
                        let offset = Vec3::from_array(transform.position) - origin;
                        transform.position = (at + offset).to_array();
                    }
                    transform
                })
            };
            pasted.extend(copy_under(
                &mut rehearsal,
                &clipboard.world,
                root,
                parent,
                placed,
                &mut buffer,
            ));
        }
        if buffer.is_empty() {
            return;
        }
        self.break_merge_runs();
        let label = if pasted.len() == 1 {
            "Paste entity".to_owned()
        } else {
            format!("Paste {} entities", pasted.len())
        };
        if let Err(error) = self.apply_edit(buffer.into_transaction(label)) {
            self.report(error.to_string());
            return;
        }
        self.select_many(pasted);
        self.refresh_textures();
    }
}
