//! The scene's input actions, as scripts read them each step.
//!
//! `sindri.input.actions` declares them as an actions document; this reads it
//! into an `ActionMap` when it changes and works out every action's value from
//! the step's input before any script runs, so every script in a pass sees the
//! same answers. Rebinding is a write to the component, which is read back on
//! the next step like any other change.

use sindri_core::{SceneComponent, World};
use sindri_platform::{ActionMap, Actions, InputState};
use sindri_scene::InputActionsComponent;

/// The actions a running scene declares, and what each is worth this step.
#[derive(Default)]
pub(crate) struct InputActions {
    /// The payload last read, to tell a change from the same declaration.
    read: Option<serde_json::Value>,
    pub(crate) map: ActionMap,
    pub(crate) state: Actions,
}

impl InputActions {
    /// Reads the declaration if it changed and every action's value from
    /// `input`. A declaration that does not read is reported once, when it
    /// changes, and leaves no actions rather than half of them.
    pub(crate) fn update(&mut self, world: &World, input: &InputState) -> Option<String> {
        let payload = world
            .entities()
            .find_map(|(_, data)| data.components.get(InputActionsComponent::TYPE_NAME))
            .cloned();
        let mut problem = None;
        if payload != self.read {
            self.map = match &payload {
                None => ActionMap::default(),
                Some(payload) => {
                    ActionMap::from_json(&payload.to_string()).unwrap_or_else(|error| {
                        problem = Some(format!("the scene's input actions do not read: {error}"));
                        ActionMap::default()
                    })
                }
            };
            self.state = Actions::default();
            self.read = payload;
        }
        self.state.update(&self.map, input);
        problem
    }
}
