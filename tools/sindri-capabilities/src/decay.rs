//! The Decay host surface, as a description something else can read.
//!
//! Read from [`sindri_decay::environment`], which is the same description the
//! analyzer type-checks scripts against and the same one the runtime host
//! answers. So a call that appears here is a call that exists, and one that
//! exists appears here — there is no third list to forget to update.
//!
//! What the environment does *not* carry is prose: what a name is for, and what
//! a call's parameters are called. Those come from [`sindri_decay::reference`],
//! which is checked against the environment here so that every name on the
//! surface has a description and no description outlives its name.

use decay_semantic::{Environment, ExternalSymbol, FunctionType, Type};
use serde_json::{Value, json};
use sindri_decay::reference::{self, Entry};

/// Everything a Decay script may name.
pub(crate) struct DecayApi {
    /// Names in scope without qualification: `sin`, `print`, and each namespace
    /// value such as `Input`.
    pub(crate) globals: Vec<Symbol>,
    /// What `this` offers beyond the script's own fields.
    pub(crate) this: Vec<Symbol>,
    /// Every described host type, such as `World` or `Transform`.
    pub(crate) types: Vec<Namespace>,
}

/// A named host type and what it offers.
pub(crate) struct Namespace {
    pub(crate) name: String,
    pub(crate) description: Option<&'static str>,
    pub(crate) members: Vec<Symbol>,
}

/// One thing a script can name: a value it reads, or a function it calls.
pub(crate) enum Symbol {
    Value {
        name: String,
        type_name: String,
        reference: Option<Entry>,
    },
    Function {
        name: String,
        params: Vec<String>,
        returns: String,
        reference: Option<Entry>,
    },
}

impl Symbol {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Value { name, .. } | Self::Function { name, .. } => name,
        }
    }

    pub(crate) fn reference(&self) -> Option<Entry> {
        match self {
            Self::Value { reference, .. } | Self::Function { reference, .. } => *reference,
        }
    }

    fn to_json(&self) -> Value {
        let description = self.reference().map(|entry| entry.text);
        match self {
            Self::Value {
                name, type_name, ..
            } => json!({
                "name": name,
                "kind": "value",
                "type": type_name,
                "description": description,
            }),
            Self::Function {
                name,
                params,
                returns,
                reference,
            } => json!({
                "name": name,
                "kind": "function",
                "parameters": params,
                "parameter_names": reference.map(|entry| entry.params),
                "returns": returns,
                "description": description,
            }),
        }
    }
}

impl DecayApi {
    pub(crate) fn to_json(&self) -> Value {
        json!({
            "schema_version": crate::SCHEMA_VERSION,
            "engine_version": env!("CARGO_PKG_VERSION"),
            "generated_by": crate::REGENERATE_COMMAND,
            "about": "Every namespace, call, and member a Decay script may name, \
        derived from the host surface the analyzer and the runtime share, with a \
        description of each and the names of each call's parameters. \
        docs/scripting.md explains the design behind them.",
            "globals": symbols_json(&self.globals),
            "this": symbols_json(&self.this),
            "types": self
                .types
                .iter()
                .map(|namespace| json!({
                    "name": namespace.name,
                    "description": namespace.description,
                    "members": symbols_json(&namespace.members),
                }))
                .collect::<Vec<_>>(),
        })
    }
}

fn symbols_json(symbols: &[Symbol]) -> Vec<Value> {
    symbols.iter().map(Symbol::to_json).collect()
}

/// Describes the host surface this engine build actually offers.
pub(crate) fn describe() -> DecayApi {
    let environment = sindri_decay::environment();

    DecayApi {
        globals: sorted(
            environment
                .globals()
                .map(|(name, symbol)| symbol_of(name, symbol, global_reference(name, symbol))),
        ),
        this: sorted(
            environment.this().members().map(|(name, symbol)| {
                symbol_of(name, symbol, reference::this_entry(name).copied())
            }),
        ),
        types: sorted_types(&environment),
    }
}

fn sorted_types(environment: &Environment) -> Vec<Namespace> {
    let mut types: Vec<Namespace> = environment
        .types()
        .map(|(name, host_type)| Namespace {
            name: name.to_owned(),
            description: reference::type_entry(name).map(|entry| entry.text),
            members: sorted(host_type.members().map(|(member, symbol)| {
                symbol_of(
                    member,
                    symbol,
                    reference::member_entry(name, member).copied(),
                )
            })),
        })
        .collect();
    types.sort_by(|left, right| left.name.cmp(&right.name));
    types
}

/// A global's description: its own for a function or a number, its type's
/// for a namespace such as `World`, whose global is only the way in.
fn global_reference(name: &str, symbol: &ExternalSymbol) -> Option<Entry> {
    if let Some(entry) = reference::global_entry(name) {
        return Some(*entry);
    }
    match symbol {
        ExternalSymbol::Value(Type::Named(ty)) if ty == name => {
            reference::type_entry(name).map(|entry| Entry {
                name: entry.name,
                params: &[],
                text: entry.text,
            })
        }
        _ => None,
    }
}

/// Sorted by name, because the environment holds these in hash maps and a
/// generated file that reorders itself between runs is a file nobody can diff.
fn sorted(symbols: impl Iterator<Item = Symbol>) -> Vec<Symbol> {
    let mut symbols: Vec<Symbol> = symbols.collect();
    symbols.sort_by(|left, right| left.name().cmp(right.name()));
    symbols
}

