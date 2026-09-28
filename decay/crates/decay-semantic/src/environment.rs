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
    /// Shared values, by state name and then field name. A program's own
    /// `state` declarations are added to these when it is analysed.
    pub(crate) states: HashMap<String, HashMap<String, StateField>>,
    /// State fields declared more than once, as `(state, field)`.
    pub(crate) ambiguous_state_fields: HashSet<(String, String)>,
    /// Which global functions a project's scripts declared, as opposed to the
    /// host's own: a file may declare one of these itself, but never one of
    /// the host's.
    pub(crate) shared_functions: HashSet<String>,
    /// Shared functions declared more than once, which no call may name.
    pub(crate) ambiguous_functions: HashSet<String>,
    /// Shared constants, worked out by the host from every file's
    /// declarations, by name.
    pub(crate) constants: HashMap<String, crate::constant::ConstValue>,
    /// Shared constants declared more than once, which nothing may use.
    pub(crate) ambiguous_constants: HashSet<String>,
    /// Enums declared somewhere the host can see, with their variants in
    /// order. A program's own are added to these when it is analysed.
    pub(crate) enums: HashMap<String, Vec<String>>,
    /// Enums declared more than once, which nothing may use.
    pub(crate) ambiguous_enums: HashSet<String>,
    /// Structs declared somewhere the host can see, with their fields in
    /// order. A program's own are added to these when it is analysed.
    pub(crate) structs: HashMap<String, Vec<(String, Type)>>,
    /// Each struct's methods, by struct and then method name.
    pub(crate) struct_methods: HashMap<String, HashMap<String, FunctionType>>,
    /// Structs declared more than once, which nothing may use.
    pub(crate) ambiguous_structs: HashSet<String>,
}

/// One field of a `state`: what it holds, and whether scripts may change it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateField {
    pub ty: Type,
    pub mutable: bool,
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

    /// Declares a function a project's script declared outside any container,
    /// which every script may call by name.
    pub fn add_shared_function(&mut self, name: impl Into<String>, function: FunctionType) {
        let name = name.into();
        self.shared_functions.insert(name.clone());
        self.add_function(name, function);
    }

    /// Says a shared function was declared more than once, so that calling
    /// it is refused rather than resolved to one of them.
    pub fn add_ambiguous_function(&mut self, name: impl Into<String>) {
        self.ambiguous_functions.insert(name.into());
    }

    /// Declares an enum every script may name, with its variants in order.
    /// A `shared const` another file declared, with its worked-out value.
    pub fn add_constant(&mut self, name: impl Into<String>, value: crate::constant::ConstValue) {
        self.constants.insert(name.into(), value);
    }

    /// A shared constant declared in more than one file.
    pub fn add_ambiguous_constant(&mut self, name: impl Into<String>) {
        self.ambiguous_constants.insert(name.into());
    }

    /// Every shared constant this environment knows, with its value.
    pub fn constants(&self) -> impl Iterator<Item = (&str, &crate::constant::ConstValue)> {
        self.constants
            .iter()
            .map(|(name, value)| (name.as_str(), value))
    }

    pub fn add_enum(&mut self, name: impl Into<String>, variants: Vec<String>) {
        self.enums.insert(name.into(), variants);
    }

    /// Says an enum was declared more than once, so that using it is refused
    /// rather than resolved to one of them.
    pub fn add_ambiguous_enum(&mut self, name: impl Into<String>) {
        let name = name.into();
        self.enums.remove(&name);
        self.ambiguous_enums.insert(name);
    }

    /// Declares a struct every script may name, with its fields in order.
    pub fn add_struct(&mut self, name: impl Into<String>, fields: Vec<(String, Type)>) {
        self.structs.insert(name.into(), fields);
    }

    /// A method another file declared on a struct: `card.heavier(best)`.
    pub fn add_struct_method(
        &mut self,
        structure: impl Into<String>,
        method: impl Into<String>,
        signature: FunctionType,
    ) {
        self.struct_methods
            .entry(structure.into())
            .or_default()
            .insert(method.into(), signature);
    }

    /// Says a struct was declared more than once, so that using it is refused
    /// rather than resolved to one of them.
    pub fn add_ambiguous_struct(&mut self, name: impl Into<String>) {
        let name = name.into();
        self.structs.remove(&name);
        self.ambiguous_structs.insert(name);
    }

    /// Every declared struct with its fields, for a tool offering them.
    pub fn structs(&self) -> impl Iterator<Item = (&str, &[(String, Type)])> {
        self.structs
            .iter()
            .map(|(name, fields)| (name.as_str(), fields.as_slice()))
    }

    /// Every declared enum with its variants, for a tool offering them.
    pub fn enums(&self) -> impl Iterator<Item = (&str, &[String])> {
        self.enums
            .iter()
            .map(|(name, variants)| (name.as_str(), variants.as_slice()))
    }

    /// An enum's variants in order, when it is declared exactly once.
    #[must_use]
    pub fn enum_variants(&self, name: &str) -> Option<&[String]> {
        self.enums.get(name).map(Vec::as_slice)
    }

    /// Declares a field of a shared state: `Game.score`.
    pub fn add_state_field(
        &mut self,
        state: impl Into<String>,
        field: impl Into<String>,
        declared: StateField,
    ) {
        self.states
            .entry(state.into())
            .or_default()
            .insert(field.into(), declared);
    }

    /// Says a state field was declared more than once, so that using it is
    /// refused rather than resolved to one of the declarations.
    pub fn add_ambiguous_state_field(
        &mut self,
        state: impl Into<String>,
        field: impl Into<String>,
    ) {
        let (state, field) = (state.into(), field.into());
        if let Some(fields) = self.states.get_mut(&state) {
            fields.remove(&field);
        }
        self.ambiguous_state_fields.insert((state, field));
    }

    /// A state's declared fields, for a host describing itself or a tool
    /// offering completions.
    pub fn state_fields(&self, state: &str) -> impl Iterator<Item = (&str, &StateField)> {
        self.states
            .get(state)
            .into_iter()
            .flatten()
            .map(|(name, field)| (name.as_str(), field))
    }

    /// Every declared state's name.
    pub fn states(&self) -> impl Iterator<Item = &str> {
        self.states.keys().map(String::as_str)
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
