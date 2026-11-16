//! Keeping play near the origin (`sindri.floating_origin`), and the order the
//! land's own steps run in.

use sindri_core::{SceneComponent, SceneEntityId, World};
use sindri_scene::FloatingOriginComponent;

use super::Session;

impl Session {
    /// The land near play, after the camera has moved: the world moved back
    /// to the origin if play has gone far, then what stands and the walls
    /// made where play now is.
    pub(super) fn step_land(
        &mut self,
        world: &mut World,
        viewport: (f32, f32),
        problems: &mut Vec<String>,
    ) {
        self.step_origin(world, problems);
        self.step_scatter(world, viewport, problems);
        self.step_walls(world, viewport, problems);
    }

    fn step_origin(&mut self, world: &mut World, problems: &mut Vec<String>) {
        let found = match self.components.query::<FloatingOriginComponent>(world) {
            Ok(found) => found,
            Err(error) => {
                problems.push(format!("Floating origin: {error}"));
                return;
            }
        };
        let Some((holder, mut origin)) = found
            .into_iter()
            .find(|(entity, _)| world.is_active(*entity))
        else {
            return;
        };
        let Some(around) = SceneEntityId::new(origin.around.trim())
            .ok()
            .and_then(|id| world.entity_for_source_id(&id))
        else {
            return;
        };
        let Some(at) = world.world_transform(around).map(|t| t.position) else {
            return;
        };
        let Some(by) = origin.shift_for([at[0], at[1]]) else {
            return;
        };
        move_world(world, by);
        self.effects.shift(by);
        origin.moved[0] += f64::from(by[0]);
        origin.moved[1] += f64::from(by[1]);
        match serde_json::to_value(&origin) {
            Ok(payload) => {
                if let Some(data) = world.get_mut(holder) {
                    data.components
                        .insert(FloatingOriginComponent::TYPE_NAME.to_owned(), payload);
                }
            }
            Err(error) => problems.push(format!("Floating origin: {error}")),
        }
    }
}

/// Moves the world by `by`: every entity with a placing whose ancestors have
/// none (the top of the world under a scene, which only groups), and so
/// everything under them. Screen elements, which are placed on the screen,
/// stay.
fn move_world(world: &mut World, by: [f32; 2]) {
    let placed_above = |world: &World, mut parent: Option<sindri_core::EntityId>| {
        while let Some(entity) = parent {
            let Some(data) = world.get(entity) else {
                return false;
            };
            if data.transform_3d.is_some() {
                return true;
            }
            parent = data.parent;
        }
        false
    };
    let tops: Vec<_> = world
        .entities()
        .filter(|(_, data)| data.transform_3d.is_some())
        .filter(|(_, data)| {
            !data
                .components
                .keys()
                .any(|name| name.starts_with("sindri.ui."))
        })
        .filter(|(_, data)| !placed_above(world, data.parent))
        .map(|(entity, _)| entity)
        .collect();
    for entity in tops {
        if let Some(data) = world.get_mut(entity)
            && let Some(transform) = &mut data.transform_3d
        {
            transform.position[0] += by[0];
            transform.position[1] += by[1];
        }
    }
}
