//! Calls: what is being called, and whether the arguments fit.

use decay_syntax::{Expr, ExprKind, Span};

use crate::diagnostic::container_function_message;
use crate::environment::ExternalSymbol;
use crate::types::{FunctionType, Type};

use super::{Analyzer, MemberLookup};

impl Analyzer<'_, '_> {
    pub(super) fn call_type(&mut self, callee: &Expr, args: &[Expr], span: Span) -> Type {
        // A method on a host type: `this.rigidbody.add_impulse(v)`. Resolved
        // before the general path so that the arguments are checked against a
        // real signature rather than accepted because the callee was unknown.
        if let ExprKind::Member { object, field } = &callee.kind {
            let object_type = self.expr_type(object);
            if object_type.dimensions().is_some() {
                return self.vector_call_type(&object_type, field, args, span);
            }
            match self.member_symbol(&object_type, field, is_this(object)) {
                Some(MemberLookup::Found(ExternalSymbol::Function(function))) => {
                    self.check_call(&function, args, span);
                    return function.return_type;
                }
                Some(MemberLookup::Found(ExternalSymbol::Value(_))) => {
                    self.error(
                        callee.span,
                        format!(
                            "`{field}` on `{}` is a value, not a function",
                            object_type.display_name()
                        ),
                    );
                }
                Some(MemberLookup::Missing) => {
                    self.error(
                        callee.span,
                        format!("`{}` has no member `{field}`", object_type.display_name()),
                    );
                }
                Some(MemberLookup::ContainerFunction) => {
                    self.error(callee.span, container_function_message(field));
                }
                None => {}
            }
            for arg in args {
                self.expr_type(arg);
            }
            return Type::Unknown;
        }
        self.call_named(callee, args, span)
    }

    pub(super) fn call_named(&mut self, callee: &Expr, args: &[Expr], span: Span) -> Type {
        if let ExprKind::Identifier(name) = &callee.kind
            && let Some(vector) = self.construct_type(name, args, span)
        {
            return vector;
        }
        if let ExprKind::Identifier(name) = &callee.kind {
            if let Some(symbol) = self.lookup(name).cloned() {
                if let Some(function) = symbol.function {
                    self.check_call(&function, args, span);
                    return function.return_type;
                }
                self.error(callee.span, format!("`{name}` is not callable"));
                for arg in args {
                    self.expr_type(arg);
                }
                return Type::Unknown;
            }

            if let Some(external) = self.environment.globals.get(name).cloned() {
                match external {
                    ExternalSymbol::Function(function) => {
                        self.check_call(&function, args, span);
                        return function.return_type;
                    }
                    ExternalSymbol::Value(_) => {
                        self.error(callee.span, format!("`{name}` is not callable"));
                    }
                }
                for arg in args {
                    self.expr_type(arg);
                }
                return Type::Unknown;
            }
        }

        self.expr_type(callee);
        for arg in args {
            self.expr_type(arg);
        }
        Type::Unknown
    }

    pub(super) fn check_call(&mut self, function: &FunctionType, args: &[Expr], span: Span) {
        if function.params.len() != args.len() {
            self.error(
                span,
                format!(
                    "expected {} argument(s), found {}",
                    function.params.len(),
                    args.len()
                ),
            );
        }

        for (argument, expected) in args.iter().zip(&function.params) {
            let actual = self.expr_type(argument);
            self.check_assignable(expected, &actual, argument.span);
        }
    }
}

/// Whether an expression is `this` itself, rather than a held reference that
/// happens to have the same type.
pub(super) fn is_this(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::Identifier(name) if name == "this")
}
