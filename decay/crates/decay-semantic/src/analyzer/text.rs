//! Text: joining it with `+`, and what a piece of it can be asked.
//!
//! Text is a language value, like a vector, so its operations are decided
//! here and noted in [`ValueMember`] for the lowering, rather than handed to
//! the host as a path.

use crate::codes::Code;
use decay_syntax::{BinaryOp, Expr, Span, StringOp};

use crate::diagnostic::ValueMember;
use crate::types::Type;

use super::Analyzer;

impl Analyzer<'_, '_> {
    /// The type of `text.property`, noting it for the lowering.
    pub(super) fn string_member_type(&mut self, field: &str, span: Span) -> Type {
        match StringOp::named(field) {
            Some((op, 0)) => {
                self.value_members.insert(span, ValueMember::Text(op));
                crate::members::string_op_signature(op).return_type
            }
            Some(_) => {
                self.error(
                    Code::FunctionNotCalled,
                    span,
                    format!("`{field}` on `String` needs arguments -- call it: `.{field}(...)`"),
                );
                Type::Unknown
            }
            None => {
                self.error(Code::UnknownMember, span, missing_member(field));
                Type::Unknown
            }
        }
    }

    /// The type of `text.method(args)`.
    pub(super) fn string_call_type(&mut self, field: &str, args: &[Expr], span: Span) -> Type {
        let op = match StringOp::named(field) {
            Some((op, arity)) if arity > 0 => op,
            Some(_) => {
                self.error(
                    Code::PropertyCalled,
                    span,
                    format!("`{field}` is a property, not a function -- write `.{field}`"),
                );
                self.check_text_arguments(args);
                return Type::Unknown;
            }
            None => {
                self.error(Code::UnknownMember, span, missing_member(field));
                self.check_text_arguments(args);
                return Type::Unknown;
            }
        };
        let signature = crate::members::string_op_signature(op);
        self.check_call(&signature, args, span);
        self.value_members.insert(span, ValueMember::Text(op));
        signature.return_type
    }

    fn check_text_arguments(&mut self, args: &[Expr]) {
        for argument in args {
            self.expr_type(argument);
        }
    }

    /// The type `left op right` has when either side is text, or `None` when
    /// neither is and the other rules apply.
    ///
    /// `+` joins, and the other side may be anything that has an obvious
    /// spelling — a number, a flag, a vector, a variant — so `"score " +
    /// score` needs no conversion function. Nothing else is done to text with
    /// an operator.
    pub(super) fn string_arithmetic(
        &mut self,
        left: (&Type, Span),
        op: BinaryOp,
        right: (&Type, Span),
    ) -> Option<Type> {
        let (left_type, left_span) = left;
        let (right_type, right_span) = right;
        if *left_type != Type::String && *right_type != Type::String {
            return None;
        }
        if op != BinaryOp::Add {
            let at = if *left_type == Type::String {
                left_span
            } else {
                right_span
            };
            self.error(
                Code::InvalidOperand,
                at,
                format!(
                    "text is only joined, with `+`; found `{}` {} `{}`",
                    left_type.display_name(),
                    symbol(op),
                    right_type.display_name()
                ),
            );
            return Some(Type::String);
        }
        for (ty, span) in [left, right] {
            if !self.joinable(ty) {
                self.error(Code::InvalidOperand,
                    span,
                    format!(
                        "`{}` has no spelling to join to text; text joins with text, numbers, `bool`, vectors and enums",
                        ty.display_name()
                    ),
                );
            }
        }
        Some(Type::String)
    }

    /// Whether a value of this type has one obvious way to be written out.
    fn joinable(&self, ty: &Type) -> bool {
        match ty {
            Type::String | Type::F32 | Type::Bool | Type::Vec2 | Type::Vec3 | Type::Unknown => true,
            Type::Named(name) => self.enums.contains_key(name),
            _ => false,
        }
    }
}

fn symbol(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Modulo => "%",
        _ => "?",
    }
}

fn missing_member(field: &str) -> String {
    let operations = StringOp::ALL
        .iter()
        .map(|(_, name, _)| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("`String` has no member `{field}`; it has {operations}")
}
