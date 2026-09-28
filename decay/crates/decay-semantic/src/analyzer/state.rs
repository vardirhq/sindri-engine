//! Shared state: values every script reads and writes by name, checked.
//!
//! `state Game { var score: f32 = 0.0; }` declares `Game.score`, a number
//! every script may reach, with one type and one starting value. What it
//! replaces is a string and a fallback at every use — `Game.get("score", 0.0)`
//! — where a misspelt name reads the fallback and nobody is told, and two
//! uses can disagree about what the fallback is.
//!
//! Where the values live is the host's decision. The language checks the
//! declarations and every use, and lowers `Game.score` to the path the host
//! answers, as it does for any host value.

use std::collections::HashSet;

use decay_syntax::{ExprKind, FieldDecl, Item, Program, StateDecl, UnaryOp};

use crate::environment::{ExternalSymbol, StateField};
use crate::types::Type;

use super::{Analyzer, MemberLookup};

impl Analyzer<'_, '_> {
    /// Adds this program's own state to the host's, checking each declaration.
    pub(super) fn collect_states(&mut self, program: &Program) {
        self.states = self.environment.states.clone();
        let mut own = HashSet::new();
        for item in &program.items {
            let Item::State(state) = item else {
                continue;
            };
            self.check_state_name(state);
            for field in &state.fields {
                if !own.insert((state.name.clone(), field.name.clone())) {
                    self.error(
                        field.span,
                        format!("duplicate state field `{}.{}`", state.name, field.name),
                    );
                    continue;
                }
                if self
                    .environment
                    .ambiguous_state_fields
                    .contains(&(state.name.clone(), field.name.clone()))
                {
                    self.error(
                        field.span,
                        format!(
                            "`{}.{}` is declared more than once in the project; keep one",
                            state.name, field.name
                        ),
                    );
                }
                let ty = self.check_state_field(state, field);
                self.states.entry(state.name.clone()).or_default().insert(
                    field.name.clone(),
                    StateField {
                        ty,
                        mutable: field.mutable,
                    },
                );
            }
        }
    }

    /// A state may add to a namespace the host already offers — `Game` — but
    /// not take a name that means something else, nor shadow one of that
    /// namespace's own members.
    fn check_state_name(&mut self, state: &StateDecl) {
        let name = &state.name;
        let namespace = match self.environment.globals.get(name) {
            None => None,
            Some(ExternalSymbol::Value(Type::Named(ty))) if ty == name => {
                self.environment.get_type(name)
            }
            Some(_) => {
                self.error(
                    state.span,
                    format!("`{name}` is already a name that is not a namespace"),
                );
                return;
            }
        };
        if self.is_container(name)
            || self.events.contains_key(name)
            || self.enums.contains_key(name)
        {
            self.error(
                state.span,
                format!("`{name}` is already a name; a state needs one of its own"),
            );
        }
        if let Some(namespace) = namespace {
            for field in &state.fields {
                if namespace.member(&field.name).is_some() {
                    self.error(
                        field.span,
                        format!("`{name}` already has a `{}`", field.name),
                    );
                }
            }
        }
    }

    /// A state field holds a number or a flag, and starts from a literal: the
    /// host sets it up before any script runs, so there is nothing yet to
    /// compute it from.
    fn check_state_field(&mut self, state: &StateDecl, field: &FieldDecl) -> Type {
        let literal = field
            .initializer
            .as_ref()
            .and_then(|expr| match &expr.kind {
                ExprKind::Number(_) => Some(Type::F32),
                ExprKind::Unary {
                    op: UnaryOp::Negate,
                    expr: inner,
                } if matches!(inner.kind, ExprKind::Number(_)) => Some(Type::F32),
                ExprKind::Bool(_) => Some(Type::Bool),
                // A variant, `Phase.Lobby`: its enum is the field's type.
                ExprKind::Member { .. } => self.variant_initializer_type(Some(expr)),
                _ => None,
            });
        let written = field.ty.as_ref().map(|ty| self.resolve_type(ty));
        let place = format!("{}.{}", state.name, field.name);
        let is_enum = |ty: &Type| matches!(ty, Type::Named(name) if self.enums.contains_key(name));
        if let Some(written) = &written
            && !matches!(written, Type::F32 | Type::Bool)
            && !is_enum(written)
        {
            self.error(
                field.span,
                format!(
                    "`{place}` is a `{}`; a state holds `f32`, `bool` and enums for now",
                    written.display_name()
                ),
            );
        }
        let Some(literal) = literal else {
            self.error(
                field.span,
                format!(
                    "`{place}` needs a starting value written as a number, `true`/`false`, or a variant"
                ),
            );
            return written.unwrap_or(Type::Unknown);
        };
        match written {
            Some(written) if written != literal => {
                self.error(
                    field.span,
                    format!(
                        "`{place}` is a `{}` but starts as a `{}`",
                        written.display_name(),
                        literal.display_name()
                    ),
                );
                written
            }
            _ => literal,
        }
    }

    /// The type a state's name has in an expression, when `name` is one the
    /// host has not already put in scope.
    pub(super) fn state_name_type(&self, name: &str) -> Option<Type> {
        self.states
            .contains_key(name)
            .then(|| Type::Named(name.to_owned()))
    }

    /// `Game.score`, when `Game` is a state: its field, or — for a state that
    /// only adds to a namespace — nothing, so the namespace answers.
    pub(super) fn state_member(&self, state: &str, field: &str) -> Option<MemberLookup> {
        if self.is_ambiguous_state_field(state, field) {
            // Reported by `check_ambiguous_use`; the type is left open so the
            // one mistake is not reported again by whatever uses it.
            return Some(MemberLookup::Found(ExternalSymbol::Value(Type::Unknown)));
        }
        let fields = self.states.get(state)?;
        match fields.get(field) {
            Some(declared) => Some(MemberLookup::Found(ExternalSymbol::Value(
                declared.ty.clone(),
            ))),
            None if self.environment.get_type(state).is_none() => Some(MemberLookup::Missing),
            None => None,
        }
    }

    fn is_ambiguous_state_field(&self, state: &str, field: &str) -> bool {
        self.environment
            .ambiguous_state_fields
            .contains(&(state.to_owned(), field.to_owned()))
    }

    /// Refuses a use of a state field the project declares twice, where it
    /// is used: the file declaring it may be one nothing compiles.
    pub(super) fn check_ambiguous_use(
        &mut self,
        object_type: &Type,
        field: &str,
        span: decay_syntax::Span,
    ) {
        if let Type::Named(state) = object_type
            && self.is_ambiguous_state_field(state, field)
        {
            self.error(
                span,
                format!("`{state}.{field}` is declared more than once in the project; keep one"),
            );
        }
    }

    /// Refuses `Game.lives = 3.0` when `lives` is a `let`.
    pub(super) fn check_state_assignment(
        &mut self,
        object: &decay_syntax::Expr,
        field: &str,
        span: decay_syntax::Span,
    ) {
        let ExprKind::Identifier(state) = &object.kind else {
            return;
        };
        if self.lookup(state).is_some() {
            return;
        }
        if let Some(declared) = self.states.get(state).and_then(|fields| fields.get(field))
            && !declared.mutable
        {
            self.error(
                span,
                format!("`{state}.{field}` is a `let` and cannot be changed"),
            );
        }
    }
}
