//! One function per expression form, and the types they produce.

use crate::codes::Code;
use decay_syntax::{AssignOp, BinaryOp, Expr, ExprKind, UnaryOp};

use crate::types::Type;

use super::Analyzer;

impl Analyzer<'_, '_> {
    pub(super) fn expr_type(&mut self, expr: &Expr) -> Type {
        match &expr.kind {
            ExprKind::Identifier(name) => self.resolve_identifier(name, expr.span),
            ExprKind::Number(_) => Type::F32,
            ExprKind::String(_) => Type::String,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::Null => Type::Null,
            ExprKind::Group(inner) => self.expr_type(inner),
            ExprKind::Unary { op, expr: inner } => {
                let inner_type = self.expr_type(inner);
                match op {
                    // A vector negates to the vector pointing the other way.
                    UnaryOp::Negate if inner_type.dimensions().is_some() => inner_type,
                    UnaryOp::Negate => {
                        self.require_type(&inner_type, &Type::F32, inner.span);
                        Type::F32
                    }
                    UnaryOp::Not => {
                        self.require_type(&inner_type, &Type::Bool, inner.span);
                        Type::Bool
                    }
                }
            }
            ExprKind::Binary { left, op, right } => self.binary_type(left, *op, right),
            ExprKind::Assign { target, op, value } => self.assignment_type(target, *op, value),
            ExprKind::Member { object, field } => self.member_type(object, field, expr.span),
            ExprKind::Call { callee, args } => self.call_type(callee, args, expr.span),
            ExprKind::Index { object, index } => self.index_type(object, index),
            ExprKind::Match { subject, arms } => self.match_value_type(subject, arms, expr.span),
            ExprKind::List(elements) => self.list_literal_type(elements),
            ExprKind::Map(entries) => self.map_literal_type(entries),
            ExprKind::Construct { name, fields } => {
                self.construct_struct_type(name, fields, expr.span)
            }
            ExprKind::Range { start, end } => self.stray_range_type(start, end, expr.span),
        }
    }

    /// The type of `items[index]`.
    ///
    /// Indexing something that is not a collection is an error naming what it
    /// actually is, rather than a runtime failure with a number in it. The
    /// index is the language's one numeric type: there is no integer type, and
    /// `docs/decay-direction.md` records why introducing one for this alone
    /// would be the wrong trade. A fractional or out-of-range index is refused
    /// when it runs, where the value is known.
    pub(super) fn index_type(&mut self, object: &Expr, index: &Expr) -> Type {
        let object_type = self.expr_type(object);
        if self.refuse_optional(&object_type, object.span) {
            self.expr_type(index);
            return Type::Unknown;
        }
        if let Type::Map(key, value) = &object_type {
            return self.map_index_type(key, value, index);
        }
        let index_type = self.expr_type(index);
        self.require_type(&index_type, &Type::F32, index.span);
        match &object_type {
            Type::Array(element) => (**element).clone(),
            Type::Unknown => Type::Unknown,
            other => {
                self.error(
                    Code::NotIndexable,
                    object.span,
                    format!("`{}` cannot be indexed", other.display_name()),
                );
                Type::Unknown
            }
        }
    }

