//! Colours: building one, reading and writing its channels, and blending.
//!
//! A colour is a language value like a vector: four numbers, `r`, `g`, `b`
//! and `a`, each from 0 to 1. The engine's colours — a sprite's `tint`, a
//! shape's `fill` — are this type, so one is read, held and assigned whole.

use decay_syntax::{ColorOp, Expr, ExprKind, Span, parse_hex};

use crate::codes::Code;
use crate::diagnostic::ValueMember;
use crate::types::{CHANNELS, COLOR, Type};

use super::Analyzer;

impl Analyzer<'_, '_> {
    /// `Color(r, g, b)`, `Color(r, g, b, a)` or `Color("#ff8800")`, when no
    /// script binding has taken the name.
    pub(super) fn color_construct_type(&mut self, args: &[Expr], span: Span) -> Option<Type> {
        if self.lookup(COLOR).is_some() {
            return None;
        }
        if let [text] = args {
            let ty = self.expr_type(text);
            if ty == Type::String {
                if let ExprKind::String(written) = &text.kind
                    && parse_hex(written).is_none()
                {
                    self.error(
                        Code::InvalidOperand,
                        text.span,
                        format!(
                            "`{written}` is not a colour: write `#rrggbb` or `#rrggbbaa`, as in \
                             `\"#ff8800\"`"
                        ),
                    );
                }
                self.value_members
                    .insert(span, ValueMember::Color(ColorOp::FromHex));
                return Some(Type::Color);
            }
            self.require_type(&ty, &Type::String, text.span);
            return Some(Type::Color);
        }
        if !matches!(args.len(), 3 | 4) {
            self.error(
                Code::ArgumentCount,
                span,
                format!(
                    "`Color` takes `r, g, b`, `r, g, b, a` or a hex text like `\"#ff8800\"`, \
                     found {} arguments",
                    args.len()
                ),
            );
        }
        for argument in args {
            let actual = self.expr_type(argument);
            self.require_type(&actual, &Type::F32, argument.span);
        }
        // Built as four channels; the lowering adds an opaque alpha to three.
        self.value_members.insert(span, ValueMember::Construct(4));
        Some(Type::Color)
    }

    /// `color.r`, and the rest: a channel, noting how the lowering reaches it.
    pub(super) fn color_member_type(&mut self, object: &Expr, field: &str, span: Span) -> Type {
        if let Some(index) = CHANNELS.iter().position(|channel| *channel == field) {
            if self.value_rooted(object) {
                self.value_members
                    .insert(span, ValueMember::Component(index));
            }
            return Type::F32;
        }
        if ColorOp::named(field).is_some() {
            self.error(
                Code::FunctionNotCalled,
                span,
                format!("`{field}` on a colour needs arguments -- call it: `.{field}(...)`"),
            );
        } else {
            self.error(Code::UnknownMember, span, missing_member(field));
        }
        Type::Unknown
    }

    /// `color.lerp(other, t)` or `color.with_alpha(a)`.
    pub(super) fn color_call_type(&mut self, field: &str, args: &[Expr], span: Span) -> Type {
        let Some((op, _)) = ColorOp::named(field) else {
            let message = if CHANNELS.contains(&field) {
                format!("`{field}` is a number, not a function -- write `.{field}`")
            } else {
                missing_member(field)
            };
            self.error(Code::UnknownMember, span, message);
            for argument in args {
                self.expr_type(argument);
            }
            return Type::Unknown;
        };
        let signature = crate::members::color_op_signature(op);
        self.check_call(&signature, args, span);
        self.value_members.insert(span, ValueMember::Color(op));
        signature.return_type
    }
}

fn missing_member(field: &str) -> String {
    let operations = ColorOp::ALL
        .iter()
        .map(|(_, name, _)| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("`Color` has no member `{field}`; it has `r`, `g`, `b`, `a`, {operations}")
}
