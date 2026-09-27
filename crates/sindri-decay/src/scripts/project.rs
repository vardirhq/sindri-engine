//! Every script in a project, as a type every other script can name.
//!
//! A script used to be able to say something about another entity only by
//! agreeing on a string with it: `World.property_number(hit, "damage", 0.0)`
//! reads a number nobody checks exists, and reads the value the scene authored
//! rather than the one the other script is holding now. Here each `script`
//! declaration becomes a described type — its fields with their types, its
//! functions as messages — and a global of the same name that finds one on an
//! entity, so `Bolt.on(hit).damage` is checked when it is compiled and reads the
//! live field when it runs.
//!
//! Only declarations are read, never bodies, so a type changes when a field or
//! a signature does and not when a line of logic does; [`Project::key`] is that
//! shape, and what tells a cached program it was compiled against another one.
//!
//! Events are read the same way: an `event` declared in any file is one every
//! file may emit and handle. So is `state`: `state Game { var score: f32 =
//! 0.0; }` in any file is `Game.score` in every file.

use std::collections::{BTreeMap, BTreeSet};

use decay_semantic::{Environment, ExternalSymbol, FunctionType, HostType, Type};
use decay_syntax::{ExprKind, Item, Member, parse};

use super::sources::{START, UPDATE};

/// The function a script's type offers for finding one: `Bolt.on(entity)`.
pub(crate) const ON: &str = "on";

/// What a project's scripts declare, as the analyzer needs it.
#[derive(Clone, Debug, Default)]
pub struct Project {
    /// Each script by name, with its fields and messages.
    pub(crate) scripts: BTreeMap<String, Declared>,
    /// Each event by name, with what it carries.
    pub(crate) events: BTreeMap<String, Vec<Type>>,
    /// Events declared more than once, which nothing may use.
    ambiguous_events: BTreeSet<String>,
    /// Shared state by name, then field.
    pub(crate) states: BTreeMap<String, BTreeMap<String, SharedField>>,
    /// State fields declared more than once, as `(state, field)`.
    ambiguous_fields: BTreeSet<(String, String)>,
    key: String,
}

/// One field of a `state`: its type, whether scripts may change it, and the
/// number it starts as.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SharedField {
    pub ty: Type,
    pub mutable: bool,
    /// Held as a number, as the board holds everything; `true` is `1.0`.
    pub initial: f64,
}

impl SharedField {
    /// A number from the board as the value this field holds.
    pub(crate) fn value(&self, number: f64) -> decay_runtime::Value {
        match self.ty {
            Type::Bool => decay_runtime::Value::Bool(number != 0.0),
            _ => decay_runtime::Value::Number(number),
        }
    }
}

/// Where a state field lives on the board: `Game.score` under `score`, so a
/// script still calling `Game.get("score", 0.0)` reads the same number while a
/// project moves over, and any other state's under its full name.
pub(crate) fn board_key(state: &str, field: &str) -> String {
    if state == GAME {
        field.to_owned()
    } else {
        format!("{state}.{field}")
    }
}

/// The state that shares the board's own namespace.
pub(crate) const GAME: &str = "Game";

/// One script's declared shape.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Declared {
    pub fields: Vec<(String, Type)>,
    pub messages: Vec<(String, Vec<Type>)>,
    /// The fields marked `@export`, which a scene may author.
    pub exported: Vec<String>,
}

