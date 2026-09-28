//! Lists: writing one, walking a range, and what a list can be asked or told.
//!
//! A list is a value, like a vector: assigning one copies it. So a change —
//! `push`, `pop`, `xs[i] = v` — is made to the list one variable or field
//! holds, and needs that variable or field to be one the script may change.

use decay_syntax::{Expr, ListOp, Span};

use crate::diagnostic::ValueMember;
use crate::types::{FunctionType, Type};

use super::Analyzer;

impl Analyzer<'_, '_> {
    /// `[a, b, c]`: a list of the first element's type, which every other
    /// element must fit. An empty one fits any list it is put in.
    pub(super) fn list_literal_type(&mut self, elements: &[Expr]) -> Type {
        let mut element = Type::Unknown;
        for (position, item) in elements.iter().enumerate() {
            let ty = self.expr_type(item);
            if position == 0 {
                element = ty;
            } else if !self.compatible(&element, &ty) {
                self.error(
                    item.span,
                    format!(
                        "a list holds one type: this is `{}`, and the first element is `{}`",
                        ty.display_name(),
                        element.display_name()
                    ),
                );
            }
        }
        Type::array_of(element)
    }

    /// `start..end` anywhere but a `for`.
    pub(super) fn stray_range_type(&mut self, start: &Expr, end: &Expr, span: Span) -> Type {
        self.expr_type(start);
        self.expr_type(end);
        self.error(
            span,
            "a range is only walked by `for`, as in `for i in 0..count { }`".to_owned(),
        );
        Type::Unknown
    }

    /// The two ends of a range a `for` walks: numbers, both.
    pub(super) fn range_type(&mut self, start: &Expr, end: &Expr) -> Type {
        for side in [start, end] {
            let ty = self.expr_type(side);
            self.require_type(&ty, &Type::F32, side.span);
        }
        Type::F32
    }

    /// `list.op(args)`.
    pub(super) fn list_call_type(
        &mut self,
        object: &Expr,
        element: &Type,
        field: &str,
        args: &[Expr],
        span: Span,
    ) -> Type {
        let Some((op, _)) = ListOp::named(field) else {
            if crate::types::is_length(field) {
                self.error(
                    span,
                    format!("`{field}` is a property, not a function -- write `.{field}`"),
                );
            } else {
                self.error(span, missing_member(element, field));
            }
            for argument in args {
                self.expr_type(argument);
            }
            return Type::Unknown;
        };
        let (params, returns) = match op {
            ListOp::Push => (vec![element.clone()], Type::Unit),
            ListOp::Pop => (vec![], element.clone()),
            ListOp::Insert => (vec![Type::F32, element.clone()], Type::Unit),
            ListOp::RemoveAt => (vec![Type::F32], element.clone()),
            ListOp::Clear => (vec![], Type::Unit),
            ListOp::Contains => (vec![element.clone()], Type::Bool),
            ListOp::IndexOf => (vec![element.clone()], Type::F32),
            ListOp::SetAt => unreachable!("not a name a script can call"),
        };
        if op.changes() {
            self.check_list_place(object, field);
        }
        self.check_call(
            &FunctionType {
                params,
                return_type: returns.clone(),
            },
            args,
            span,
        );
        self.value_members.insert(span, ValueMember::List(op));
        returns
    }

    /// `list.push` read without calling it, or any other name on a list.
    pub(super) fn list_member_error(&mut self, element: &Type, field: &str, span: Span) {
        if ListOp::named(field).is_some() {
            self.error(
                span,
                format!("`{field}` on a list is a function -- call it: `.{field}(...)`"),
            );
        } else {
            self.error(span, missing_member(element, field));
        }
    }

    /// `list[index] = value`: the element's type, once the list is one the
    /// script may change.
    pub(super) fn list_element_target_type(&mut self, object: &Expr, index: &Expr) -> Type {
        let object_type = self.expr_type(object);
        let index_type = self.expr_type(index);
        self.require_type(&index_type, &Type::F32, index.span);
        match &object_type {
            Type::Array(element) => {
                self.check_list_place(object, "[]");
                (**element).clone()
            }
            Type::Unknown => Type::Unknown,
            other => {
                self.error(
                    object.span,
                    format!("`{}` cannot be indexed", other.display_name()),
                );
                Type::Unknown
            }
        }
    }

    /// A change is made to the list a variable, parameter or field of this
    /// script holds, and only when that one may change.
    fn check_list_place(&mut self, object: &Expr, what: &str) {
        let doing = if what == "[]" {
            "an element can be set".to_owned()
        } else {
            format!("`{what}` can change it")
        };
        let place = self.place_root(object);
        match place {
            Some((_, true)) => {}
            Some((name, false)) => self.error(
                object.span,
                format!("`{name}` is a `let`, so it cannot change -- declare it `var` if {doing}"),
            ),
            None => self.error(
                object.span,
                format!(
                    "only a list in a variable or a field of this script can change, and this is \
                     neither -- put it in a `var` first if {doing}"
                ),
            ),
        }
    }
}

fn missing_member(element: &Type, field: &str) -> String {
    let operations = ListOp::ALL
        .iter()
        .map(|(_, name, _)| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "`{}` has no member `{field}`; it has `{}`, and {operations}",
        Type::array_of(element.clone()).display_name(),
        crate::types::LENGTH
    )
}
