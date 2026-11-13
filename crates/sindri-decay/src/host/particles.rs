//! Performing `Effects.play`, `Effects.stop` and `Effects.emit` on an
//! entity's `sindri.particles` emitter.
//!
//! Playing and stopping write the emitter's own `playing` field, so what a
//! script switched is what the scene shows and what the inspector reads. The
//! same acts are an emitter's own methods, `this.particles.play()`, which
//! reach a named part too.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::{EntityId, SceneComponent, part_key};
use sindri_scene::ParticlesComponent;

use crate::surface::EffectsCall;
use crate::surface::particles::{CALLS, ParticlesCall};

use super::WorldHost;
use super::convert::number;

impl WorldHost<'_> {
    /// `Effects.play(entity)` and its kin: the entity's own emitter.
    pub(super) fn particles_call(
        &mut self,
        call: EffectsCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let entity = self.entity_argument(path, args, 0, "the emitter")?;
        let action = match call {
            EffectsCall::Play => ParticlesCall::Play,
            EffectsCall::Stop => ParticlesCall::Stop,
            _ => ParticlesCall::Emit,
        };
        self.particles_act(entity, None, action, args.get(1), path)
    }

    /// `this.particles.play()` and `this.entity.particles("surge").emit(4)`:
    /// an emitter's own methods, on the object's own emitter or the part
    /// being served. `None` when the call is not one of them.
    pub(super) fn particles_method(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        args: &[Value],
    ) -> Result<Option<Value>, RuntimeError> {
        let Some(parts) = Self::addressed(subject, path) else {
            return Ok(None);
        };
        let name = match parts.as_slice() {
            ["particles", name] | ["entity", "particles", name] => *name,
            _ => return Ok(None),
        };
        let Some((_, call)) = CALLS.iter().find(|(known, _)| *known == name) else {
            return Ok(None);
        };
        let entity = self.subject(subject, path)?;
        let part = self.part_of(entity, ParticlesComponent::TYPE_NAME);
        self.particles_act(entity, part.as_deref(), *call, args.first(), path)
            .map(Some)
    }

    /// Plays, stops or throws from one emitter: `part` names it, `None` for
    /// the entity's own.
    fn particles_act(
        &mut self,
        entity: EntityId,
        part: Option<&str>,
        call: ParticlesCall,
        count: Option<&Value>,
        path: &Path,
    ) -> Result<Value, RuntimeError> {
        let failed = |why: String| RuntimeError::Host(format!("{}: {why}", path.dotted()));
        let key = part_key(ParticlesComponent::TYPE_NAME, part);
        let payload = self
            .world
            .get_mut(entity)
            .and_then(|data| data.components.get_mut(&key))
            .ok_or_else(|| {
                failed(match part {
                    Some(part) => {
                        format!("entity {} has no particles named {part}", entity.index())
                    }
                    None => format!("entity {} has no particles", entity.index()),
                })
            })?;
        if let ParticlesCall::Play | ParticlesCall::Stop = call {
            let playing = matches!(call, ParticlesCall::Play);
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
        let count = number(path, count.unwrap_or(&Value::Null))?;
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
            effects.emit_part_now(entity, part, &particles, [at[0], at[1]], count) as f64,
        ))
    }
}
