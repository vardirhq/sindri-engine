//! Selection actions from the mobile toolbar.

use super::Host;
use serde_json::Value;

impl Host {
    pub(super) fn add_command(&mut self, command: &str, value: &str) -> Result<(), String> {
        let (opened, editing) = self.editing()?;
        match command {
            "addPrefab" => {
                editing.place_prefab(opened, value)?;
            }
            "addBasic" => {
                editing.add_basic(&mut opened.world, value)?;
            }
            _ => return Err(format!("unknown add command {command}")),
        }
        Ok(())
    }

    pub(super) fn object_command(
        &mut self,
        command: &str,
        arguments: &[Value],
    ) -> Result<(), String> {
        match command {
            "clearObjects" => {
                self.objects.clear();
                Ok(())
            }
            "refreshObjects" => {
                let (opened, editing) = self.editing()?;
                self.objects = editing.scene_objects(&opened.world);
                Ok(())
            }
            "reparentObject" => {
                let child = arguments.first().and_then(Value::as_str).ok_or("Missing object")?;
                let parent = arguments.get(1).and_then(Value::as_str).filter(|id| !id.is_empty());
                let (opened, editing) = self.editing()?;
                editing.reparent_object(&mut opened.world, child, parent)
            }
            "gizmoPress" => {
                let id = arguments
                    .first()
                    .and_then(Value::as_str)
                    .ok_or("gizmoPress: no object")?;
                let number = |index: usize| -> Result<f32, String> {
                    #[allow(clippy::cast_possible_truncation)] // Surface coordinates fit in f32.
                    arguments
                        .get(index)
                        .and_then(Value::as_f64)
                        .map(|value| value as f32)
                        .filter(|value| value.is_finite())
                        .ok_or_else(|| format!("gizmoPress: invalid coordinate {index}"))
                };
                let pixel = [number(1)?, number(2)?];
                let (opened, editing) = self.editing()?;
                editing.press_gizmo(&opened.world, id, pixel)
            }
            "selectObject" => {
                let id = arguments
                    .first()
                    .and_then(Value::as_str)
                    .ok_or_else(|| format!("{command}: argument 0 is not text"))?;
                let (opened, editing) = self.editing()?;
                editing.select_object(&opened.world, id)
            }
            _ => Err(format!("unknown object command {command}")),
        }
    }

    pub(super) fn selection_command(&mut self, command: &str) -> Result<(), String> {
        let (opened, editing) = self.editing()?;
        match command {
            "selectMore" => {
                editing.select_more();
                Ok(())
            }
            "duplicate" => editing.duplicate_selection(&mut opened.world),
            "delete" => editing.delete_selection(&mut opened.world),
            _ => Err(format!("unknown selection command {command}")),
        }
    }
}
