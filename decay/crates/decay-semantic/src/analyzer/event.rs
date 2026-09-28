//! Events: declaring one, emitting one, and handling one.
//!
//! An event is the language's, not the host's: `event GoalScored(team: f32);`
//! says what it carries, `GoalScored.emit(1.0)` sends one, and
//! `on GoalScored(team) { ... }` in any script receives it. What the analysis
//! checks is that all three agree — the name exists, is declared once, and the
//! values match — so a misspelled event is refused before Play rather than
//! emitted to nobody.
//!
//! The host decides when a handler runs. The language only lowers the emit as
//! a call on the path `GoalScored.emit`, and the handler as a function named
//! by [`decay_syntax::handler_name`].

use crate::codes::Code;
use decay_syntax::{EventDecl, FunctionDecl, Item, Program, Span};

use crate::environment::ExternalSymbol;
use crate::types::{EMIT, FunctionType, Type, event_type};

use super::{Analyzer, MemberLookup};

impl Analyzer<'_, '_> {
    /// Adds this program's own events to the ones the host declared, checking
    /// each declaration.
    pub(super) fn collect_events(&mut self, program: &Program) {
        self.events = self.environment.events.clone();
        let mut own = std::collections::HashSet::new();
        for item in &program.items {
            let Item::Event(event) = item else {
                continue;
            };
            if !own.insert(event.name.clone()) {
                self.error(
                    Code::Duplicate,
                    event.span,
                    format!("duplicate declaration `{}`", event.name),
                );
                continue;
            }
            self.check_event(event);
            let params = event
                .params
                .iter()
                .map(|param| param.ty.as_ref().map_or(Type::Unknown, Type::from_ref))
                .collect();
            self.events.insert(event.name.clone(), params);
        }
    }

    fn check_event(&mut self, event: &EventDecl) {
        let name = &event.name;
        if self.environment.ambiguous_events.contains(name) {
            self.error(
                Code::DeclaredInSeveralFiles,
                event.span,
                format!("event `{name}` is declared in more than one file; keep one"),
            );
        }
        if self.is_container(name)
            || self.environment.globals.contains_key(name)
            || self.environment.get_type(name).is_some()
        {
            self.error(
                Code::NameTaken,
                event.span,
                format!("`{name}` is already a name; an event needs one of its own"),
            );
        }
        for param in &event.params {
            match &param.ty {
                Some(ty) => {
                    self.resolve_type(ty);
                }
                None => self.error(
                    Code::MissingType,
                    param.span,
                    format!(
                        "event parameter `{}` needs a type: every handler relies on it",
                        param.name
                    ),
                ),
            }
        }
    }

    /// The type an event's name has in an expression, when `name` is one.
    pub(super) fn event_name_type(&mut self, name: &str, span: Span) -> Option<Type> {
        if self.events.contains_key(name) {
            return Some(Type::Named(event_type(name)));
        }
        if self.environment.ambiguous_events.contains(name) {
            self.error(
                Code::DeclaredInSeveralFiles,
                span,
                format!("event `{name}` is declared in more than one file; keep one"),
            );
            return Some(Type::Unknown);
        }
        None
    }

    /// `GoalScored.emit`, the one member an event has.
    pub(super) fn event_member(&self, type_name: &str, field: &str) -> Option<MemberLookup> {
        let event = type_name.strip_prefix("event ")?;
        let params = self.events.get(event)?;
        Some(if field == EMIT {
            MemberLookup::Found(ExternalSymbol::Function(FunctionType {
                params: params.clone(),
                return_type: Type::Unit,
            }))
        } else {
            MemberLookup::Missing
        })
    }

    /// The parameter types a handler takes: the event's, where the handler
    /// leaves one unwritten, and checked against it where it writes one.
    ///
    /// `None` for an ordinary function.
    pub(super) fn handler_params(&mut self, function: &FunctionDecl) -> Option<Vec<Type>> {
        let (event, span) = function.handles.as_ref()?;
        let Some(expected) = self.events.get(event).cloned() else {
            if self.environment.ambiguous_events.contains(event) {
                self.error(
                    Code::DeclaredInSeveralFiles,
                    *span,
                    format!("event `{event}` is declared in more than one file; keep one"),
                );
            } else {
                self.error(
                    Code::UnknownEvent,
                    *span,
                    format!("unknown event `{event}`"),
                );
            }
            return Some(vec![Type::Unknown; function.params.len()]);
        };
        if expected.len() != function.params.len() {
            self.error(
                Code::HandlerSignature,
                *span,
                format!(
                    "`{event}` carries {} value(s), and this handler takes {}",
                    expected.len(),
                    function.params.len()
                ),
            );
        }
        let mut params = Vec::with_capacity(function.params.len());
        for (index, param) in function.params.iter().enumerate() {
            let wanted = expected.get(index).cloned().unwrap_or(Type::Unknown);
            let Some(written) = param.ty.as_ref().map(Type::from_ref) else {
                params.push(wanted);
                continue;
            };
            if !self.compatible(&written, &wanted) {
                self.error(
                    Code::HandlerSignature,
                    param.span,
                    format!(
                        "`{event}` carries `{}` here, not `{}`",
                        wanted.display_name(),
                        written.display_name()
                    ),
                );
            }
            params.push(written);
        }
        Some(params)
    }
}
