//! What a number can be asked: how to write it as text.
//!
//! A number is a language value, so its operations are decided here and noted
//! in [`ValueMember`] for the lowering, as text's and a vector's are.

use crate::codes::Code;
use decay_syntax::{Expr, NumberOp, Span};

use crate::diagnostic::ValueMember;
use crate::types::Type;

use super::Analyzer;

impl Analyzer<'_, '_> {
    /// `n.fixed` without its arguments, or a member a number does not have.
    pub(super) fn number_member_type(&mut self, field: &str, span: Span) -> Type {
        if NumberOp::named(field).is_some() {
            self.error(
                Code::FunctionNotCalled,
                span,
                format!("`{field}` on `f32` needs arguments -- call it: `.{field}(...)`"),
            );
        } else {
            self.error(Code::UnknownMember, span, missing_member(field));
        }
        Type::Unknown
    }

    /// The type of `n.fixed(2)`.
    pub(super) fn number_call_type(&mut self, field: &str, args: &[Expr], span: Span) -> Type {
        let Some((op, _)) = NumberOp::named(field) else {
            self.error(Code::UnknownMember, span, missing_member(field));
            for argument in args {
                self.expr_type(argument);
            }
            return Type::Unknown;
        };
        let signature = crate::members::number_op_signature(op);
        self.check_call(&signature, args, span);
        self.value_members.insert(span, ValueMember::Number(op));
        signature.return_type
    }
}

fn missing_member(field: &str) -> String {
    let operations = NumberOp::ALL
        .iter()
        .map(|(_, name, _)| format!("`{name}(...)`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("`f32` has no member `{field}`; it has {operations}")
}
