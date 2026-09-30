//! The strip that says an entity is part of a prefab instance, what that
//! instance changed, and the verbs that go with it.
//!
//! An instance looks like any other entities in the hierarchy and the rest of
//! the inspector, and a value somebody edits there becomes an override the
//! same way. So this is where the difference is visible: which prefab, what
//! here no longer matches it, and a way to take each difference back, keep it
//! for every instance, or cut the link.

use eframe::egui;
use sindri_core::{EntityId, EntityOverride, SceneEntityId};

use crate::prefab::PrefabBrush;
use crate::ui::icons;
use crate::ui::widgets::button::{self, Intent};
use crate::ui::widgets::{panel, section};

use super::super::EditorApp;
use super::super::instances::{Revert, file_name};

/// What was pressed, done once the strip has finished drawing.
enum Pressed {
    Open(String),
    Select(EntityId),
    Revert(EntityId, Revert),
    Apply(EntityId),
    Unpack(EntityId),
}

impl EditorApp {
    /// Draws the strip for `entity`, when it is part of an instance.
    pub(super) fn instance_strip(&mut self, ui: &mut egui::Ui, entity: EntityId) {
        let Some(link) = self.world.get(entity).and_then(|data| data.prefab.clone()) else {
            return;
        };
        let Some(root) = self.world.instance_root(entity) else {
            return;
        };
        let placed = self
            .world
            .instance_entity(root, self.file.prefabs())
            .ok()
            .and_then(|placed| placed.prefab);
        let removed: Vec<SceneEntityId> = placed
            .as_ref()
            .filter(|_| link.root)
            .map(|instance| instance.removed.iter().cloned().collect())
            .unwrap_or_default();
        // Editor state is the instance's own bookkeeping, not a difference
        // anybody made on purpose, so it is not listed as one.
        let changes = placed
            .and_then(|mut instance| instance.overrides.remove(&link.path))
            .map(|changes| EntityOverride {
                editor: std::collections::BTreeMap::new(),
                ..changes
            })
            .unwrap_or_default();
        let mut pressed = None;
        section::group(
            ui,
            icons::PREFAB,
            &format!("Instance of {}", file_name(&link.source)),
        );
        ui.add_enabled_ui(self.authoring_enabled(), |ui| {
            ui.horizontal_wrapped(|ui| {
                if button::labelled(ui, "Open prefab", Intent::Normal, "Show the prefab").clicked()
                {
                    pressed = Some(Pressed::Open(link.source.clone()));
                }
                if !link.root
                    && button::labelled(ui, "Select instance", Intent::Normal, "Select its root")
                        .clicked()
                {
                    pressed = Some(Pressed::Select(root));
                }
                if link.root {
                    if button::labelled(
                        ui,
                        "Apply",
                        Intent::Primary,
                        "Write what this instance changed into the prefab, for every instance",
                    )
                    .clicked()
                    {
                        pressed = Some(Pressed::Apply(root));
                    }
                    if button::labelled(
                        ui,
                        "Revert all",
                        Intent::Normal,
                        "Make the whole instance what the prefab says",
                    )
                    .clicked()
                    {
                        pressed = Some(Pressed::Revert(root, Revert::All));
                    }
                    if button::labelled(
                        ui,
                        "Unpack",
                        Intent::Danger,
                        "Keep these entities, no longer linked to the prefab",
                    )
                    .clicked()
                    {
                        pressed = Some(Pressed::Unpack(root));
                    }
                }
            });
            if let Some(revert) = overridden_rows(ui, &link.path, &changes) {
                pressed = Some(Pressed::Revert(root, revert));
            }
            for path in &removed {
                ui.horizontal(|ui| {
                    panel::note(ui, &format!("Does without {}", path.as_str()));
                    if button::row_icon(ui, icons::UNDO, Intent::Quiet, "Bring it back").clicked() {
                        pressed = Some(Pressed::Revert(root, Revert::Removed(path.clone())));
                    }
                });
            }
        });
        panel::rule(ui);
        match pressed {
            None => {}
            Some(Pressed::Open(source)) => {
                if let Some(path) = self.file.prefabs().path_for(&source) {
                    self.prefab_brush = Some(PrefabBrush::open(&path));
                }
            }
            Some(Pressed::Select(root)) => self.select(Some(root)),
            Some(Pressed::Revert(root, scope)) => self.revert_instance(root, scope),
            Some(Pressed::Apply(root)) => self.apply_instance(root),
            Some(Pressed::Unpack(root)) => self.unpack_instance(root),
        }
    }
}

/// A row per thing this entity overrides, each with a way to take it back.
fn overridden_rows(
    ui: &mut egui::Ui,
    path: &SceneEntityId,
    changes: &EntityOverride,
) -> Option<Revert> {
    if changes.is_empty() {
        panel::note(ui, "Nothing here differs from the prefab.");
        return None;
    }
    let mut reverted = None;
    let mut fields = Vec::new();
    if changes.name.is_some() {
        fields.push("name");
    }
    if changes.transform_3d.is_some() {
        fields.push("transform");
    }
    if changes.disabled.is_some() {
        fields.push("switched on or off");
    }
    if !fields.is_empty() {
        ui.horizontal(|ui| {
            panel::note(ui, &format!("Overrides its {}", fields.join(", ")));
            if button::row_icon(ui, icons::UNDO, Intent::Quiet, "Revert everything here").clicked()
            {
                reverted = Some(Revert::Entity(path.clone()));
            }
        });
    }
    for (type_name, patch) in &changes.components {
        ui.horizontal(|ui| {
            let what = if patch.is_null() {
                format!("Removes {type_name}")
            } else {
                format!("Overrides {type_name}")
            };
            panel::note(ui, &what);
            if button::row_icon(ui, icons::UNDO, Intent::Quiet, "Revert to the prefab's").clicked()
            {
                reverted = Some(Revert::Component(path.clone(), type_name.clone()));
            }
        });
    }
    reverted
}
