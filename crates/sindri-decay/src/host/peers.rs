//! One script reaching another by type: `Bolt.on(hit)`, `bolt.damage`,
//! `bolt.hit(2.0)`.
//!
//! A field is read and written where it lives. On a script that is running,
//! that is its instance, so a write is seen by its next line; on one that has
//! not started — spawned a moment ago, say — it is the value the instance will
//! start with, which is what `World.set_property` authored before this.
//!
//! A call is a message. It is not run there and then: the calling script is in
//! the middle of its own frame and the world is lent to it. It is queued, and
//! delivered once the pass has run, in the order it was sent.
//!
//! An event is the same, sent to everyone listening: `GoalScored.emit(1.0)`
//! is queued alongside the messages and delivered, in its turn, to every
//! running script with an `on GoalScored` handler.

use std::collections::BTreeMap;

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::{EntityId, SceneComponent, part_key, split_part};

use super::WorldHost;
use super::parts::{PartRef, SCRIPT_PART};
use crate::scripts::{Message, Project, Running};
use crate::{ScriptComponent, ScriptSlot};

/// Everything a script needs to reach the others.
pub(crate) struct Peers<'a> {
    /// Every other running script. The one calling is not in here: it is
    /// borrowed for the call, which is also why a script cannot message itself
    /// this way — it calls its own function by name.
    pub(crate) running: &'a mut BTreeMap<ScriptSlot, Running>,
    /// Fields set on scripts that have not started yet.
    pub(crate) starting: &'a mut crate::scripts::StartingValues,
    pub(crate) project: &'a Project,
    pub(crate) messages: &'a mut Vec<Message>,
}

impl<'a> WorldHost<'a> {
    /// Lets this host reach the other scripts in the pass.
    #[must_use]
    pub(crate) fn with_peers(mut self, peers: Peers<'a>) -> Self {
        self.peers = Some(peers);
        self
    }

    /// The script component of one slot.
    fn script_payload(&self, slot: &ScriptSlot) -> Option<&serde_json::Value> {
        self.world
            .get(slot.entity)?
            .components
            .get(&part_key(ScriptComponent::TYPE_NAME, slot.part.as_deref()))
    }

    /// Which script a slot runs, from its `sindri.script` component.
    fn script_name(&self, slot: &ScriptSlot) -> Option<String> {
        self.script_payload(slot)?
            .get("script")?
            .as_str()
            .map(str::to_owned)
    }

    /// The script a reference reaches: one attached as a part, by its part
    /// reference, or an entity's own, by the entity.
    fn peer_slot(&self, subject: Option<u64>) -> Option<ScriptSlot> {
        let reference = subject?;
        match self.blackboard.parts.get(reference) {
            Some(part) if part.kind == SCRIPT_PART => {
                Some(ScriptSlot::new(part.entity, Some(part.name.clone())))
            }
            Some(_) => None,
            None => Some(ScriptSlot::own(EntityId::from_bits(reference))),
        }
    }

    /// The first of an entity's scripts that runs `script`: its own, then its
    /// parts by name.
    fn running_as(&self, entity: EntityId, script: &str) -> Option<ScriptSlot> {
        self.world
            .get(entity)?
            .components
            .iter()
            .find_map(|(key, payload)| {
                let (kind, part) = split_part(key);
                (kind == ScriptComponent::TYPE_NAME
                    && payload.get("script").and_then(serde_json::Value::as_str) == Some(script))
                .then(|| ScriptSlot::new(entity, part.map(str::to_owned)))
            })
    }

    /// `Bolt.on(entity)`: the script on the entity that runs `Bolt` (its
    /// own, else the first part that does), and otherwise null. Its own is
    /// answered as the entity, as it always was; a part as a reference to
    /// the part.
    pub(super) fn script_on(
        &mut self,
        script: &str,
        path: &Path,
        args: &[Value],
    ) -> Result<Option<Value>, RuntimeError> {
        let Some(peers) = &self.peers else {
            return Ok(None);
        };
        if !peers.project.scripts.contains_key(script) {
            return Ok(None);
        }
        let entity = match args.first() {
            Some(Value::Null) => return Ok(Some(Value::Null)),
            _ => self.entity_argument(path, args, 0, "the entity to look on")?,
        };
        Ok(Some(match self.running_as(entity, script) {
            None => Value::Null,
            Some(ScriptSlot { part: None, .. }) => Value::Reference(entity.to_bits()),
            Some(ScriptSlot {
                part: Some(name), ..
            }) => Value::Reference(self.blackboard.parts.intern(PartRef {
                entity,
                kind: SCRIPT_PART,
                name,
            })),
        }))
    }

