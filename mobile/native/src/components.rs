//! Add supported components using the engine's validated defaults.

use serde_json::{Value, json};
use sindri_core::{CommandBuffer, ComponentSchemaRegistry, EntityId, World, WorldCommand};
use sindri_scene::SceneExtractor;
use crate::Editing;

const SUPPORTED: &[&str] = &[
    "sindri.light2d", "sindri.physics2d.collider", "sindri.physics2d.rigid_body",
    "sindri.shadow2d", "sindri.light2d.occluder",
];

pub(crate) fn addable(world: &World, entity: EntityId, registry: &ComponentSchemaRegistry) -> Vec<Value> {
    let Some(data) = world.get(entity) else { return Vec::new(); };
    SUPPORTED.iter().filter_map(|kind| {
        if data.components.contains_key(*kind) || registry.default_payload(kind).is_none() { return None; }
        let metadata = registry.metadata(kind)?;
        Some(json!({"type": kind, "display": metadata.display_name}))
    }).collect()
}

impl Editing {
    /// Add a supported component to the selection in one undoable edit.
    pub fn add_component(&mut self, world: &mut World, extractor: &SceneExtractor, kind: &str) -> Result<(), String> {
        let entity = self.selected.ok_or("Nothing is selected")?;
        let data = world.get(entity).ok_or("Object no longer exists")?;
        if !SUPPORTED.contains(&kind) { return Err("This component cannot be added here yet".to_owned()); }
        if data.components.contains_key(kind) { return Err("This object already has that component".to_owned()); }
        let registry = extractor.components();
        let payload = registry.default_payload(kind).cloned().ok_or("This component needs additional setup")?;
        registry.validate_payload(kind, &payload).map_err(|error| error.to_string())?;
        let mut commands = CommandBuffer::new();
        commands.push(WorldCommand::SetComponent { entity, type_name: kind.to_owned(), payload });
        self.history.break_merge_run();
        self.history.apply(commands.into_transaction("Add component"), world).map_err(|error| error.to_string())?;
        Ok(())
    }
}
