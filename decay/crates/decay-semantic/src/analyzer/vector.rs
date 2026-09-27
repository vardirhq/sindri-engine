//! Vectors: building them, reading their components, their arithmetic, and
//! what else they can be asked.
//!
//! A vector is a language value, not a host type, so everything here is
//! decided by the language. What the analyzer adds on top of a type is a note
//! in [`ValueMember`] for each read or call the lowering has to perform itself
//! rather than hand to the host as a path.

use decay_syntax::{BinaryOp, Expr, ExprKind, Span, VectorOp};

use crate::diagnostic::ValueMember;
use crate::types::{COMPONENTS, Type, VEC2, VEC3};

use super::Analyzer;

impl Analyzer<'_, '_> {
    /// `Vec2(x, y)` or `Vec3(x, y, z)`, when `name` is one of them and no
    /// script binding has taken the name.
    pub(super) fn construct_type(&mut self, name: &str, args: &[Expr], span: Span) -> Option<Type> {
        let ty = match name {
            VEC2 => Type::Vec2,
            VEC3 => Type::Vec3,
            _ => return None,
        };
        if self.lookup(name).is_some() {
            return None;
        }
        let dimensions = ty.dimensions().unwrap_or(0);
        if args.len() != dimensions {
            self.error(
                span,
                format!(
                    "`{name}` takes {dimensions} numbers ({}), found {}",
                    COMPONENTS[..dimensions].join(", "),
                    args.len()
                ),
            );
        }
        for argument in args {
            let actual = self.expr_type(argument);
            self.require_type(&actual, &Type::F32, argument.span);
        }
        self.value_members
            .insert(span, ValueMember::Construct(dimensions));
        Some(ty)
    }

    /// The type of `vector.field`, noting how the lowering reaches it.
    pub(super) fn vector_member_type(
        &mut self,
        object: &Expr,
        object_type: &Type,
        field: &str,
        span: Span,
    ) -> Type {
        let dimensions = object_type.dimensions().unwrap_or(0);
        if let Some(index) = COMPONENTS[..dimensions].iter().position(|c| *c == field) {
            if self.value_rooted(object) {
                self.value_members
                    .insert(span, ValueMember::Component(index));
            }
            return Type::F32;
        }
        match VectorOp::named(field) {
            Some((op, 0)) => {
                self.value_members.insert(span, ValueMember::Vector(op));
                match op {
                    VectorOp::Normalized => object_type.clone(),
                    _ => Type::F32,
                }
            }
            Some(_) => {
                self.error(
                    span,
                    format!(
                        "`{field}` on `{}` needs arguments -- call it: `.{field}(...)`",
                        object_type.display_name()
                    ),
                );
                Type::Unknown
            }
            None => {
                self.error(span, missing_member(object_type, field));
                Type::Unknown
            }
        }
    }

    /// The type of `vector.method(args)`.
    pub(super) fn vector_call_type(
        &mut self,
        object_type: &Type,
        field: &str,
        args: &[Expr],
        span: Span,
    ) -> Type {
        let (params, returns) = match VectorOp::named(field) {
            Some((VectorOp::Dot | VectorOp::Distance, _)) => (vec![object_type.clone()], Type::F32),
            Some((VectorOp::Lerp, _)) => {
                (vec![object_type.clone(), Type::F32], object_type.clone())
            }
            Some((VectorOp::Length | VectorOp::Normalized, _)) => {
                self.error(
                    span,
                    format!("`{field}` is a property, not a function -- write `.{field}`"),
                );
                self.check_arguments_only(args);
                return Type::Unknown;
            }
            None => {
                let dimensions = object_type.dimensions().unwrap_or(0);
                if COMPONENTS[..dimensions].contains(&field) {
                    self.error(
                        span,
                        format!("`{field}` is a number, not a function -- write `.{field}`"),
                    );
                } else {
                    self.error(span, missing_member(object_type, field));
                }
                self.check_arguments_only(args);
                return Type::Unknown;
            }
        };
        let op = VectorOp::named(field).map(|(op, _)| op);
        self.check_call(
            &crate::types::FunctionType {
                params,
                return_type: returns.clone(),
            },
            args,
            span,
        );
        if let Some(op) = op {
            self.value_members.insert(span, ValueMember::Vector(op));
        }
        returns
    }

    fn check_arguments_only(&mut self, args: &[Expr]) {
        for argument in args {
            self.expr_type(argument);
        }
    }

