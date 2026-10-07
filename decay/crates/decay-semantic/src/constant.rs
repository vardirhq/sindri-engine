//! Constants: `const ARENA: f32 = 12.0;`, worked out when the file compiles.
//!
//! A constant's value is folded here from literals, operators, enum variants
//! and other constants, and every use of it becomes that value. Nothing is
//! looked up while a script runs, and nothing can change it, so a constant is
//! as cheap as the literal it names.
//!
//! Folding is its own step rather than part of the walk over a program because
//! a host folds too: a `shared const` in one file is used in another, and the
//! host that reads every file's declarations is the one place that can work
//! out one whose value names a constant from a third.

use std::collections::{BTreeMap, HashMap, HashSet};

use decay_syntax::{BinaryOp, ConstDecl, Expr, ExprKind, Span, UnaryOp};

use crate::codes::Code;
use crate::types::Type;

/// A constant's value.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Number(f64),
    Bool(bool),
    Text(String),
    /// An enum's variant: `Phase.Lobby`.
    Variant {
        enumeration: String,
        variant: String,
    },
    /// A color: `Color(1.0, 0.5, 0.0)` or `Color("#ff8000")`.
    Color([f64; 4]),
}

impl ConstValue {
    /// The type a value of this kind has.
    #[must_use]
    pub fn ty(&self) -> Type {
        match self {
            Self::Number(_) => Type::F32,
            Self::Bool(_) => Type::Bool,
            Self::Text(_) => Type::String,
            Self::Variant { enumeration, .. } => Type::Named(enumeration.clone()),
            Self::Color(_) => Type::Color,
        }
    }

