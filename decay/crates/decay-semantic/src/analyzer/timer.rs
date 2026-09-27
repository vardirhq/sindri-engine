//! Timers: starting one, and asking it how it is doing.
//!
//! A timer is a language value, like a vector: `Timer(2.4)` starts one, and
//! `.done`, `.left`, `.duration` and `.progress` read it. When time passes is
//! the host's decision — the language never sees a clock — so what the
//! analysis adds on top of the type is a note for the lowering, as it does
//! for a vector.

use decay_syntax::{Expr, Span, TimerProperty};

use crate::diagnostic::ValueMember;
use crate::types::{TIMER, Type};

use super::Analyzer;

impl Analyzer<'_, '_> {
    /// `Timer(seconds)`, when no script binding has taken the name.
    pub(super) fn start_timer_type(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Option<Type> {
        if name != TIMER || self.lookup(name).is_some() {
            return None;
        }
        if args.len() != 1 {
            self.error(
                span,
                format!(
                    "`{TIMER}` takes the seconds it runs for, found {} argument(s)",
                    args.len()
                ),
            );
        }
        for argument in args {
            let actual = self.expr_type(argument);
            self.require_type(&actual, &Type::F32, argument.span);
        }
        self.value_members.insert(span, ValueMember::StartTimer);
        Some(Type::Timer)
    }

    /// The type of `timer.property`, noting it for the lowering.
    pub(super) fn timer_member_type(&mut self, field: &str, span: Span) -> Type {
        match TimerProperty::named(field) {
            Some(property) => {
                self.value_members
                    .insert(span, ValueMember::Timer(property));
                match property {
                    TimerProperty::Done => Type::Bool,
                    TimerProperty::Left | TimerProperty::Duration | TimerProperty::Progress => {
                        Type::F32
                    }
                }
            }
            None => {
                self.error(span, missing_member(field));
                Type::Unknown
            }
        }
    }

    /// `timer.done()`, which is a property, or anything else called on one.
    pub(super) fn timer_call_type(&mut self, field: &str, args: &[Expr], span: Span) -> Type {
        if TimerProperty::named(field).is_some() {
            self.error(
                span,
                format!("`{field}` is a property, not a function -- write `.{field}`"),
            );
        } else {
            self.error(span, missing_member(field));
        }
        for argument in args {
            self.expr_type(argument);
        }
        Type::Unknown
    }

    /// `timer.left = 1.0`: a timer is read, never written into.
    pub(super) fn refuse_timer_write(&mut self, field: &str, span: Span) {
        self.error(
            span,
            format!("a timer's `{field}` cannot be set -- start a new one with `{TIMER}(seconds)`"),
        );
    }
}

fn missing_member(field: &str) -> String {
    let properties = TimerProperty::ALL
        .iter()
        .map(|(_, name)| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("`{TIMER}` has no member `{field}`; it has {properties}")
}