fn symbol_of(name: &str, symbol: &ExternalSymbol, reference: Option<Entry>) -> Symbol {
    match symbol {
        ExternalSymbol::Value(ty) => Symbol::Value {
            name: name.to_owned(),
            type_name: type_name(ty),
            reference,
        },
        ExternalSymbol::Function(FunctionType {
            params,
            return_type,
        }) => Symbol::Function {
            name: name.to_owned(),
            params: params.iter().map(type_name).collect(),
            returns: type_name(return_type),
            reference,
        },
    }
}

/// How a type is written in Decay source, so a reader can copy it into a script.
pub(crate) fn type_name(ty: &Type) -> String {
    match ty {
        Type::F32 => "f32".to_owned(),
        Type::Bool => "bool".to_owned(),
        Type::String => "String".to_owned(),
        Type::Unit => "unit".to_owned(),
        Type::Null => "null".to_owned(),
        Type::Named(name) => name.clone(),
        Type::Vec2 | Type::Vec3 | Type::Timer | Type::Color => ty.display_name().into_owned(),
        Type::Array(element) => format!("List<{}>", type_name(element)),
        Type::Map(key, value) => format!("Map<{}, {}>", type_name(key), type_name(value)),
        Type::Optional(inner) => format!("{}?", type_name(inner)),
        Type::Unknown => "unknown".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Symbol, describe, type_name};
    use decay_semantic::Type;

    #[test]
    fn the_description_carries_a_call_the_host_answers() {
        let api = describe();
        let world = api
            .types
            .iter()
            .find(|namespace| namespace.name == "World")
            .expect("the world namespace is described");

        let find = world
            .members
            .iter()
            .find(|member| member.name() == "find")
            .expect("World.find is on the surface");

        match find {
            Symbol::Function { params, .. } => {
                assert_eq!(params, &["String".to_owned()], "find takes one name");
            }
            Symbol::Value { .. } => panic!("World.find is a call, not a value"),
        }
    }

    #[test]
    fn members_are_sorted_so_the_file_does_not_churn() {
        let api = describe();
        for namespace in &api.types {
            let mut names: Vec<&str> = namespace.members.iter().map(Symbol::name).collect();
            let sorted = {
                let mut copy = names.clone();
                copy.sort_unstable();
                copy
            };
            names.dedup();
            assert_eq!(namespace.members.len(), names.len(), "no duplicate members");
            assert_eq!(
                namespace
                    .members
                    .iter()
                    .map(Symbol::name)
                    .collect::<Vec<_>>(),
                sorted,
                "{} members are sorted",
                namespace.name
            );
        }
    }

    #[test]
    fn an_array_type_reads_as_decay_writes_it() {
        assert_eq!(
            type_name(&Type::Array(Box::new(Type::Named("Entity".to_owned())))),
            "List<Entity>"
        );
    }
}

#[cfg(test)]
mod reference_tests {
    use super::{Symbol, describe};
    use sindri_decay::reference;

    fn check(owner: &str, symbol: &Symbol, missing: &mut Vec<String>) {
        let Some(entry) = symbol.reference() else {
            missing.push(format!("{owner}{} has no description", symbol.name()));
            return;
        };
        if let Symbol::Function { params, .. } = symbol
            && params.len() != entry.params.len()
        {
            missing.push(format!(
                "{owner}{} takes {} parameters but names {}",
                symbol.name(),
                params.len(),
                entry.params.len()
            ));
        }
    }

    /// Every name a script can reach says what it is for, and every call names
    /// each of its parameters.
    #[test]
    fn every_name_on_the_surface_is_described() {
        let api = describe();
        let mut missing = Vec::new();
        for symbol in &api.globals {
            check("", symbol, &mut missing);
        }
        for symbol in &api.this {
            check("this.", symbol, &mut missing);
        }
        for namespace in &api.types {
            if namespace.description.is_none() {
                missing.push(format!("type {} has no description", namespace.name));
            }
            for symbol in &namespace.members {
                check(&format!("{}.", namespace.name), symbol, &mut missing);
            }
        }
        assert!(missing.is_empty(), "{}", missing.join("\n"));
    }

    /// No description outlives its name.
    #[test]
    fn every_description_names_something_on_the_surface() {
        let api = describe();
        let mut stale = Vec::new();
        for entry in reference::GLOBALS {
            if !api.globals.iter().any(|symbol| symbol.name() == entry.name) {
                stale.push(entry.name.to_owned());
            }
        }
        for entry in reference::THIS {
            if !api.this.iter().any(|symbol| symbol.name() == entry.name) {
                stale.push(format!("this.{}", entry.name));
            }
        }
        for described in reference::TYPES.iter().flat_map(|types| types.iter()) {
            let Some(namespace) = api
                .types
                .iter()
                .find(|namespace| namespace.name == described.name)
            else {
                stale.push(format!("type {}", described.name));
                continue;
            };
            for entry in described.members {
                if !namespace
                    .members
                    .iter()
                    .any(|symbol| symbol.name() == entry.name)
                {
                    stale.push(format!("{}.{}", described.name, entry.name));
                }
            }
        }
        assert!(
            stale.is_empty(),
            "described but not on the surface: {stale:?}"
        );
    }
}