    /// As a reader would write it: `12`, `true`, `"Ready"`, `Phase.Lobby`.
    #[must_use]
    pub fn display(&self) -> String {
        match self {
            Self::Number(number) => format!("{number}"),
            Self::Bool(flag) => format!("{flag}"),
            Self::Text(text) => format!("{text:?}"),
            Self::Variant {
                enumeration,
                variant,
            } => format!("{enumeration}.{variant}"),
            Self::Color(channels) => format!(
                "Color({})",
                channels
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

/// A constant that could not be worked out, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct FoldError {
    pub code: Code,
    pub span: Span,
    pub message: String,
}

/// Works out every declared constant.
///
/// `known` holds constants already worked out elsewhere, which a declaration
/// here may use; a declaration here of the same name wins. `variants` answers
/// an enum's variants by the enum's name. Every declaration that can be worked
/// out is, even when another cannot, so one mistake does not hide the rest.
#[must_use]
pub fn fold_constants<'a, S: std::hash::BuildHasher>(
    declarations: impl IntoIterator<Item = &'a ConstDecl>,
    known: &HashMap<String, ConstValue, S>,
    variants: &dyn Fn(&str) -> Option<Vec<String>>,
) -> (BTreeMap<String, ConstValue>, Vec<FoldError>) {
    let known: HashMap<&str, &ConstValue> = known
        .iter()
        .map(|(name, value)| (name.as_str(), value))
        .collect();
    let mut folder = Folder {
        declared: HashMap::new(),
        known: &known,
        variants,
        done: BTreeMap::new(),
        failed: HashSet::new(),
        visiting: Vec::new(),
        errors: Vec::new(),
    };
    let mut order = Vec::new();
    for declaration in declarations {
        order.push(declaration.name.clone());
        folder
            .declared
            .entry(declaration.name.clone())
            .or_insert(declaration);
    }
    for name in order {
        folder.constant(&name);
    }
    (folder.done, folder.errors)
}

struct Folder<'a, 'k> {
    declared: HashMap<String, &'a ConstDecl>,
    known: &'k HashMap<&'k str, &'k ConstValue>,
    variants: &'k dyn Fn(&str) -> Option<Vec<String>>,
    done: BTreeMap<String, ConstValue>,
    /// Declarations already reported, so a constant that names a broken one
    /// is not reported again for it.
    failed: HashSet<String>,
    /// The declarations being worked out, innermost last, to catch one that
    /// needs itself.
    visiting: Vec<String>,
    errors: Vec<FoldError>,
}

impl Folder<'_, '_> {
    /// A declared constant's value, working it out the first time it is asked
    /// for. `None` when it cannot be, which has been reported.
    fn constant(&mut self, name: &str) -> Option<ConstValue> {
        if let Some(value) = self.done.get(name) {
            return Some(value.clone());
        }
        if self.failed.contains(name) {
            return None;
        }
        let declaration = *self.declared.get(name)?;
        if let Some(start) = self.visiting.iter().position(|visiting| visiting == name) {
            let mut cycle = self.visiting[start..].to_vec();
            cycle.push(name.to_owned());
            self.error(
                Code::ConstantCycle,
                declaration.name_span,
                format!(
                    "constant `{name}` needs its own value to be worked out: {}",
                    cycle.join(" -> ")
                ),
            );
            for member in cycle {
                self.failed.insert(member);
            }
            return None;
        }
        self.visiting.push(name.to_owned());
        let value = self.expr(&declaration.value);
        self.visiting.pop();
        let Some(value) = value else {
            self.failed.insert(name.to_owned());
            return None;
        };
        let declared = Type::from_ref(&declaration.ty);
        let allowed = matches!(
            declared,
            Type::F32 | Type::Bool | Type::String | Type::Color
        ) || matches!(&declared, Type::Named(enumeration) if (self.variants)(enumeration).is_some());
        if !allowed {
            self.error(
                Code::ConstantType,
                declaration.ty.span,
                format!(
                    "a constant holds `f32`, `bool`, `String`, `Color` or an enum, not `{}`",
                    declared.display_name()
                ),
            );
            self.failed.insert(name.to_owned());
            return None;
        }
        if value.ty() != declared {
            self.error(
                Code::TypeMismatch,
                declaration.value.span,
                format!(
                    "constant `{name}` is declared `{}` but its value is `{}`",
                    declared.display_name(),
                    value.ty().display_name()
                ),
            );
            self.failed.insert(name.to_owned());
            return None;
        }
        self.done.insert(name.to_owned(), value.clone());
        Some(value)
    }

    fn expr(&mut self, expr: &Expr) -> Option<ConstValue> {
        match &expr.kind {
            ExprKind::Number(number) => Some(ConstValue::Number(*number)),
            ExprKind::Bool(flag) => Some(ConstValue::Bool(*flag)),
            ExprKind::String(text) => Some(ConstValue::Text(text.clone())),
            ExprKind::Group(inner) => self.expr(inner),
            ExprKind::Identifier(name) => self.name(name, expr.span),
            ExprKind::Member { object, field } => self.variant(object, field, expr.span),
            ExprKind::Unary { op, expr: inner } => {
                let value = self.expr(inner)?;
                match (op, value) {
                    (UnaryOp::Negate, ConstValue::Number(number)) => {
                        Some(ConstValue::Number(-number))
                    }
                    (UnaryOp::Not, ConstValue::Bool(flag)) => Some(ConstValue::Bool(!flag)),
                    (UnaryOp::Negate, value) => self.operand(expr.span, "-", &value),
                    (UnaryOp::Not, value) => self.operand(expr.span, "!", &value),
                }
            }
            ExprKind::Binary { left, op, right } => {
                let left_value = self.expr(left)?;
                let right_value = self.expr(right)?;
                self.binary(*op, &left_value, &right_value, expr.span)
            }
            ExprKind::Call { callee, args } if matches!(&callee.kind, ExprKind::Identifier(name) if name == crate::types::COLOR) => {
                self.color(args, expr.span)
            }
            _ => {
                self.error(
                    Code::ConstantNotFixed,
                    expr.span,
                    "a constant's value must be worked out when the file compiles: use \
                     literals, operators, enum variants and other constants"
                        .to_owned(),
                );
                None
            }
        }
    }

    /// `Color(r, g, b)`, `Color(r, g, b, a)` or `Color("#rrggbb")`, from
    /// values that are themselves constant.
    fn color(&mut self, args: &[Expr], span: Span) -> Option<ConstValue> {
        let values = args
            .iter()
            .map(|argument| self.expr(argument))
            .collect::<Option<Vec<_>>>()?;
        let channels = match values.as_slice() {
            [ConstValue::Text(text)] => decay_syntax::parse_hex(text),
            [
                ConstValue::Number(r),
                ConstValue::Number(g),
                ConstValue::Number(b),
            ] => Some([*r, *g, *b, 1.0]),
            [
                ConstValue::Number(r),
                ConstValue::Number(g),
                ConstValue::Number(b),
                ConstValue::Number(a),
            ] => Some([*r, *g, *b, *a]),
            _ => None,
        };
        if channels.is_none() {
            self.error(
                Code::ConstantNotFixed,
                span,
                "a constant color is `Color(r, g, b)`, `Color(r, g, b, a)` or `Color(\"#rrggbb\")`"
                    .to_owned(),
            );
        }
        channels.map(ConstValue::Color)
    }

    fn name(&mut self, name: &str, span: Span) -> Option<ConstValue> {
        if self.declared.contains_key(name) {
            return self.constant(name);
        }
        if let Some(value) = self.known.get(name) {
            return Some((*value).clone());
        }
        self.error(
            Code::ConstantNotFixed,
            span,
            format!("`{name}` is not a constant, so a constant's value cannot use it"),
        );
        None
    }

    fn variant(&mut self, object: &Expr, field: &str, span: Span) -> Option<ConstValue> {
        if let ExprKind::Identifier(enumeration) = &object.kind
            && let Some(variants) = (self.variants)(enumeration)
        {
            if variants.iter().any(|variant| variant == field) {
                return Some(ConstValue::Variant {
                    enumeration: enumeration.clone(),
                    variant: field.to_owned(),
                });
            }
            self.error(
                Code::UnknownMember,
                span,
                format!("`{enumeration}` has no variant `{field}`"),
            );
            return None;
        }
        self.error(
            Code::ConstantNotFixed,
            span,
            "a constant's value may name an enum's variant, but nothing else reached with `.`"
                .to_owned(),
        );
        None
    }

    fn binary(
        &mut self,
        op: BinaryOp,
        left: &ConstValue,
        right: &ConstValue,
        span: Span,
    ) -> Option<ConstValue> {
        use ConstValue::{Bool, Number, Text};
        let value = match (op, left, right) {
            (BinaryOp::Add, Number(a), Number(b)) => Number(a + b),
            (BinaryOp::Subtract, Number(a), Number(b)) => Number(a - b),
            (BinaryOp::Multiply, Number(a), Number(b)) => Number(a * b),
            (BinaryOp::Divide | BinaryOp::Modulo, Number(_), Number(b)) if *b == 0.0 => {
                self.error(
                    Code::ConstantNotFixed,
                    span,
                    "a constant's value divides by zero".to_owned(),
                );
                return None;
            }
            (BinaryOp::Divide, Number(a), Number(b)) => Number(a / b),
            (BinaryOp::Modulo, Number(a), Number(b)) => Number(a % b),
            (BinaryOp::Add, Text(a), Text(b)) => Text(format!("{a}{b}")),
            (BinaryOp::Less, Number(a), Number(b)) => Bool(a < b),
            (BinaryOp::LessEqual, Number(a), Number(b)) => Bool(a <= b),
            (BinaryOp::Greater, Number(a), Number(b)) => Bool(a > b),
            (BinaryOp::GreaterEqual, Number(a), Number(b)) => Bool(a >= b),
            (BinaryOp::And, Bool(a), Bool(b)) => Bool(*a && *b),
            (BinaryOp::Or, Bool(a), Bool(b)) => Bool(*a || *b),
            (BinaryOp::Equal | BinaryOp::NotEqual, a, b) if a.ty() == b.ty() => {
                Bool((a == b) == (op == BinaryOp::Equal))
            }
            (BinaryOp::Add, Text(_), _) | (BinaryOp::Add, _, Text(_)) => {
                self.error(
                    Code::ConstantNotFixed,
                    span,
                    "a constant joins text only to text; join a number to it where it is used"
                        .to_owned(),
                );
                return None;
            }
            _ => {
                return self.operand(span, op.symbol(), left);
            }
        };
        Some(value)
    }

    fn operand(&mut self, span: Span, op: &str, value: &ConstValue) -> Option<ConstValue> {
        self.error(
            Code::InvalidOperand,
            span,
            format!(
                "`{}` cannot be used with `{op}` in a constant",
                value.ty().display_name()
            ),
        );
        None
    }

    fn error(&mut self, code: Code, span: Span, message: String) {
        self.errors.push(FoldError {
            code,
            span,
            message,
        });
    }
}
