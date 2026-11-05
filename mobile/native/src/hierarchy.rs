//! Scene hierarchy and undoable parent changes for the mobile editor.

use sindri_core::{CommandBuffer, EntityId, World, WorldCommand};
use crate::Editing;

fn name(world: &World, entity: EntityId) -> String {
    world.get(entity).and_then(|data| data.name.clone().or_else(|| data.source_id.as_ref().map(|id| id.as_str().to_owned())))
        .unwrap_or_else(|| format!("{entity:?}"))
}

pub(crate) fn rows(world: &World, selected: &[EntityId]) -> Vec<serde_json::Value> {
    let mut roots: Vec<_> = world.entities().filter(|(_, data)| data.parent.is_none()).map(|(id, _)| id).collect();
    roots.sort_by_key(|id| (name(world, *id), id.to_bits()));
    let mut pending: Vec<_> = roots.into_iter().rev().map(|id| (id, 0_usize)).collect();
    let mut rows = Vec::new();
    while let Some((entity, depth)) = pending.pop() {
        let Some(data) = world.get(entity) else { continue; };
        let mut children = data.children.clone();
        children.sort_by_key(|id| (name(world, *id), id.to_bits()));
        rows.push(serde_json::json!({
            "id": entity.to_bits().to_string(), "name": name(world, entity),
            "depth": depth, "hasChildren": !children.is_empty(),
            "selected": selected.contains(&entity), "disabled": data.disabled,
            "screen": data.components.keys().any(|key| key.starts_with("sindri.ui.")),
        }));
        pending.extend(children.into_iter().rev().map(|id| (id, depth + 1)));
    }
    rows
}

fn handle(id: &str) -> Result<EntityId, String> {
    id.parse::<u64>().map(EntityId::from_bits).map_err(|_| "Invalid object handle".to_owned())
}

fn is_screen_ui(world: &World, entity: EntityId) -> bool {
    let mut current = Some(entity);
    while let Some(id) = current {
        let Some(data) = world.get(id) else { break; };
        if data.components.keys().any(|key| key.starts_with("sindri.ui.")) { return true; }
        current = data.parent;
    }
    false
}

impl Editing {
    /// Change one object's parent as a single undoable edit.
    pub fn reparent_object(&mut self, world: &mut World, child: &str, parent: Option<&str>) -> Result<(), String> {
        let child = handle(child)?;
        let parent = parent.map(handle).transpose()?;
        world.check_set_parent(child, parent).map_err(|error| error.to_string())?;
        let data = world.get(child).ok_or("Object no longer exists")?;
        if data.parent == parent { return Ok(()); }
        if data.prefab.as_ref().is_some_and(|link| !link.root && world.instance_root(child).is_some()) {
            return Err("Cannot move a prefab child. Change the prefab or unpack its instance first.".to_owned());
        }
        let mut buffer = CommandBuffer::new();
        buffer.push(WorldCommand::SetParent { entity: child, parent });
        if !is_screen_ui(world, child) {
            let placed = world.world_transform(child).unwrap_or_else(|| world.parent_space(child));
            let space = parent.and_then(|id| world.world_transform(id).or_else(|| Some(world.parent_space(id)))).unwrap_or_default();
            if space.scale.iter().any(|value| value.abs() <= f32::EPSILON) {
                return Err("Cannot preserve the position under a zero-scale parent.".to_owned());
            }
            buffer.push(WorldCommand::SetTransform3D { entity: child, transform: Some(placed.relative_to(space)) });
        }
        self.history.break_merge_run();
        self.history.apply(buffer.into_transaction("Reparent object"), world).map_err(|error| error.to_string())?;
        Ok(())
    }
}
