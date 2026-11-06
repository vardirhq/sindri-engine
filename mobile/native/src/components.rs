//! Add supported components using the engine's validated defaults.

use crate::Editing;
use serde_json::{Value, json};
use sindri_core::{CommandBuffer, ComponentSchemaRegistry, EntityId, World, WorldCommand};
use sindri_scene::SceneExtractor;
use crate::OpenedScene;

const SUPPORTED: &[&str] = &[
    "sindri.light2d",
    "sindri.physics2d.collider",
    "sindri.physics2d.rigid_body",
    "sindri.shadow2d",
    "sindri.light2d.occluder",
];

pub(crate) fn addable(
    world: &World,
    entity: EntityId,
    registry: &ComponentSchemaRegistry,
) -> Vec<Value> {
    let Some(data) = world.get(entity) else {
        return Vec::new();
    };
    let mut choices: Vec<Value> = SUPPORTED
        .iter()
        .filter_map(|kind| {
            if data.components.contains_key(*kind) || registry.default_payload(kind).is_none() {
                return None;
            }
            let metadata = registry.metadata(kind)?;
            Some(json!({"type": kind, "display": metadata.display_name}))
        })
        .collect();
    if !data.components.contains_key("sindri.sprite") && !screen_ui(world, entity) {
        choices.push(json!({"type": "sindri.sprite", "display": "Sprite"}));
    }
    choices
}

fn screen_ui(world: &World, entity: EntityId) -> bool {
    let mut current = Some(entity);
    while let Some(id) = current {
        let Some(data) = world.get(id) else { break; };
        if data.components.keys().any(|name| name.starts_with("sindri.ui.")) { return true; }
        current = data.parent;
    }
    false
}

impl Editing {
    /// Add or replace a sprite image without changing the selected object.
    pub fn set_sprite(&mut self, opened: &mut OpenedScene, texture: &str) -> Result<(), String> {
        let entity = self.selected.ok_or("Nothing is selected")?;
        if screen_ui(&opened.world, entity) { return Err("Screen UI needs an image component instead".to_owned()); }
        if !opened.sprite_hits.contains_texture(texture) { return Err("This image is not loaded in the project".to_owned()); }
        let data = opened.world.get(entity).ok_or("Object no longer exists")?;
        let mut payload = data.components.get("sindri.sprite").cloned().unwrap_or_else(|| json!({}));
        payload.as_object_mut().ok_or("Invalid sprite component")?.insert("texture".to_owned(), json!(texture));
        opened.extractor.components().validate_payload("sindri.sprite", &payload).map_err(|error| error.to_string())?;
        let mut commands = CommandBuffer::new();
        commands.push(WorldCommand::SetComponent { entity, type_name: "sindri.sprite".to_owned(), payload });
        self.history.break_merge_run();
        self.history.apply(commands.into_transaction("Set sprite"), &mut opened.world).map_err(|error| error.to_string())?;
        self.invalidate_outline();
        Ok(())
    }
    /// Add a supported component to the selection in one undoable edit.
    pub fn add_component(
        &mut self,
        world: &mut World,
        extractor: &SceneExtractor,
        kind: &str,
    ) -> Result<(), String> {
        let entity = self.selected.ok_or("Nothing is selected")?;
        let data = world.get(entity).ok_or("Object no longer exists")?;
        if !SUPPORTED.contains(&kind) {
            return Err("This component cannot be added here yet".to_owned());
        }
        if data.components.contains_key(kind) {
            return Err("This object already has that component".to_owned());
        }
        let registry = extractor.components();
        let payload = registry
            .default_payload(kind)
            .cloned()
            .ok_or("This component needs additional setup")?;
        registry
            .validate_payload(kind, &payload)
            .map_err(|error| error.to_string())?;
        let mut commands = CommandBuffer::new();
        commands.push(WorldCommand::SetComponent {
            entity,
            type_name: kind.to_owned(),
            payload,
        });
        self.history.break_merge_run();
        self.history
            .apply(commands.into_transaction("Add component"), world)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
