//! Semantic light validation before an inspector draft becomes a command.
use serde_json::Value;
use sindri_scene::LightComponent;

pub(super) fn validate(payload: &Value) -> Result<(), String> {
    serde_json::from_value::<LightComponent>(payload.clone())
        .map_err(|error| error.to_string())?
        .validate()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::inspector_panel::draft::component_commands;
    use serde_json::json;
    use sindri_core::{CommandHistory, EntityData, World};

    #[test]
    fn invalid_light_edits_emit_no_commands_and_valid_cones_undo_and_redo() {
        let scene = sindri_scene::SceneExtractor::new().unwrap();
        let payload = serde_json::to_value(LightComponent {
            kind: sindri_scene::LightKind::Spot,
            ..Default::default()
        })
        .unwrap();
        let mut world = World::default();
        let entity = world.spawn(EntityData {
            components: [("sindri.light".into(), payload)].into(),
            ..Default::default()
        });
        let initial = world.get(entity).unwrap().components.clone();
        for (field, invalid) in [
            ("outer_angle", json!(0.1)),
            ("inner_angle", json!(2)),
            ("range", json!(0)),
            ("intensity", json!(-1)),
            ("color", json!([-1, 0, 0])),
        ] {
            let mut draft = initial.clone();
            draft.get_mut("sindri.light").unwrap()[field] = invalid;
            let (commands, refused) =
                component_commands(entity, &initial, &draft, scene.components());
            assert!(commands.is_empty(), "{field}");
            assert_eq!(refused.len(), 1);
            assert_eq!(world.get(entity).unwrap().components, initial);
        }
        let mut draft = initial.clone();
        draft.get_mut("sindri.light").unwrap()["outer_angle"] = json!(0.9);
        let (commands, refused) = component_commands(entity, &initial, &draft, scene.components());
        assert!(refused.is_empty());
        let mut history = CommandHistory::default();
        history
            .apply(commands.into_transaction("Edit cone"), &mut world)
            .unwrap();
        assert_eq!(world.get(entity).unwrap().components, draft);
        history.undo(&mut world).unwrap();
        assert_eq!(world.get(entity).unwrap().components, initial);
        history.redo(&mut world).unwrap();
        assert_eq!(world.get(entity).unwrap().components, draft);
    }
}