    /// The script an entity runs and what it declares, when a path through a
    /// reference to it names one of its fields.
    fn declared_field(&self, subject: Option<u64>, path: &Path) -> Option<(ScriptSlot, String)> {
        let peers = self.peers.as_ref()?;
        let slot = self.peer_slot(subject)?;
        let script = self.script_name(&slot)?;
        let field = path.0.first()?;
        peers
            .project
            .scripts
            .get(&script)?
            .fields
            .iter()
            .any(|(name, _)| name == field)
            .then(|| (slot, field.clone()))
    }

    /// `bolt.damage`, and `bolt.heading.x` for a component of a vector field.
    pub(super) fn peer_load(
        &mut self,
        subject: Option<u64>,
        path: &Path,
    ) -> Result<Option<Value>, RuntimeError> {
        let Some((slot, field)) = self.declared_field(subject, path) else {
            return Ok(None);
        };
        let value = self.peer_field(&slot, &field, path)?;
        match path.0.get(1) {
            None => Ok(Some(value)),
            Some(component) => Ok(component_of(&value, component).map(Value::Number)),
        }
    }

    fn peer_field(
        &self,
        slot: &ScriptSlot,
        field: &str,
        path: &Path,
    ) -> Result<Value, RuntimeError> {
        if *slot == self.slot() {
            return Err(own_entity(path));
        }
        let peers = self.peers.as_ref().ok_or_else(|| own_entity(path))?;
        if let Some(running) = peers.running.get(slot) {
            return running
                .instance
                .field(field)
                .cloned()
                .ok_or_else(|| RuntimeError::UnknownPath(path.dotted()));
        }
        // Not started yet: what another script set for it to start with, then
        // what the scene authored.
        if let Some(value) = peers
            .starting
            .get(slot)
            .and_then(|values| values.get(field))
        {
            return Ok(value.clone());
        }
        self.authored(slot)
            .and_then(|properties| properties.get(field).and_then(crate::scripts::to_value))
            .ok_or_else(|| {
                RuntimeError::Host(format!(
                    "`{field}` cannot be read yet: that script has not started, and nothing \
                     has set its starting value"
                ))
            })
    }