    pub(super) fn binary_type(&mut self, left: &Expr, op: BinaryOp, right: &Expr) -> Type {
        let left_type = self.expr_type(left);
        let right_type = self.expr_type(right);

        match op {
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Modulo => {
                self.arithmetic_type((&left_type, left.span), op, (&right_type, right.span))
            }
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                self.require_type(&left_type, &Type::F32, left.span);
                self.require_type(&right_type, &Type::F32, right.span);
                Type::Bool
            }
            BinaryOp::Fallback => {
                self.fallback_type((&left_type, left.span), (&right_type, right.span))
            }
            BinaryOp::And | BinaryOp::Or => {
                self.require_type(&left_type, &Type::Bool, left.span);
                self.require_type(&right_type, &Type::Bool, right.span);
                Type::Bool
            }
            BinaryOp::Equal | BinaryOp::NotEqual => {
                // Either way round: comparing a `Bolt` with an `Entity` asks
                // whether they are the same thing, whichever is written first.
                if !self.compatible(&left_type, &right_type)
                    && !self.compatible(&right_type, &left_type)
                {
                    self.error(
                        Code::CannotCompare,
                        right.span,
                        format!(
                            "cannot compare `{}` with `{}`",
                            left_type.display_name(),
                            right_type.display_name()
                        ),
                    );
                }
                Type::Bool
            }
        }
    }

    /// The type of arithmetic over two operands, vectors or numbers.
    pub(super) fn arithmetic_type(
        &mut self,
        left: (&Type, decay_syntax::Span),
        op: BinaryOp,
        right: (&Type, decay_syntax::Span),
    ) -> Type {
        if let Some(text) = self.string_arithmetic(left, op, right) {
            return text;
        }
        if let Some(vector) = self.vector_arithmetic(left, op, right) {
            return vector;
        }
        self.require_type(left.0, &Type::F32, left.1);
        self.require_type(right.0, &Type::F32, right.1);
        Type::F32
    }

    pub(super) fn assignment_type(&mut self, target: &Expr, op: AssignOp, value: &Expr) -> Type {
        let target_type = self.assignment_target_type(target);
        let value_type = self.expr_type(value);

        let binary = match op {
            AssignOp::Assign => None,
            AssignOp::Add => Some(BinaryOp::Add),
            AssignOp::Subtract => Some(BinaryOp::Subtract),
            AssignOp::Multiply => Some(BinaryOp::Multiply),
            AssignOp::Divide => Some(BinaryOp::Divide),
            AssignOp::Modulo => Some(BinaryOp::Modulo),
        };
        match binary {
            None => self.check_assignable(&target_type, &value_type, value.span),
            // `position += velocity * dt` is the arithmetic it spells, and the
            // result has to fit back where it came from: `speed += direction`
            // is a vector going into a number.
            Some(binary) => {
                let result = self.arithmetic_type(
                    (&target_type, target.span),
                    binary,
                    (&value_type, value.span),
                );
                self.check_assignable(&target_type, &result, value.span);
            }
        }

        target_type
    }

    pub(super) fn assignment_target_type(&mut self, target: &Expr) -> Type {
        match &target.kind {
            ExprKind::Identifier(name) => {
                if let Some(symbol) = self.lookup(name).cloned() {
                    if symbol.function.is_some() {
                        self.error(
                            Code::NotAPlace,
                            target.span,
                            format!("cannot assign to function `{name}`"),
                        );
                    } else if !symbol.mutable {
                        self.error(
                            Code::Immutable,
                            target.span,
                            format!("cannot assign to immutable `{name}`"),
                        );
                    }
                    symbol.ty
                } else if self.constants.contains_key(name) {
                    self.error(
                        Code::Immutable,
                        target.span,
                        format!("cannot assign to constant `{name}`"),
                    );
                    Type::Unknown
                } else {
                    self.error(
                        Code::UnknownName,
                        target.span,
                        format!("unknown name `{name}`"),
                    );
                    Type::Unknown
                }
            }
            ExprKind::Member { object, field } => {
                let object_type = self.expr_type(object);
                self.check_host_member_write(object, &object_type, field, target.span);
                if ((object_type.dimensions().is_some() || object_type == Type::Color)
                    && self.value_rooted(object))
                    || self.is_struct(&object_type)
                {
                    self.check_component_target(object, target.span);
                }
                self.check_state_assignment(object, field, target.span);
                if object_type == Type::Timer {
                    self.refuse_timer_write(field, target.span);
                    return Type::Unknown;
                }
                self.member_of(object, &object_type, field, target.span)
            }
            ExprKind::Index { object, index } => {
                self.element_target_type(object, index, target.span)
            }
            _ => {
                self.error(
                    Code::NotAPlace,
                    target.span,
                    "invalid assignment target".to_owned(),
                );
                Type::Unknown
            }
        }
    }
}