    /// The type `left op right` has when either side is a vector, or `None`
    /// when neither is and the numeric rules apply.
    pub(super) fn vector_arithmetic(
        &mut self,
        left: (&Type, Span),
        op: BinaryOp,
        right: (&Type, Span),
    ) -> Option<Type> {
        let (left_type, left_span) = left;
        let (right_type, right_span) = right;
        if left_type.dimensions().is_none() && right_type.dimensions().is_none() {
            return None;
        }
        let number = |ty: &Type| matches!(ty, Type::F32 | Type::Unknown);
        let vector = if left_type.dimensions().is_some() {
            left_type.clone()
        } else {
            right_type.clone()
        };
        let symbol = match op {
            BinaryOp::Add => "+",
            BinaryOp::Subtract => "-",
            BinaryOp::Multiply => "*",
            BinaryOp::Divide => "/",
            BinaryOp::Modulo => "%",
            _ => return None,
        };
        let fits = match op {
            BinaryOp::Add | BinaryOp::Subtract => {
                left_type == right_type
                    || matches!(left_type, Type::Unknown)
                    || matches!(right_type, Type::Unknown)
            }
            BinaryOp::Multiply => {
                (left_type.dimensions().is_some() && number(right_type))
                    || (number(left_type) && right_type.dimensions().is_some())
            }
            BinaryOp::Divide => left_type.dimensions().is_some() && number(right_type),
            _ => false,
        };
        if !fits {
            let found = format!(
                "found `{}` {symbol} `{}`",
                left_type.display_name(),
                right_type.display_name()
            );
            let message = match op {
                BinaryOp::Add | BinaryOp::Subtract => {
                    format!("`{symbol}` needs two vectors of the same size, {found}")
                }
                BinaryOp::Multiply => format!(
                    "a vector is multiplied by a number, {found} -- for the dot product write `a.dot(b)`"
                ),
                BinaryOp::Divide => format!("a vector is divided by a number, {found}"),
                _ => format!("`{symbol}` works on numbers, not vectors: {found}"),
            };
            let at = if left_type.dimensions().is_some() && !number(right_type) {
                right_span
            } else {
                left_span
            };
            self.error(at, message);
        }
        Some(vector)
    }

    /// Whether `object` is a value the script holds — so a component of it is
    /// read by the language — rather than a path the host answers.
    ///
    /// A local, a parameter, a field of the script, and anything computed are
    /// values. `this.transform.position` and `target.transform.position` are
    /// paths, and reading `.x` through them stays the host's single path load
    /// it always was.
    pub(super) fn value_rooted(&self, object: &Expr) -> bool {
        match &object.kind {
            ExprKind::Group(inner) => self.value_rooted(inner),
            ExprKind::Identifier(name) => self.lookup(name).is_some(),
            ExprKind::Member {
                object: inner,
                field,
            } => {
                self.value_members.contains_key(&object.span)
                    || (matches!(&inner.kind, ExprKind::Identifier(root) if root == "this")
                        && self.own_field(field).is_some())
            }
            _ => true,
        }
    }

    /// The script's own field of this name, if there is one.
    pub(super) fn own_field(&self, name: &str) -> Option<&super::Symbol> {
        self.scopes
            .first()
            .and_then(|scope| scope.get(name))
            .filter(|symbol| symbol.function.is_none())
    }

    /// Checks `vector.component = ...` beyond what any assignment checks:
    /// only a component of something the script can store back into, and only
    /// when that thing is mutable.
    pub(super) fn check_component_target(&mut self, object: &Expr, span: Span) {
        let root = match &object.kind {
            ExprKind::Identifier(name) => self
                .lookup(name)
                .map(|symbol| (name.clone(), symbol.mutable)),
            ExprKind::Member {
                object: inner,
                field,
            } if matches!(&inner.kind, ExprKind::Identifier(root) if root == "this") => self
                .own_field(field)
                .map(|symbol| (format!("this.{field}"), symbol.mutable)),
            _ => None,
        };
        match root {
            Some((_, true)) => {}
            Some((name, false)) => {
                self.error(
                    span,
                    format!("cannot assign to a component of immutable `{name}`"),
                );
            }
            None => self.error(
                span,
                "only a component of a variable or field can be assigned".to_owned(),
            ),
        }
    }
}

fn missing_member(object_type: &Type, field: &str) -> String {
    let dimensions = object_type.dimensions().unwrap_or(0);
    let components = COMPONENTS[..dimensions]
        .iter()
        .map(|c| format!("`{c}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let operations = VectorOp::ALL
        .iter()
        .map(|(_, name, _)| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "`{}` has no member `{field}`; it has {components}, and {operations}",
        object_type.display_name()
    )
}
