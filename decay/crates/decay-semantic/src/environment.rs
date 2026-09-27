//! What a host puts in scope before analysis starts.
//!
//! The language does not know about Sindri. Globals, host types, and
//! their members arrive through here, which is why this crate compiles
//! without an engine and why a second host needs no change to it.

use std::collections::{HashMap, HashSet};

use crate::types::{FunctionType, HostType, Type};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalSymbol {
    Value(Type),
    Function(FunctionType),
}

#[derive(Debug, Clone, Default)]
pub struct Environment {
    pub(crate) globals: HashMap<String, ExternalSymbol>,
    pub(crate) types: HashMap<String, HostType>,
    /// What `this` offers beyond the container's own fields.
    ///
    /// `this` is two things at once: the script's own state, and the entity the
    /// host attached it to. A container field always wins, so a script can
    /// never be shadowed by the engine growing a name.
    pub(crate) this: HostType,
    /// Named types that are also another named type: each script a host
    /// describes is also an entity, so a `Bolt` goes wherever an `Entity`
    /// does. One way only: an entity is not a `Bolt` until the host says so.
    pub(crate) supertypes: HashMap<String, String>,
    /// Events declared somewhere the host can see, by name, with what each
    /// carries. A program's own `event` declarations are added to these
    /// when it is analysed, so one file on its own still knows its events.
    pub(crate) events: HashMap<String, Vec<Type>>,
    /// Events declared more than once, which no emit or handler may name:
    /// which declaration it meant would be a guess.
    pub(crate) ambiguous_events: HashSet<String>,
}

impl Environment {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_value(&mut self, name: impl Into<String>, ty: Type) {
        self.globals.insert(name.into(), ExternalSymbol::Value(ty));
    }

    pub fn add_function(&mut self, name: impl Into<String>, function: FunctionType) {
        self.globals
            .insert(name.into(), ExternalSymbol::Function(function));
    }

    /// Describes a named type's members.
    ///
    /// A named type that is *not* described stays permissive: its members are
    /// `Unknown`, exactly as every member was before this existed. That is
    /// deliberate — describing the host is gradual, and a host part-way through
    /// describing itself must not reject scripts that were working.
    pub fn add_type(&mut self, name: impl Into<String>, ty: HostType) {
        self.types.insert(name.into(), ty);
    }

    /// Adds a member to `this`, such as the transform of the entity a script is
    /// attached to.
    pub fn add_this_value(&mut self, name: impl Into<String>, ty: Type) {
        self.this = std::mem::take(&mut self.this).with_value(name, ty);
    }

    /// Adds a callable member to `this`.
    pub fn add_this_function(&mut self, name: impl Into<String>, function: FunctionType) {
        self.this = std::mem::take(&mut self.this).with_function(name, function);
    }

    /// Says that every `name` is also a `supertype`.
    pub fn add_supertype(&mut self, name: impl Into<String>, supertype: impl Into<String>) {
        self.supertypes.insert(name.into(), supertype.into());
    }

    /// Declares an event any script may emit or handle.
    pub fn add_event(&mut self, name: impl Into<String>, params: Vec<Type>) {
        self.events.insert(name.into(), params);
    }

    /// Says an event name was declared more than once, so that using it is
    /// refused rather than resolved to one of them.
    pub fn add_ambiguous_event(&mut self, name: impl Into<String>) {
        let name = name.into();
        self.events.remove(&name);
        self.ambiguous_events.insert(name);
    }

    /// What an event carries, when it is declared exactly once.
    #[must_use]
    pub fn event(&self, name: &str) -> Option<&[Type]> {
        self.events.get(name).map(Vec::as_slice)
    }

    /// Every declared event, for a host emitting a description of itself.
    pub fn events(&self) -> impl Iterator<Item = (&str, &[Type])> {
        self.events
            .iter()
            .map(|(name, params)| (name.as_str(), params.as_slice()))
    }

    /// Whether a value of type `name` may be used as a `wanted`.
    #[must_use]
    pub fn is_a(&self, name: &str, wanted: &str) -> bool {
        let mut current = name;
        for _ in 0..8 {
            if current == wanted {
                return true;
            }
            match self.supertypes.get(current) {
                Some(next) => current = next,
                None => return false,
            }
        }
        false
    }

    #[must_use]
    pub fn get_type(&self, name: &str) -> Option<&HostType> {
        self.types.get(name)
    }

    #[must_use]
    pub const fn this(&self) -> &HostType {
        &self.this
    }

    /// Every described type, for a host emitting a description of itself.
    pub fn types(&self) -> impl Iterator<Item = (&str, &HostType)> {
        self.types.iter().map(|(name, ty)| (name.as_str(), ty))
    }

    /// Every global, for the same reason.
    pub fn globals(&self) -> impl Iterator<Item = (&str, &ExternalSymbol)> {
        self.globals
            .iter()
            .map(|(name, symbol)| (name.as_str(), symbol))
    }
}
