//! Containers and functions: their members, fields, and bodies.

use std::collections::{HashMap, HashSet};

use decay_syntax::{Expr, ExprKind, FieldDecl, FunctionDecl, Member, Span};

use crate::types::{FunctionType, Type};

use super::{Analyzer, Symbol};

impl Analyzer<'_, '_> {
    pub(super) fn analyze_container(&mut self, container: &decay_syntax::ContainerDecl) {
        let mut members = HashMap::new();

        for member in &container.members {
            match member {
                Member::Field(field) => {
                    let ty = match field.ty.as_ref() {
                        Some(ty) => self.resolve_type(ty),
                        None => self
                            .variant_initializer_type(field.initializer.as_ref())
                            .unwrap_or_else(|| built_value_type(field.initializer.as_ref())),
                    };
                    self.insert_member(
                        &mut members,
                        &field.name,
                        Symbol {
                            ty,
                            mutable: field.mutable,
                            function: None,
                        },
                        field.span,
                    );
                }
                Member::Function(function) => {
                    let params = self.handler_params(function).unwrap_or_else(|| {
                        function
                            .params
                            .iter()
                            .map(|param| param.ty.as_ref().map_or(Type::Unknown, Type::from_ref))
                            .collect()
                    });
                    let signature = FunctionType {
                        params: params.clone(),
                        return_type: function
                            .return_type
                            .as_ref()
                            .map_or(Type::Unit, Type::from_ref),
                    };
                    self.insert_member(
                        &mut members,
                        &function.name,
                        Symbol {
                            ty: Type::Unknown,
                            mutable: false,
                            function: Some(signature),
                        },
                        function.span,
                    );
                }
            }
        }

        // Field initializers run in declaration order, so a field may read only
        // one declared above it. Reading one below used to compile and then
        // fail at runtime with `UnknownPath` — a mistake a statically typed
        // language should refuse before Play, and one whose runtime failure
        // named a path rather than the field that could not have a value yet.
        let mut declared = HashSet::new();
        for member in &container.members {
            if let Member::Field(field) = member {
                self.check_field_order(field, &members, &declared);
                self.check_field_initializer(field, &members);
                declared.insert(field.name.clone());
            }
        }

        for member in &container.members {
            if let Member::Function(function) = member {
                self.analyze_function(container, function, &members);
            }
        }
    }

    pub(super) fn insert_member(
        &mut self,
        members: &mut HashMap<String, Symbol>,
        name: &str,
        symbol: Symbol,
        span: Span,
    ) {
        if members.insert(name.to_owned(), symbol).is_some() {
            self.error(span, format!("duplicate member `{name}`"));
        }
    }

    /// Reports every field this initializer reads that is not available yet.
    ///
    /// The type check that follows still sees the whole member map, so the
    /// initializer is analyzed as written and reports nothing about an unknown
    /// name on top of this. That keeps one mistake to one diagnostic, and lets
    /// this one say what is actually wrong rather than that the name does not
    /// exist — it does exist, a few lines further down, which is the point.
    fn check_field_order(
        &mut self,
        field: &FieldDecl,
        members: &HashMap<String, Symbol>,
        declared: &HashSet<String>,
    ) {
        let Some(initializer) = &field.initializer else {
            return;
        };

        let mut names = Vec::new();
        Self::collect_field_reads(initializer, &mut names);
        for (name, span) in names {
            if declared.contains(&name) {
                continue;
            }
            if name == field.name {
                self.error(
                    span,
                    format!("field `{name}` cannot read itself in its own initializer"),
                );
            } else if members
                .get(&name)
                .is_some_and(|symbol| symbol.function.is_none())
            {
                self.error(
                    span,
                    format!(
                        "field `{}` reads field `{name}`, which is declared below it;                          initializers run in declaration order",
                        field.name
                    ),
                );
            }
        }
    }

