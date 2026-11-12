//! The bridge's side of a prefab open in the object workspace.
//!
//! While one is open, the commands the workspace sends (field edits, a drag's
//! settle, collider shapes, undo, redo and save) act on the prefab rather than
//! the scene, and the state the app reads shows the prefab's root in place of
//! the scene's selection. Everything else still acts on the scene behind it.

use serde_json::Value;

use super::Host;
use crate::PrefabSession;

/// The commands a prefab session takes while it is open.
const ROUTED: [&str; 7] = [
    "setField",
    "settle",
    "colliderAdd",
    "colliderRemove",
    "undo",
    "redo",
    "save",
];

impl Host {
    /// Opens the project's prefab `asset` in the workspace, saving the scene
    /// first so the prefab's instances are followed from what is on disk.
    pub(super) fn open_prefab(&mut self, asset: &str) -> Result<(), String> {
        if self.playing.is_some() {
            return Err("Stop Play before editing a prefab.".to_owned());
        }
        if self.recovery_conflict {
            return Err("Choose which scene to keep before editing a prefab.".to_owned());
        }
        self.close_prefab()?;
        if self.editing.as_ref().is_some_and(crate::Editing::unsaved) {
            self.save()?;
        }
        let opened = self.opened.as_ref().ok_or("no scene is open")?;
        self.prefab = Some(PrefabSession::open(opened, asset)?);
        Ok(())
    }

    /// Closes the open prefab, saving it first if it holds edits.
    pub(super) fn close_prefab(&mut self) -> Result<(), String> {
        if self
            .prefab
            .as_ref()
            .is_some_and(|session| session.editing.unsaved())
        {
            self.save_prefab()?;
        }
        self.prefab = None;
        Ok(())
    }

    /// Writes the prefab, follows it in the scene, and saves the scene the
    /// way every scene edit is saved.
    fn save_prefab(&mut self) -> Result<(), String> {
        let session = self.prefab.as_mut().ok_or("no prefab is open")?;
        let opened = self.opened.as_mut().ok_or("no scene is open")?;
        let editing = self.editing.as_mut().ok_or("no scene is open")?;
        if session.save(opened, editing)? {
            self.save()?;
        }
        Ok(())
    }

    /// Runs `command` on the open prefab, or `None` when it is not one the
    /// prefab takes or no prefab is open.
    pub(super) fn prefab_command(
        &mut self,
        command: &str,
        arguments: &[Value],
    ) -> Option<Result<(), String>> {
        if self.prefab.is_none() || !ROUTED.contains(&command) {
            return None;
        }
        if command == "save" {
            return Some(self.save_prefab());
        }
        let text = |index: usize| {
            arguments
                .get(index)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{command}: argument {index} is not text"))
        };
        let extractor = &self.opened.as_ref()?.extractor;
        let session = self.prefab.as_mut()?;
        let world = &mut session.world;
        let editing = &mut session.editing;
        Some(match command {
            "setField" => (|| {
                let value = arguments
                    .get(2)
                    .cloned()
                    .ok_or_else(|| format!("{command}: no value"))?;
                let merging = arguments.get(3).and_then(Value::as_bool).unwrap_or(false);
                editing.set_field(world, extractor, text(0)?, text(1)?, value, merging)
            })(),
            "settle" => {
                editing.settle();
                Ok(())
            }
            "colliderAdd" | "colliderRemove" => (|| {
                let index = text(0)?.parse::<usize>().unwrap_or(0);
                editing.edit_collider_pieces(world, extractor, index, command == "colliderAdd")
            })(),
            "undo" => editing.undo(world),
            "redo" => editing.redo(world),
            _ => Err(format!("unknown command {command}")),
        })
    }

    /// The open prefab's root as the inspector shows it, its history, and
    /// whether it holds edits; `None` with no prefab open.
    pub(super) fn prefab_state(&self) -> Option<(Value, Option<String>, Option<String>, bool)> {
        let session = self.prefab.as_ref()?;
        let opened = self.opened.as_ref()?;
        Some((
            session
                .editing
                .inspector(&session.world, &opened.extractor, &opened.assets.sheets),
            session.editing.history.undo_label().map(str::to_owned),
            session.editing.history.redo_label().map(str::to_owned),
            session.editing.unsaved(),
        ))
    }
}
