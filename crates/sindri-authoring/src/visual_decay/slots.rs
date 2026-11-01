//! Expressions as slots: literals to edit in place, names and calls to read,
//! operators to see the shape of, and anything else as its exact source.

use decay_syntax::{BinaryOp, Expr, ExprKind, UnaryOp};

use super::{Slot, Span};

pub(super) fn span(span: decay_syntax::Span) -> Span {
    [span.start, span.end]
}

/// The exact source of `span`.
pub(super) fn text(source: &str, span: decay_syntax::Span) -> String {
    source
        .get(span.start..span.end)
        .unwrap_or_default()
        .to_owned()
}

/// `expr` as a slot.
pub(super) fn slot(source: &str, expr: &Expr) -> Slot {
    let at = span(expr.span);
    match &expr.kind {
        ExprKind::Number(value) => Slot::Number {
            value: *value,
            span: at,
        },
        ExprKind::String(value) => Slot::Text {
            value: value.clone(),
            span: at,
        },
        ExprKind::Bool(value) => Slot::Bool {
            value: *value,
            span: at,
        },
        ExprKind::Null => Slot::Null { span: at },
        ExprKind::Identifier(_) | ExprKind::Member { .. } => {
            path(expr).map_or_else(|| code(source, expr), |path| Slot::Name { path, span: at })
        }
        ExprKind::Call { callee, args } => match path(callee) {
            Some(callee) => Slot::Call {
                callee,
                args: args.iter().map(|arg| slot(source, arg)).collect(),
                span: at,
            },
            None => code(source, expr),
        },
        ExprKind::Binary { left, op, right } => Slot::Op {
            op: binary(*op),
            left: Box::new(slot(source, left)),
            right: Box::new(slot(source, right)),
            span: at,
        },
        // `-1.0` is one number to a person, though the parser reads a
        // negation of one.
        ExprKind::Unary {
            op: UnaryOp::Negate,
            expr: inner,
        } if matches!(inner.kind, ExprKind::Number(_)) => match inner.kind {
            ExprKind::Number(value) => Slot::Number {
                value: -value,
                span: at,
            },
            _ => code(source, expr),
        },
        ExprKind::Unary { op, expr: inner } => Slot::Unary {
            op: match op {
                UnaryOp::Negate => "-",
                UnaryOp::Not => "!",
            },
            operand: Box::new(slot(source, inner)),
            span: at,
        },
        ExprKind::Group(inner) => match slot(source, inner) {
            // The brackets belong to the slot, so editing it keeps them.
            Slot::Op {
                op, left, right, ..
            } => Slot::Op {
                op,
                left,
                right,
                span: at,
            },
            _ => code(source, expr),
        },
        _ => code(source, expr),
    }
}

fn code(source: &str, expr: &Expr) -> Slot {
    Slot::Code {
        source: text(source, expr.span),
        span: span(expr.span),
    }
}

/// `this.speed`, `Game.lit`, `World.find`: a dotted path of names.
pub(super) fn path(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Identifier(name) => Some(name.clone()),
        ExprKind::Member { object, field } => Some(format!("{}.{field}", path(object)?)),
        _ => None,
    }
}

const fn binary(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Modulo => "%",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::Fallback => "??",
    }
}
