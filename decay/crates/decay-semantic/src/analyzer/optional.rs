//! Values that may be missing: `f32?`, `String?`, `List<Entity>?`.
//!
//! An optional holds a value of its type or `null`, and is never used as its
//! type until something has said which: `value ?? fallback`, or a check that
//! narrows a `let` or parameter — inside `if best != null { }`, and after
//! `if best == null { return; }`, `best` is a plain `f32`.

use std::collections::HashMap;

use decay_syntax::{BinaryOp, Block, Expr, ExprKind, Span, Stmt};

use crate::codes::Code;
use crate::types::Type;

use super::{Analyzer, Symbol};

/// What to add to a mismatch when the value given may be `null` and would
/// otherwise have fitted.
pub(crate) fn null_hint(actual: &Type, expected: &Type) -> &'static str {
    match actual {
        Type::Optional(inner) if !matches!(expected, Type::Optional(_)) && **inner == *expected => {
            " -- it may be `null`: give a fallback with `?? value`, or check `!= null` first"
        }
        _ => "",
    }
}

impl Analyzer<'_, '_> {
    /// `value ?? fallback`: the value's type without `null` when the fallback
    /// fits it — or the optional type again, when the fallback may be `null`
    /// too.
    pub(super) fn fallback_type(&mut self, value: (&Type, Span), fallback: (&Type, Span)) -> Type {
        let (value_type, value_span) = value;
        let (fallback_type, fallback_span) = fallback;
        let result = match value_type {
            Type::Optional(inner) => (**inner).clone(),
            Type::Named(_) | Type::Unknown => value_type.clone(),
            Type::Null => {
                self.error(
                    Code::InvalidOperand,
                    value_span,
                    "`null ?? value` is always the value; write the value".to_owned(),
                );
                return fallback_type.clone();
            }
            other => {
                self.error(
                    Code::InvalidOperand,
                    value_span,
                    format!(
                        "this is `{}`, which is never `null`, so `??` never uses its fallback",
                        other.display_name()
                    ),
                );
                return other.clone();
            }
        };
        if self.compatible(&result, fallback_type.without_null())
            || matches!(fallback_type, Type::Null)
        {
            return if matches!(fallback_type, Type::Optional(_) | Type::Null) {
                result.or_null()
            } else {
                result
            };
        }
        self.error(
            Code::TypeMismatch,
            fallback_span,
            format!(
                "the fallback is `{}`, but the value is `{}`",
                fallback_type.display_name(),
                result.display_name()
            ),
        );
        result
    }

    /// A value that may be `null` used where it cannot be: reading a member,
    /// indexing, or calling one of its operations. Reports it, answering
    /// whether it did.
    pub(super) fn refuse_optional(&mut self, ty: &Type, span: Span) -> bool {
        let Type::Optional(inner) = ty else {
            return false;
        };
        self.error(
            Code::TypeMismatch,
            span,
            format!(
                "this is `{}?`, which may be `null` -- give a fallback with `?? value`, or check \
                 `!= null` first",
                inner.display_name()
            ),
        );
        true
    }

    /// `if condition { then } else { otherwise }`, with the `let`s and
    /// parameters the condition shows are not `null` narrowed in `then`, and
    /// the ones it shows are `null` narrowed in `otherwise`.
    pub(super) fn analyze_if_branches(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        else_branch: Option<&Block>,
    ) {
        let not_null = self.narrowed(condition, BinaryOp::NotEqual);
        self.with_narrowed(not_null, |analyzer| {
            analyzer.analyze_block(then_branch, true);
        });
        if let Some(else_branch) = else_branch {
            let known_otherwise = self.narrowed(condition, BinaryOp::Equal);
            self.with_narrowed(known_otherwise, |analyzer| {
                analyzer.analyze_block(else_branch, true);
            });
        }
    }

    /// After `if x == null { return; }` — a branch that never falls through,
    /// with no `else` — the rest of the block may take `x` as not `null`.
    /// Pushes the scope that says so, answering whether it did.
    pub(super) fn narrow_after(&mut self, statement: &Stmt) -> bool {
        let Stmt::If {
            condition,
            then_branch,
            else_branch: None,
            ..
        } = statement
        else {
            return false;
        };
        if !never_falls_through(then_branch) {
            return false;
        }
        let narrowed = self.narrowed(condition, BinaryOp::Equal);
        if narrowed.is_empty() {
            return false;
        }
        self.scopes.push(narrowed);
        true
    }

    fn with_narrowed(
        &mut self,
        narrowed: HashMap<String, Symbol>,
        analyze: impl FnOnce(&mut Self),
    ) {
        if narrowed.is_empty() {
            analyze(self);
            return;
        }
        // Its own scope, outside the block's, so a `let` in the block may
        // still reuse the name.
        self.scopes.push(narrowed);
        analyze(self);
        self.scopes.pop();
    }

    /// The immutable optionals a condition shows are not `null`, each as the
    /// symbol it becomes: `x != null` for `op` `NotEqual`; or `x == null` for
    /// `Equal`, where it is the *other* branch in which `x` is known.
    ///
    /// `a != null && b != null` narrows both where it holds; `a == null ||
    /// b == null` narrows both where it does not.
    fn narrowed(&self, condition: &Expr, op: BinaryOp) -> HashMap<String, Symbol> {
        let mut found = HashMap::new();
        self.collect_narrowed(condition, op, &mut found);
        found
    }

    fn collect_narrowed(
        &self,
        condition: &Expr,
        op: BinaryOp,
        found: &mut HashMap<String, Symbol>,
    ) {
        let joined = if op == BinaryOp::NotEqual {
            BinaryOp::And
        } else {
            BinaryOp::Or
        };
        match &condition.kind {
            ExprKind::Group(inner) => self.collect_narrowed(inner, op, found),
            ExprKind::Binary {
                left,
                op: written,
                right,
            } if *written == joined => {
                self.collect_narrowed(left, op, found);
                self.collect_narrowed(right, op, found);
            }
            ExprKind::Binary {
                left,
                op: written,
                right,
            } if *written == op => {
                let ((ExprKind::Identifier(name), ExprKind::Null)
                | (ExprKind::Null, ExprKind::Identifier(name))) = (&left.kind, &right.kind)
                else {
                    return;
                };
                if let Some(symbol) = self.lookup(name)
                    && !symbol.mutable
                    && symbol.function.is_none()
                    && let Type::Optional(inner) = &symbol.ty
                {
                    let mut narrowed = symbol.clone();
                    narrowed.ty = (**inner).clone();
                    found.insert(name.clone(), narrowed);
                }
            }
            _ => {}
        }
    }
}

/// Whether a block always leaves: its last statement returns, breaks or
/// continues.
fn never_falls_through(block: &Block) -> bool {
    matches!(
        block.statements.last(),
        Some(Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. })
    )
}
