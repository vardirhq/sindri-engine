//! The walk over a parsed program, and the scope it keeps while walking.
//!
//! The traversal is here; each kind of thing it walks into has a file of
//! its own. A new statement form, expression form, or member rule is an
//! arm in the matching leaf, not a change to this file.

mod call;
mod enumeration;
mod event;
mod expr;
mod function;
mod item;
mod list;
mod member;
mod state;
mod stmt;
mod structure;
mod text;
mod timer;
mod vector;

use std::collections::{HashMap, HashSet};

use decay_syntax::{Item, Program, Span, TypeRef};

use crate::diagnostic::{Diagnostic, DiagnosticPhase, ValueMembers, line_column};
use crate::environment::{Environment, ExternalSymbol};
use crate::types::{FunctionType, Type};

/// What asking a type for a member found.
///
/// The distinction that matters is between a type the host described and one it
/// did not: only the former can say a member is missing.
pub(super) enum MemberLookup {
    Found(ExternalSymbol),
    Missing,
    /// A function the container itself declares, reached through `this`, which
    /// is never what the author meant -- see [`Analyzer::member_symbol`].
    ContainerFunction,
}

#[derive(Debug, Clone)]
pub(super) struct Symbol {
    ty: Type,
    mutable: bool,
    function: Option<FunctionType>,
}

pub(super) struct Analyzer<'a, 'd> {
    source: &'a str,
    environment: &'a Environment,
    diagnostics: &'d mut Vec<Diagnostic>,
    /// Where a member read resolved to something the language owns rather than
    /// to a path the host answers. Written here, read by the lowering.
    ///
    value_members: &'d mut ValueMembers,
    scopes: Vec<HashMap<String, Symbol>>,
    current_return: Type,
    /// Every `script` and `component` in the program, so that `this` can be
    /// told apart from a host type of the same shape.
    containers: HashSet<String>,
    /// How many `while` bodies enclose the statement being analyzed, so that a
    /// `break` or `continue` with nothing to break out of is refused here
    /// rather than lowered into a jump with no target.
    loop_depth: usize,
    /// Every event this program may emit or handle: the host's, and its own.
    events: HashMap<String, Vec<Type>>,
    /// Every shared state this program may reach: the host's, and its own.
    states: HashMap<String, HashMap<String, crate::environment::StateField>>,
    /// Every enum this program may name, with its variants in order: the
    /// host's, and its own.
    enums: HashMap<String, Vec<String>>,
    /// Every struct this program may name, with its fields in order: the
    /// host's, and its own.
    structs: HashMap<String, Vec<(String, Type)>>,
    /// This file's own shared functions, by name.
    functions: HashMap<String, FunctionType>,
    /// Whether the function being analysed is a shared one, which has no
    /// `this` to reach for.
    in_shared_function: bool,
}

#[allow(clippy::zero_sized_map_values)]
impl<'a, 'd> Analyzer<'a, 'd> {
    pub(super) fn new(
        source: &'a str,
        environment: &'a Environment,
        diagnostics: &'d mut Vec<Diagnostic>,
        value_members: &'d mut ValueMembers,
    ) -> Self {
        Self {
            source,
            environment,
            diagnostics,
            value_members,
            scopes: Vec::new(),
            current_return: Type::Unit,
            containers: HashSet::new(),
            loop_depth: 0,
            events: HashMap::new(),
            states: HashMap::new(),
            functions: HashMap::new(),
            enums: HashMap::new(),
            structs: HashMap::new(),
            in_shared_function: false,
        }
    }

    pub(super) fn analyze_program(&mut self, program: &Program) {
        let mut containers = HashMap::<String, Span>::new();

        // Collected up front: a function body may name `this`, and `this` can
        // only be resolved once it is known which names are containers.
        for item in &program.items {
            if let Item::Script(container) | Item::Component(container) = item {
                self.containers.insert(container.name.clone());
            }
        }
        // And enums and events too, since a field or a handler anywhere may
        // name one declared further down.
        self.collect_enums(program);
        self.collect_structs(program);
        self.collect_events(program);
        self.collect_states(program);
        self.collect_functions(program);

        for item in &program.items {
            let container = match item {
                Item::Script(container) | Item::Component(container) => container,
                Item::Event(_) | Item::State(_) | Item::Enum(_) | Item::Struct(_) => continue,
                Item::Function(function) => {
                    self.analyze_shared_function(function);
                    continue;
                }
            };

            if containers
                .insert(container.name.clone(), container.span)
                .is_some()
            {
                self.error(
                    container.span,
                    format!("duplicate declaration `{}`", container.name),
                );
            }

            self.analyze_container(container);
        }
    }