impl Project {
    /// Reads every source's declarations.
    ///
    /// A name declared twice is left out: which one `Bolt` meant would be a
    /// guess, and a guess that compiles is worse than a name that does not. So
    /// is a name the engine already uses, which a script type must never
    /// shadow.
    pub fn read<'a>(sources: impl IntoIterator<Item = &'a str>, reserved: &Environment) -> Self {
        let mut scripts: BTreeMap<String, Declared> = BTreeMap::new();
        let mut twice = BTreeSet::new();
        let mut events = BTreeMap::new();
        let mut ambiguous_events = BTreeSet::new();
        let mut states: BTreeMap<String, BTreeMap<String, SharedField>> = BTreeMap::new();
        let mut ambiguous_fields = BTreeSet::new();
        for source in sources {
            for item in parse(source).program.items {
                let container = match item {
                    Item::Script(container) => container,
                    Item::Event(event) => {
                        let params: Vec<Type> = event
                            .params
                            .iter()
                            .map(|param| param.ty.as_ref().map_or(Type::Unknown, Type::from_ref))
                            .collect();
                        if events.insert(event.name.clone(), params).is_some() {
                            ambiguous_events.insert(event.name);
                        }
                        continue;
                    }
                    Item::State(state) => {
                        read_state(&state, &mut states, &mut ambiguous_fields);
                        continue;
                    }
                    Item::Component(_) => continue,
                };
                if scripts
                    .insert(container.name.clone(), Declared::of(&container))
                    .is_some()
                {
                    twice.insert(container.name.clone());
                }
            }
        }
        scripts.retain(|name, _| {
            !twice.contains(name)
                && reserved.get_type(name).is_none()
                && !reserved.globals().any(|(global, _)| global == name)
        });
        // An event may not take a name a script or the engine already has;
        // the file declaring it is told so by the analyzer.
        events.retain(|name, _| {
            !ambiguous_events.contains(name)
                && !scripts.contains_key(name)
                && reserved.get_type(name).is_none()
                && !reserved.globals().any(|(global, _)| global == name)
        });
        keep_states(&mut states, &ambiguous_fields, &scripts, &events, reserved);
        let key =
            format!("{scripts:?}{events:?}{ambiguous_events:?}{states:?}{ambiguous_fields:?}");
        Self {
            scripts,
            events,
            ambiguous_events,
            states,
            ambiguous_fields,
            key,
        }
    }

    /// The declared shape of every script, as one comparable value.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Adds every script type to an environment.
    ///
    /// A script is also the entity it runs on, so its type starts as an
    /// entity's — `bolt.transform.position` — and goes wherever an `Entity`
    /// does. Where one of its own names is also an entity's, the entity's wins,
    /// as it does when the path is answered.
    pub fn describe(&self, environment: &mut Environment) {
        let entity = crate::surface::ENTITY;
        let base = environment.get_type(entity).cloned().unwrap_or_default();
        for (name, declared) in &self.scripts {
            let mut ty = HostType::new();
            for (member, symbol) in base.members() {
                ty = match symbol {
                    ExternalSymbol::Value(value) => ty.with_value(member, value.clone()),
                    ExternalSymbol::Function(function) => {
                        ty.with_function(member, function.clone())
                    }
                };
            }
            for (field, field_type) in &declared.fields {
                if ty.member(field).is_none() {
                    ty = ty.with_value(field.clone(), field_type.clone());
                }
            }
            for (message, params) in &declared.messages {
                if ty.member(message).is_some() {
                    continue;
                }
                ty = ty.with_function(
                    message.clone(),
                    FunctionType {
                        params: params.clone(),
                        return_type: Type::Unit,
                    },
                );
            }
            environment.add_type(name.clone(), ty);
            environment.add_supertype(name.clone(), entity);
            let finder = finder_type(name);
            environment.add_type(
                finder.clone(),
                HostType::new().with_function(
                    ON,
                    FunctionType {
                        params: vec![Type::Named(crate::surface::ENTITY.to_owned())],
                        return_type: Type::Named(name.clone()),
                    },
                ),
            );
            environment.add_value(name.clone(), Type::Named(finder));
        }
        for (name, params) in &self.events {
            environment.add_event(name.clone(), params.clone());
        }
        for name in &self.ambiguous_events {
            environment.add_ambiguous_event(name.clone());
        }
        for (state, fields) in &self.states {
            for (field, declared) in fields {
                environment.add_state_field(
                    state.clone(),
                    field.clone(),
                    decay_semantic::StateField {
                        ty: declared.ty.clone(),
                        mutable: declared.mutable,
                    },
                );
            }
        }
        for (state, field) in &self.ambiguous_fields {
            environment.add_ambiguous_state_field(state.clone(), field.clone());
        }
    }
}

/// The type of the global a script is found through, `Bolt` in `Bolt.on(x)`,
/// as a diagnostic names it.
pub(crate) fn finder_type(script: &str) -> String {
    format!("script {script}")
}

