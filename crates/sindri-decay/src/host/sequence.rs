//! Playing an authored sequence, and asking where it has got to.
//!
//! The same two halves as `Animation`: which sequence plays is a field of the
//! component, written here and picked up by the next advance; where it has
//! got to, and which cues it has just reached, live beside the world in
//! `Sequences`. Sequences advance after scripts, so a cue reached on one step
//! is answered by `Sequence.cued` on the next.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::SceneComponent;
use sindri_scene::SequenceComponent;

use crate::surface::SequenceCall;

use super::WorldHost;
use super::convert::number;

const COMPONENT: &str = SequenceComponent::TYPE_NAME;

impl WorldHost<'_> {
    pub(super) fn sequence_call(
        &mut self,
        call: SequenceCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let target = self.entity_argument(path, args, 0, "the sequence")?;
        let text = |index: usize, what: &str| match args.get(index) {
            Some(Value::String(text)) => Ok(text.clone()),
            _ => Err(RuntimeError::Host(format!(
                "{} names its {what} with text",
                path.dotted()
            ))),
        };
        match call {
            SequenceCall::Play => {
                let name = text(1, "sequence")?;
                self.write_field(
                    target,
                    COMPONENT,
                    "playing",
                    serde_json::Value::String(name),
                    path,
                )
            }
            SequenceCall::Stop => {
                self.write_field(target, COMPONENT, "playing", serde_json::Value::Null, path)
            }
            SequenceCall::Restart => {
                if let Some(sequences) = self.sequences.as_mut() {
                    sequences.restart(target);
                }
                Ok(Value::Unit)
            }
            SequenceCall::Speed => {
                let speed = number(
                    path,
                    args.get(1).ok_or_else(|| {
                        RuntimeError::Host(format!("{} wants more arguments", path.dotted()))
                    })?,
                )?;
                self.write_field(
                    target,
                    COMPONENT,
                    "speed",
                    serde_json::Value::from(speed),
                    path,
                )
            }
            SequenceCall::Finished => Ok(Value::Bool(
                self.sequences
                    .as_ref()
                    .is_some_and(|sequences| sequences.is_finished(target)),
            )),
            SequenceCall::Time => Ok(Value::Number(f64::from(
                self.sequences
                    .as_ref()
                    .and_then(|sequences| sequences.time(target))
                    .unwrap_or(0.0),
            ))),
            SequenceCall::Cued => {
                let cue = text(1, "cue")?;
                Ok(Value::Bool(
                    self.sequences
                        .as_ref()
                        .is_some_and(|sequences| sequences.cued(target, &cue)),
                ))
            }
            SequenceCall::Name => Ok(Value::String(
                self.world
                    .get(target)
                    .and_then(|data| data.components.get(COMPONENT))
                    .and_then(|payload| payload.get("playing"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            )),
        }
    }
}