    /// Every name an initializer reads that could be one of the container's own
    /// fields: a bare identifier, or `this.<name>`, which mean the same field.
    fn collect_field_reads(expr: &Expr, out: &mut Vec<(String, Span)>) {
        match &expr.kind {
            ExprKind::Identifier(name) => out.push((name.clone(), expr.span)),
            ExprKind::Member { object, field } => {
                if matches!(&object.kind, ExprKind::Identifier(name) if name == "this") {
                    out.push((field.clone(), expr.span));
                } else {
                    Self::collect_field_reads(object, out);
                }
            }
            ExprKind::Unary { expr: inner, .. } | ExprKind::Group(inner) => {
                Self::collect_field_reads(inner, out);
            }
            ExprKind::Binary { left, right, .. } => {
                Self::collect_field_reads(left, out);
                Self::collect_field_reads(right, out);
            }
            ExprKind::Assign { target, value, .. } => {
                Self::collect_field_reads(target, out);
                Self::collect_field_reads(value, out);
            }
            ExprKind::Index { object, index } => {
                Self::collect_field_reads(object, out);
                Self::collect_field_reads(index, out);
            }
            ExprKind::Call { callee, args } => {
                Self::collect_field_reads(callee, out);
                for argument in args {
                    Self::collect_field_reads(argument, out);
                }
            }
            ExprKind::Number(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::Null => {}
        }
    }

    pub(super) fn check_field_initializer(
        &mut self,
        field: &FieldDecl,
        members: &HashMap<String, Symbol>,
    ) {
        let Some(initializer) = &field.initializer else {
            if field.ty.is_none() {
                self.error(
                    field.span,
                    format!("field `{}` needs a type or initializer", field.name),
                );
            }
            return;
        };

        self.scopes = vec![members.clone()];
        let actual = self.expr_type(initializer);
        if let Some(expected) = field.ty.as_ref().map(Type::from_ref) {
            self.check_assignable(&expected, &actual, initializer.span);
        }
        self.scopes.clear();
    }

    pub(super) fn analyze_function(
        &mut self,
        container: &decay_syntax::ContainerDecl,
        function: &FunctionDecl,
        members: &HashMap<String, Symbol>,
    ) {
        self.current_return = function
            .return_type
            .as_ref()
            .map_or(Type::Unit, Type::from_ref);
        self.scopes = vec![members.clone(), HashMap::new()];

        self.define_local(
            "this",
            Symbol {
                ty: Type::Named(container.name.clone()),
                mutable: false,
                function: None,
            },
            function.span,
        );

        // The declared signature, which for a handler is where an unwritten
        // parameter type was filled in from its event.
        let declared = members
            .get(&function.name)
            .and_then(|symbol| symbol.function.as_ref())
            .map(|signature| signature.params.clone())
            .unwrap_or_default();
        for (index, param) in function.params.iter().enumerate() {
            self.define_local(
                &param.name,
                Symbol {
                    ty: declared
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| param.ty.as_ref().map_or(Type::Unknown, Type::from_ref)),
                    mutable: false,
                    function: None,
                },
                param.span,
            );
        }

        self.analyze_block(&function.body, false);
        self.scopes.clear();
        self.current_return = Type::Unit;
    }

    pub(super) fn is_container(&self, name: &str) -> bool {
        self.containers.contains(name)
    }
}

/// The type of a field left unannotated whose initializer builds one of the
/// language's own values: `var fuse = Timer(2.0);`, `var at = Vec2(0.0, 0.0);`.
///
/// Only those, and only when spelled as the construction: their members are
/// answered by the language rather than the host, so a field of unknown type
/// would read `fuse.left` as a host path and fail when it ran. Every other
/// unannotated field stays unknown, as it always was.
fn built_value_type(initializer: Option<&Expr>) -> Type {
    let Some(ExprKind::Call { callee, .. }) = initializer.map(|expr| &expr.kind) else {
        return Type::Unknown;
    };
    match &callee.kind {
        ExprKind::Identifier(name) => match name.as_str() {
            crate::types::VEC2 => Type::Vec2,
            crate::types::VEC3 => Type::Vec3,
            crate::types::TIMER => Type::Timer,
            _ => Type::Unknown,
        },
        _ => Type::Unknown,
    }
}
