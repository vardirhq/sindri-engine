//! `Action.*`: what the person means, read from the scene's input actions.
//!
//! A binding is written as text: one source (`"key.Space"`), two for an axis
//! (`"key.A/key.D"`, negative then positive), or four for a direction
//! (`"key.W/key.S/key.A/key.D"`, up, down, left, right). The same text is what
//! `Action.bindings` reads and `Action.rebind` writes, so a rebinding screen
//! shows a binding and changes it in one vocabulary.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use serde_json::json;
use sindri_core::SceneComponent;
use sindri_platform::{
    ActionId, ActionMap, Binding, GamepadButton, InputState, Key, MouseButton, Source,
};
use sindri_scene::InputActionsComponent;

use super::WorldHost;

/// The calls `Action` answers.
pub(crate) const ACTION_CALLS: &[&str] = &[
    "held",
    "pressed",
    "released",
    "axis",
    "vector",
    "bindings",
    "rebind",
    "last_pressed",
];

fn error(path: &Path, message: &str) -> RuntimeError {
    RuntimeError::Host(format!("{}: {message}", path.dotted()))
}

impl WorldHost<'_> {
    pub(super) fn action_call(
        &mut self,
        name: &str,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        if name == "last_pressed" {
            return Ok(Value::String(
                last_pressed(self.context.input).map_or_else(String::new, Source::name),
            ));
        }
        let Some(actions) = self.actions else {
            return Err(error(path, "this host is not reading input actions"));
        };
        let Some(Value::String(action)) = args.first() else {
            return Err(error(path, "takes an action's name first"));
        };
        let id = actions.map.id(action).ok_or_else(|| {
            let known: Vec<&str> = actions.map.names().collect();
            error(
                path,
                &format!(
                    "no input action is called '{action}'; the scene declares {}",
                    if known.is_empty() {
                        "none".to_owned()
                    } else {
                        known.join(", ")
                    }
                ),
            )
        })?;
        let state = actions.state.get(id);
        Ok(match name {
            "held" => Value::Bool(state.held()),
            "pressed" => Value::Bool(state.pressed()),
            "released" => Value::Bool(state.released()),
            "axis" => Value::Number(f64::from(state.axis())),
            "vector" => Value::Vec2(state.vector().map(f64::from)),
            "bindings" => Value::array(
                actions
                    .map
                    .bindings(id)
                    .iter()
                    .map(|binding| Value::String(binding_text(binding)))
                    .collect(),
            ),
            _ => {
                let map = actions.map.clone();
                self.rebind(&map, id, action, path, args)?;
                Value::Unit
            }
        })
    }

    /// `Action.rebind(name, index, binding)`: replaces one of an action's
    /// bindings, checked as declaring it would be, and writes it into the
    /// scene's actions so the next step reads it.
    fn rebind(
        &mut self,
        map: &ActionMap,
        id: ActionId,
        action: &str,
        path: &Path,
        args: &[Value],
    ) -> Result<(), RuntimeError> {
        let (Some(Value::Number(index)), Some(Value::String(text))) = (args.get(1), args.get(2))
        else {
            return Err(error(path, "takes a name, a binding's index and its text"));
        };
        let mut bindings = map.bindings(id).to_vec();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let at = index.max(0.0) as usize;
        if !index.is_finite() || index.fract() != 0.0 || at > bindings.len() {
            return Err(error(
                path,
                &format!(
                    "'{action}' has {} bindings; an index is one of them, or the next to add one",
                    bindings.len()
                ),
            ));
        }
        let binding = parse(text)
            .ok_or_else(|| error(path, &format!("'{text}' is not a binding this build reads")))?;
        if at == bindings.len() {
            bindings.push(binding);
        } else {
            bindings[at] = binding;
        }
        // Checked against the map first: rebinding is not a way around what
        // declaring refuses, such as a direction bound to a single key.
        map.clone()
            .rebind(id, bindings.clone())
            .map_err(|refused| error(path, &refused.to_string()))?;
        let written: Vec<serde_json::Value> = bindings.iter().map(binding_json).collect();
        let holder = self
            .world
            .entities()
            .find(|(_, data)| {
                data.components
                    .contains_key(InputActionsComponent::TYPE_NAME)
            })
            .map(|(entity, _)| entity);
        let entry = holder
            .and_then(|entity| self.world.get_mut(entity))
            .and_then(|data| data.components.get_mut(InputActionsComponent::TYPE_NAME))
            .and_then(|payload| payload.get_mut("actions"))
            .and_then(serde_json::Value::as_array_mut)
            .and_then(|entries| {
                entries.iter_mut().find(|entry| {
                    entry.get("name").and_then(serde_json::Value::as_str) == Some(action)
                })
            })
            .ok_or_else(|| error(path, "the scene's input actions are gone"))?;
        entry["bindings"] = json!(written);
        Ok(())
    }
}

/// The first key, mouse button or pad button pressed this step, for a
/// rebinding screen waiting for "press the key to use".
fn last_pressed(input: &InputState) -> Option<Source> {
    Key::ALL
        .iter()
        .find(|key| input.key_pressed(**key))
        .map(|key| Source::Key(*key))
        .or_else(|| {
            MouseButton::ALL
                .iter()
                .find(|button| input.button_pressed(**button))
                .map(|button| Source::MouseButton(*button))
        })
        .or_else(|| {
            GamepadButton::ALL
                .into_iter()
                .find(|button| input.gamepads().pressed(0, *button))
                .map(Source::GamepadButton)
        })
}

fn binding_text(binding: &Binding) -> String {
    binding
        .sources()
        .into_iter()
        .map(Source::name)
        .collect::<Vec<_>>()
        .join("/")
}

fn parse(text: &str) -> Option<Binding> {
    let sources: Option<Vec<Source>> = text.split('/').map(Source::from_name).collect();
    match sources?.as_slice() {
        [source] => Some(Binding::Simple(*source)),
        [negative, positive] => Some(Binding::Axis {
            negative: *negative,
            positive: *positive,
        }),
        [up, down, left, right] => Some(Binding::Vector {
            up: *up,
            down: *down,
            left: *left,
            right: *right,
        }),
        _ => None,
    }
}

fn binding_json(binding: &Binding) -> serde_json::Value {
    let sources: Vec<String> = binding.sources().into_iter().map(Source::name).collect();
    match sources.as_slice() {
        [negative, positive] => json!({"axis": {"negative": negative, "positive": positive}}),
        [up, down, left, right] => {
            json!({"vector": {"up": up, "down": down, "left": left, "right": right}})
        }
        _ => json!(sources.first()),
    }
}
