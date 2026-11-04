//! Undoable object and prefab placement in the mobile scene view.

use sindri_core::{
    CommandBuffer, EntityData, EntityId, PrefabLibrary, SceneEntityId, Transform3D, World,
    WorldCommand,
};
use sindri_scene::Light2dComponent;

use crate::{Editing, OpenedScene};

impl Editing {
    /// Place an authored prefab at the view center as one undoable instance.
    pub fn place_prefab(
        &mut self,
        opened: &mut OpenedScene,
        asset: &str,
    ) -> Result<EntityId, String> {
        if !opened.assets.prefabs.ids().any(|id| id == asset) {
            return Err("This prefab is not in the project".to_owned());
        }
        let path = sindri_runtime::project::assets_root(&opened.project).join(asset);
        let id = opened
            .file
            .prefabs()
            .id_for(&path)
            .ok_or("This prefab cannot be referenced from this scene")?;
        opened.file.prefabs_mut().read_one(&id)?;
        let root = opened
            .file
            .prefabs()
            .prefab(&id)
            .ok_or("The prefab is not loaded")?
            .root()
            .map_err(|error| error.to_string())?;
        let mut transform = root.transform_3d.unwrap_or_default();
        transform.position[0] = self.canvas.center[0];
        transform.position[1] = self.canvas.center[1];
        let mut rehearsal = opened.world.clone();
        let mut commands = CommandBuffer::new();
        let entity = sindri_authoring::prefab::spawn_instance(
            &mut rehearsal,
            &id,
            opened.file.prefabs(),
            None,
            Some(transform),
            &mut commands,
        )?;
        self.history.break_merge_run();
        self.history
            .apply(commands.into_transaction("Add prefab"), &mut opened.world)
            .map_err(|error| error.to_string())?;
        self.clear_selection();
        self.selected = Some(entity);
        Ok(entity)
    }

    /// Add a basic object at the view center as one undoable step.
    pub fn add_basic(&mut self, world: &mut World, kind: &str) -> Result<EntityId, String> {
        let (label, component) = match kind {
            "empty" => ("Empty", None),
            "light2d" => (
                "Light 2D",
                Some(
                    serde_json::to_value(Light2dComponent::default())
                        .map_err(|error| error.to_string())?,
                ),
            ),
            _ => return Err("Unknown object type".to_owned()),
        };
        let mut number = 1u32;
        let source_id = loop {
            let id = SceneEntityId::new(format!(
                "{}-{number}",
                if kind == "empty" { "empty" } else { "light" }
            ))
            .map_err(|error| error.to_string())?;
            if world.entity_for_source_id(&id).is_none() {
                break id;
            }
            number = number.checked_add(1).ok_or("No object IDs remain")?;
        };
        let mut data = EntityData {
            source_id: Some(source_id),
            name: Some(format!("{label} {number}")),
            transform_3d: Some(Transform3D {
                position: [self.canvas.center[0], self.canvas.center[1], 0.0],
                ..Transform3D::default()
            }),
            ..EntityData::default()
        };
        if let Some(component) = component {
            data.components
                .insert("sindri.light2d".to_owned(), component);
        }
        let entity = world.next_handle();
        let mut commands = CommandBuffer::new();
        commands.push(WorldCommand::Spawn {
            entity,
            data: Box::new(data),
        });
        self.history.break_merge_run();
        self.history
            .apply(commands.into_transaction(format!("Add {label}")), world)
            .map_err(|error| error.to_string())?;
        self.clear_selection();
        self.selected = Some(entity);
        Ok(entity)
    }
}
