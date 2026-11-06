//! Undoable changes to the pieces of a 2D collider.

use crate::Editing;
use serde_json::{Value, json};
use sindri_core::{CommandBuffer, World, WorldCommand};
use sindri_scene::SceneExtractor;

const COLLIDER: &str = "sindri.physics2d.collider";

impl Editing {
    /// Add a default piece or remove one, preserving the other authored pieces.
    pub fn edit_collider_pieces(
        &mut self,
        world: &mut World,
        extractor: &SceneExtractor,
        index: usize,
        add: bool,
    ) -> Result<(), String> {
        let entity = self.selected.ok_or("Nothing is selected")?;
        let stored = world
            .get(entity)
            .and_then(|data| data.components.get(COLLIDER))
            .cloned()
            .ok_or("This object has no collider")?;
        let compound = stored.get("pieces").and_then(Value::as_array);
        let mut pieces = compound.cloned().unwrap_or_else(|| vec![stored.clone()]);
        if add {
            let piece = extractor
                .components()
                .default_payload(COLLIDER)
                .and_then(|value| value.get("pieces"))
                .and_then(Value::as_array)
                .and_then(|pieces| pieces.first())
                .cloned()
                .ok_or("No default collider shape")?;
            pieces.push(piece);
        } else {
            if pieces.len() <= 1 {
                return Err("A collider needs at least one shape".to_owned());
            }
            if index >= pieces.len() {
                return Err("That collider shape no longer exists".to_owned());
            }
            pieces.remove(index);
        }
        let mut payload = if compound.is_some() {
            stored
        } else {
            json!({})
        };
        payload
            .as_object_mut()
            .ok_or("Invalid collider")?
            .insert("pieces".to_owned(), json!(pieces));
        extractor
            .components()
            .validate_payload(COLLIDER, &payload)
            .map_err(|error| error.to_string())?;
        let mut commands = CommandBuffer::new();
        commands.push(WorldCommand::SetComponent {
            entity,
            type_name: COLLIDER.to_owned(),
            payload,
        });
        self.history.break_merge_run();
        self.history
            .apply(commands.into_transaction("Edit collider shapes"), world)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
