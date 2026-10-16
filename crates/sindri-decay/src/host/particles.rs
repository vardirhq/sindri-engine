//! Performing `Effects.play`, `Effects.stop` and `Effects.emit` on an
//! entity's `sindri.particles` emitter.
//!
//! Playing and stopping write the emitter's own `playing` field, so what a
//! script switched is what the scene shows and what the inspector reads.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::SceneComponent;
use sindri_scene::ParticlesComponent;

use crate::surface::EffectsCall;

use super::WorldHost;
use super::convert::number;

impl WorldHost<'_> {
    pub(super) fn particles_call(
        &mut self,
        call: EffectsCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let entity = self.entity_argument(path, args, 0, "the emitter")?;
        let failed = |why: String| RuntimeError::Host(format!("{}: {why}", path.dotted()));
        let payload = self
            .world
            .get_mut(entity)
            .and_then(|data| data.components.get_mut(ParticlesComponent::TYPE_NAME))
            .ok_or_else(|| failed(format!("entity {} has no particles", entity.index())))?;
        if let EffectsCall::Play | EffectsCall::Stop = call {
            let playing = matches!(call, EffectsCall::Play);
            if let Some(fields) = payload.as_object_mut() {
                fields.insert("playing".to_owned(), serde_json::Value::Bool(playing));
            }
            return Ok(Value::Number(1.0));
        }
        let particles: ParticlesComponent = serde_json::from_value(payload.clone())
            .map_err(|error| failed(format!("its particles do not read: {error}")))?;
        particles
            .validate()
            .map_err(|error| failed(error.to_string()))?;
        let count = number(path, args.get(1).unwrap_or(&Value::Null))?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let count = count.clamp(0.0, f64::from(u32::MAX)) as u32;
        let at = self
            .world
            .world_transform(entity)
            .unwrap_or_default()
            .position_2d();
        let effects = self
            .effects
            .as_deref_mut()
            .ok_or_else(|| failed("this host is not running any effects".to_owned()))?;
        #[allow(clippy::cast_precision_loss)]
        Ok(Value::Number(
            effects.emit_now(entity, &particles, [at[0], at[1]], count) as f64,
        ))
    }
}
