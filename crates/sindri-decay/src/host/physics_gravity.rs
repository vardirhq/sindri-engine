//! Reading and changing which way is down, through the scene's authored
//! Physics 2D World rather than behind its back.
//!
//! Gravity is authored: the first active `sindri.physics2d.world` decides it
//! at every fixed step. A script changes that component, so what the scene
//! says and what the solver does stay one thing, the change survives the
//! solver being rebuilt, and the editor's inspector shows it during Play.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::{EntityId, SceneComponent};
use sindri_scene::PhysicsWorld2dComponent;

use super::WorldHost;
use crate::surface::PhysicsCall;

impl WorldHost<'_> {
    pub(super) fn gravity_call(
        &mut self,
        call: PhysicsCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let Some(physics) = self.physics.as_ref() else {
            return Err(error("needs physics, and this host is not running any"));
        };
        let settings = self.active_world_settings();
        if call == PhysicsCall::Gravity {
            let [x, y] = settings
                .and_then(|entity| {
                    let payload = self
                        .world
                        .get(entity)?
                        .components
                        .get(PhysicsWorld2dComponent::TYPE_NAME)?;
                    serde_json::from_value::<PhysicsWorld2dComponent>(payload.clone()).ok()
                })
                .map_or_else(|| physics.world.gravity(), |authored| authored.gravity);
            return Ok(Value::Vec2([f64::from(x), f64::from(y)]));
        }
        let Some(Value::Vec2([x, y])) = args.first() else {
            return Err(error("takes the new gravity as a Vec2"));
        };
        #[allow(clippy::cast_possible_truncation)]
        // Physics is f32; the finiteness check below covers overflow.
        let gravity = [*x as f32, *y as f32];
        if !gravity.iter().all(|axis| axis.is_finite()) {
            return Err(error("gravity must be finite"));
        }
        let entity = settings.ok_or_else(|| {
            error("needs an active Physics 2D World to change; this scene authors none")
        })?;
        let payload = self
            .world
            .get_mut(entity)
            .and_then(|data| data.components.get_mut(PhysicsWorld2dComponent::TYPE_NAME))
            .ok_or_else(|| error("the Physics 2D World went away"))?;
        // Only the gravity changes; layer names and unknown fields stay.
        payload["gravity"] = serde_json::json!(gravity);
        Ok(Value::Unit)
    }

    /// The entity whose world settings the solver uses: the first active one.
    fn active_world_settings(&self) -> Option<EntityId> {
        self.world
            .entities()
            .find(|(entity, data)| {
                self.world.is_active(*entity)
                    && data
                        .components
                        .contains_key(PhysicsWorld2dComponent::TYPE_NAME)
            })
            .map(|(entity, _)| entity)
    }
}
