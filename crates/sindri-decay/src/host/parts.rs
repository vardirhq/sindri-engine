//! A named part, reached from a script: `this.entity.light2d("halo")`.
//!
//! An entity can hold several components of one kind as named parts
//! (`sindri.light2d` and `sindri.light2d#halo`; `docs/object-parts.md`). The
//! member alone is the entity's own part, as it always was; called with a
//! name it answers a reference to another part of that kind, or `null`.
//!
//! The reference is the runtime's opaque `Value::Reference`, so a script holds,
//! compares and stores it as it does an entity. What it means is kept here, on
//! the blackboard that outlives each call: an interned list of (entity, kind,
//! part), the top bit of the reference marking it as one of these rather than
//! an entity. An entity's bits are its slot in the high half and its
//! generation in the low, and no world holds two billion slots, so the top bit
//! is never an entity's.
//!
//! While a part is being read or written the host swaps the component key the
//! leaf names (`sindri.light2d`) for the part's (`sindri.light2d#halo`), so
//! every member of the kind, its color channels included, reaches the part by
//! the same code that reaches the entity's own.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::{EntityId, part_key};

use super::WorldHost;
use crate::surface::PART_KINDS;

/// The bit that marks a reference as a part rather than an entity.
const PART_BIT: u64 = 1 << 63;

/// One part a script has reached: its entity, its kind's member name and the
/// part's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PartRef {
    pub entity: EntityId,
    pub kind: &'static str,
    pub name: String,
}

/// Every part scripts have reached, so the same part is always the same
/// reference and `a == b` means what a script expects.
#[derive(Clone, Debug, Default)]
pub struct PartRefs {
    parts: Vec<PartRef>,
}

impl PartRefs {
    /// The reference for `part`, the same one each time it is asked.
    pub(crate) fn intern(&mut self, part: PartRef) -> u64 {
        let index = self
            .parts
            .iter()
            .position(|known| *known == part)
            .unwrap_or_else(|| {
                self.parts.push(part);
                self.parts.len() - 1
            });
        PART_BIT | index as u64
    }

    /// The part a reference names, or `None` for an entity's reference.
    pub(crate) fn get(&self, reference: u64) -> Option<&PartRef> {
        if reference & PART_BIT == 0 {
            return None;
        }
        usize::try_from(reference & !PART_BIT)
            .ok()
            .and_then(|index| self.parts.get(index))
    }
}

/// The component a kind's member reaches, if it has named parts.
fn component_of(kind: &str) -> Option<(&'static str, &'static str)> {
    PART_KINDS
        .iter()
        .find(|(member, _)| *member == kind)
        .copied()
}

impl WorldHost<'_> {
    /// `light2d("halo")`: the part's reference, or `null` when the entity has
    /// no part of that name. `None` when the call is not a part lookup.
    pub(super) fn part_lookup(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        args: &[Value],
    ) -> Option<Result<Value, RuntimeError>> {
        let under = Self::addressed(subject, path)?;
        let kind = match (subject, under.as_slice()) {
            (_, [kind]) | (None, ["entity", kind]) => *kind,
            _ => return None,
        };
        let (kind, component) = component_of(kind)?;
        if subject.is_some_and(|reference| self.blackboard.parts.get(reference).is_some()) {
            return Some(Err(RuntimeError::Host(format!(
                "{} asks a part for a part; ask its entity",
                path.dotted()
            ))));
        }
        let name = match args {
            [Value::String(name)] => name.clone(),
            _ => {
                return Some(Err(RuntimeError::Host(format!(
                    "{}(...) takes the part's name",
                    path.dotted()
                ))));
            }
        };
        Some(self.subject(subject, path).map(|entity| {
            let held = self.world.get(entity).is_some_and(|data| {
                data.components
                    .contains_key(&part_key(component, Some(&name)))
            });
            if held {
                Value::Reference(self.blackboard.parts.intern(PartRef { entity, kind, name }))
            } else {
                Value::Null
            }
        }))
    }

    /// When `subject` is a part, the entity it belongs to and the path it
    /// reaches there (`radius` becomes `light2d.radius`), with the part's name
    /// to read or write it under.
    pub(super) fn through_part(
        &self,
        subject: Option<u64>,
        path: &Path,
    ) -> Option<(u64, Path, String)> {
        let part = self.blackboard.parts.get(subject?)?;
        let mut reached = Vec::with_capacity(path.0.len() + 1);
        reached.push(part.kind.to_owned());
        reached.extend(path.0.iter().cloned());
        Some((part.entity.to_bits(), Path(reached), part.name.clone()))
    }

    /// Runs `act` on the part `subject` names, rooted at its entity, with the
    /// part's key served; `None` when `subject` is not a part.
    pub(super) fn as_part<T>(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        act: impl FnOnce(&mut Self, Option<u64>, &Path) -> T,
    ) -> Option<T> {
        let (entity, reached, part) = self.through_part(subject, path)?;
        Some(self.serving(part, |host| act(host, Some(entity), &reached)))
    }

    /// Runs `act` with `part`'s key served, then the key before it again.
    pub(super) fn serving<T>(&mut self, part: String, act: impl FnOnce(&mut Self) -> T) -> T {
        let outer = self.part.replace(part);
        let result = act(self);
        self.part = outer;
        result
    }

    /// The key a component leaf reads and writes: the part being served, or
    /// the entity's own.
    pub(super) fn component_key(&self, component: &str) -> String {
        part_key(component, self.part.as_deref())
    }
}