impl Declared {
    /// What one script declares: its fields, and its functions as messages.
    fn of(container: &decay_syntax::ContainerDecl) -> Self {
        let mut declared = Self::default();
        for member in &container.members {
            match member {
                Member::Field(field) => {
                    let ty = field
                        .ty
                        .as_ref()
                        .map_or_else(|| literal_type(field.initializer.as_ref()), Type::from_ref);
                    declared.fields.push((field.name.clone(), ty));
                    if field
                        .attributes
                        .iter()
                        .any(|attribute| attribute.name == "export")
                    {
                        declared.exported.push(field.name.clone());
                    }
                }
                // A handler is the host's to call, like `start`, not a message.
                Member::Function(function)
                    if function.name != START
                        && function.name != UPDATE
                        && function.handles.is_none() =>
                {
                    let params = function
                        .params
                        .iter()
                        .map(|param| param.ty.as_ref().map_or(Type::Unknown, Type::from_ref))
                        .collect();
                    declared.messages.push((function.name.clone(), params));
                }
                Member::Function(_) => {}
            }
        }
        declared
    }
}

/// Adds one `state` declaration's fields, noting any declared before.
fn read_state(
    state: &decay_syntax::StateDecl,
    states: &mut BTreeMap<String, BTreeMap<String, SharedField>>,
    ambiguous: &mut BTreeSet<(String, String)>,
) {
    for field in &state.fields {
        if states
            .entry(state.name.clone())
            .or_default()
            .insert(field.name.clone(), shared_field(field))
            .is_some()
        {
            ambiguous.insert((state.name.clone(), field.name.clone()));
        }
    }
}

/// Leaves out what no script may use: a state named like a script, an event,
/// or an engine name that is not a namespace, and a field declared twice.
///
/// A state may add to a namespace the engine offers, as `Game` adds to the
/// board; the file declaring it is told by the analyzer when it may not.
fn keep_states(
    states: &mut BTreeMap<String, BTreeMap<String, SharedField>>,
    ambiguous: &BTreeSet<(String, String)>,
    scripts: &BTreeMap<String, Declared>,
    events: &BTreeMap<String, Vec<Type>>,
    reserved: &Environment,
) {
    states.retain(|name, _| {
        !scripts.contains_key(name)
            && !events.contains_key(name)
            && reserved.globals().all(|(global, symbol)| {
                global != name
                    || matches!(symbol, decay_semantic::ExternalSymbol::Value(Type::Named(ty)) if ty == name)
            })
    });
    for (state, field) in ambiguous {
        if let Some(fields) = states.get_mut(state) {
            fields.remove(field);
        }
    }
}

/// A state field as the host keeps it. The analyzer refuses anything but a
/// number or a flag starting from a literal, so what it cannot read here is a
/// declaration that will not compile anyway.
fn shared_field(field: &decay_syntax::FieldDecl) -> SharedField {
    let ty = field
        .ty
        .as_ref()
        .map_or_else(|| literal_type(field.initializer.as_ref()), Type::from_ref);
    let initial = match field.initializer.as_ref().map(|expr| &expr.kind) {
        Some(ExprKind::Number(number)) => *number,
        Some(ExprKind::Unary { expr, .. }) => match expr.kind {
            ExprKind::Number(number) => -number,
            _ => 0.0,
        },
        Some(ExprKind::Bool(true)) => 1.0,
        _ => 0.0,
    };
    SharedField {
        ty,
        mutable: field.mutable,
        initial,
    }
}

/// A field's type when it is not written, from a literal initializer.
///
/// Only a literal: anything else needs the analyzer, which reads bodies, and a
/// field whose type is left unknown here is accepted wherever it is used rather
/// than refused somewhere it is right.
fn literal_type(initializer: Option<&decay_syntax::Expr>) -> Type {
    match initializer.map(|expr| &expr.kind) {
        Some(ExprKind::Number(_)) => Type::F32,
        Some(ExprKind::Bool(_)) => Type::Bool,
        Some(ExprKind::String(_)) => Type::String,
        Some(ExprKind::Unary { expr, .. }) if matches!(expr.kind, ExprKind::Number(_)) => Type::F32,
        Some(ExprKind::Call { callee, .. }) => match &callee.kind {
            ExprKind::Identifier(name) if name == "Vec2" => Type::Vec2,
            ExprKind::Identifier(name) if name == "Vec3" => Type::Vec3,
            _ => Type::Unknown,
        },
        _ => Type::Unknown,
    }
}

#[cfg(test)]
mod tests;