    fn authored(&self, slot: &ScriptSlot) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.script_payload(slot)?.get("properties")?.as_object()
    }

    /// `bolt.damage = 3.0`, and `bolt.heading.x = 1.0`.
    pub(super) fn peer_store(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        value: Value,
    ) -> Result<bool, RuntimeError> {
        let Some((slot, field)) = self.declared_field(subject, path) else {
            return Ok(false);
        };
        let value = match path.0.get(1) {
            None => value,
            Some(component) => {
                let whole = self.peer_field(&slot, &field, path)?;
                let index = ["x", "y", "z"]
                    .iter()
                    .position(|known| known == component)
                    .ok_or_else(|| RuntimeError::UnknownPath(path.dotted()))?;
                replace_component(&whole, index, &value)
                    .ok_or_else(|| RuntimeError::UnknownPath(path.dotted()))?
            }
        };
        if slot == self.slot() {
            return Err(own_entity(path));
        }
        let peers = self.peers.as_mut().ok_or_else(|| own_entity(path))?;
        if let Some(running) = peers.running.get_mut(&slot) {
            // A `let` is fixed once the script has started, from outside as
            // much as from inside.
            if running.instance.is_mutable(&field) == Some(false) {
                return Err(RuntimeError::Host(format!(
                    "`{field}` is a `let` on {}, fixed once it has started",
                    running.script
                )));
            }
            running.instance.set_field(&field, value)?;
            return Ok(true);
        }
        // Not started: this becomes what it starts with.
        //
        // An `@export` field is also recorded where `World.set_property` put
        // it, so a script still reading it with `World.property_number` sees
        // what it always did while a project moves over one call at a time.
        peers
            .starting
            .entry(slot.clone())
            .or_default()
            .insert(field.clone(), value.clone());
        let key = part_key(ScriptComponent::TYPE_NAME, slot.part.as_deref());
        if self.script_exports(&slot, &field)
            && let Some(json) = authored_json(&value)
            && let Some(properties) = self
                .world
                .get_mut(slot.entity)
                .and_then(|data| data.components.get_mut(&key))
                .and_then(serde_json::Value::as_object_mut)
                .and_then(|payload| {
                    payload
                        .entry("properties")
                        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
                        .as_object_mut()
                })
        {
            properties.insert(field, json);
        }
        Ok(true)
    }

    /// Whether the script this entity runs marks this field `@export`.
    fn script_exports(&self, slot: &ScriptSlot, field: &str) -> bool {
        let Some(script) = self.script_name(slot) else {
            return false;
        };
        self.peers
            .as_ref()
            .and_then(|peers| peers.project.scripts.get(&script))
            .is_some_and(|declared| declared.exported.iter().any(|name| name == field))
    }

    /// `bolt.hit(2.0)`: queued for after the pass.
    pub(super) fn peer_call(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        args: &[Value],
    ) -> Option<Value> {
        let slot = self.peer_slot(subject)?;
        let [name] = path.0.as_slice() else {
            return None;
        };
        let script = self.script_name(&slot);
        let peers = self.peers.as_mut()?;
        let declared = script
            .as_ref()
            .and_then(|script| peers.project.scripts.get(script))
            .is_some_and(|declared| declared.messages.iter().any(|(known, _)| known == name));
        if !declared {
            return None;
        }
        peers.messages.push(Message {
            to: Some(slot),
            name: name.clone(),
            args: args.to_vec(),
        });
        Some(Value::Unit)
    }
}

impl WorldHost<'_> {
    /// `GoalScored.emit(1.0)`: queued for after the pass, for every script
    /// with a handler for it.
    pub(super) fn emit(&mut self, event: &str, args: &[Value]) -> Option<Value> {
        let peers = self.peers.as_mut()?;
        if !peers.project.events.contains_key(event) {
            return None;
        }
        peers.messages.push(Message {
            to: None,
            name: decay_syntax::handler_name(event),
            args: args.to_vec(),
        });
        Some(Value::Unit)
    }
}

fn own_entity(path: &Path) -> RuntimeError {
    RuntimeError::Host(format!(
        "{} is this script's own entity: write `this.{}` instead",
        path.dotted(),
        path.0.first().map_or("", String::as_str)
    ))
}

fn component_of(value: &Value, component: &str) -> Option<f64> {
    let index = ["x", "y", "z"]
        .iter()
        .position(|known| *known == component)?;
    value.components()?.get(index).copied()
}

fn replace_component(value: &Value, index: usize, number: &Value) -> Option<Value> {
    let Value::Number(number) = number else {
        return None;
    };
    let mut components = value.components()?.to_vec();
    *components.get_mut(index)? = *number;
    Value::vector(&components)
}

/// A value as a scene would author it, for the kinds a scene can hold.
fn authored_json(value: &Value) -> Option<serde_json::Value> {
    Some(match value {
        Value::Number(number) => serde_json::Value::from(*number),
        Value::Bool(flag) => serde_json::Value::from(*flag),
        Value::String(text) => serde_json::Value::from(text.clone()),
        Value::Vec2(components) => serde_json::Value::from(components.to_vec()),
        Value::Vec3(components) => serde_json::Value::from(components.to_vec()),
        Value::Color(channels) => serde_json::Value::from(channels.to_vec()),
        // As a scene authors one: the variant's own name.
        Value::Variant(name) => serde_json::Value::from(crate::scripts::variant_name(name)),
        _ => return None,
    })
}