    pub(super) fn resolve_identifier(&mut self, name: &str, span: Span) -> Type {
        if self.in_shared_function && name == "this" && self.lookup(name).is_none() {
            self.error(
                span,
                "a function outside any script has no `this` -- pass what it needs as a parameter"
                    .to_owned(),
            );
            return Type::Unknown;
        }
        if let Some(symbol) = self.lookup(name) {
            if symbol.function.is_some() {
                return Type::Unknown;
            }
            return symbol.ty.clone();
        }

        if let Some(event) = self.event_name_type(name, span) {
            return event;
        }

        if let Some(enumeration) = self.enum_name_type(name, span) {
            return enumeration;
        }

        if let Some(symbol) = self.environment.globals.get(name) {
            return match symbol {
                ExternalSymbol::Value(ty) => ty.clone(),
                ExternalSymbol::Function(_) => Type::Unknown,
            };
        }

        if let Some(state) = self.state_name_type(name) {
            return state;
        }

        self.error(span, format!("unknown name `{name}`"));
        Type::Unknown
    }

    pub(super) fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    /// The type a written annotation means, reporting a malformed one.
    ///
    /// `Type::from_ref` answers for any `TypeRef` because the IR and the host
    /// both need it to; this is where a *script* writing one is told it wrote
    /// it wrong. `Array` is the only type that takes an argument and the only
    /// one that needs one, so both halves of that are checked here rather than
    /// discovered as an `unknown` element type three errors later.
    pub(super) fn resolve_type(&mut self, reference: &TypeRef) -> Type {
        let is_array = crate::types::is_list(&reference.name);
        match (&reference.argument, is_array) {
            (None, true) => self.error(
                reference.span,
                format!(
                    "`{}` needs an element type, as in `{}<Entity>`",
                    reference.name,
                    crate::types::LIST
                ),
            ),
            (Some(_), false) => self.error(
                reference.span,
                format!(
                    "`{}` takes no type argument; only `{}` does",
                    reference.name,
                    crate::types::LIST
                ),
            ),
            _ => {}
        }
        if let Some(argument) = &reference.argument {
            self.resolve_type(argument);
        }
        Type::from_ref(reference)
    }

    pub(super) fn require_type(&mut self, actual: &Type, expected: &Type, span: Span) {
        if !self.compatible(expected, actual) {
            self.error(
                span,
                format!(
                    "expected `{}`, found `{}`",
                    expected.display_name(),
                    actual.display_name()
                ),
            );
        }
    }

    pub(super) fn check_assignable(&mut self, expected: &Type, actual: &Type, span: Span) {
        if !self.compatible(expected, actual) {
            self.error(
                span,
                format!(
                    "cannot assign `{}` to `{}`",
                    actual.display_name(),
                    expected.display_name()
                ),
            );
        }
    }

    pub(super) fn compatible(&self, expected: &Type, actual: &Type) -> bool {
        matches!(expected, Type::Unknown)
            || matches!(actual, Type::Unknown)
            || expected == actual
            // An empty list, `[]`, fits any list; a list of `Bolt`s fits a
            // list of `Entity`. A list is a value, copied where it goes, so
            // adding an `Entity` to the copy cannot reach the `Bolt`s.
            || matches!((expected, actual), (Type::Array(wanted), Type::Array(given))
                if self.compatible(wanted, given))
            || matches!((expected, actual), (Type::Named(_), Type::Null))
            || matches!((expected, actual), (Type::Named(wanted), Type::Named(given))
                if self.environment.is_a(given, wanted))
    }

    pub(super) fn error(&mut self, span: Span, message: String) {
        let (line, column) = line_column(self.source, span.start);
        self.diagnostics.push(Diagnostic {
            phase: DiagnosticPhase::Semantic,
            message,
            span,
            line,
            column,
        });
    }
}
