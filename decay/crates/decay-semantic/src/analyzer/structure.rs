//! Structs: a value made of named, typed fields.
//!
//! `struct Card { root: Entity, name: String }` declares one;
//! `Card(root: e, name: "Arc")` builds one, naming every field; `card.name`
//! reads one field and `card.name = "Nova"` writes it. A struct is a value,
//! like a vector: assigning one copies it, so a field is written through the
//! variable or field that holds it, and the lowering reaches a field by its
//! position exactly as it reaches a vector's component.

use crate::codes::Code;
use std::collections::{HashMap, HashSet};

use decay_syntax::{Expr, Item, Program, Span, StructDecl};

use crate::diagnostic::ValueMember;
use crate::types::Type;

use super::Analyzer;

impl Analyzer<'_, '_> {
    /// Adds this program's own structs to the host's, checking each.
    pub(super) fn collect_structs(&mut self, program: &Program) {
        self.structs = self.environment.structs.clone();
        let mut own = HashSet::new();
        // Names first, so a field may be of a struct declared further down.
        for item in &program.items {
            if let Item::Struct(declared) = item
                && own.insert(declared.name.clone())
            {
                self.structs.insert(declared.name.clone(), Vec::new());
            }
        }
        let mut seen = HashSet::new();
        for item in &program.items {
            let Item::Struct(declared) = item else {
                continue;
            };
            if !seen.insert(declared.name.clone()) {
                self.error(
                    Code::Duplicate,
                    declared.span,
                    format!("duplicate declaration `{}`", declared.name),
                );
                continue;
            }
            let fields = self.check_struct(declared);
            self.structs.insert(declared.name.clone(), fields);
        }
    }

    fn check_struct(&mut self, declared: &StructDecl) -> Vec<(String, Type)> {
        let name = &declared.name;
        if self.environment.ambiguous_structs.contains(name) {
            self.error(
                Code::DeclaredInSeveralFiles,
                declared.span,
                format!("struct `{name}` is declared in more than one file; keep one"),
            );
        }
        if self.is_container(name)
            || self.enums.contains_key(name)
            || self.environment.globals.contains_key(name)
            || self.environment.get_type(name).is_some()
        {
            self.error(
                Code::NameTaken,
                declared.span,
                format!("`{name}` is already a name; a struct needs one of its own"),
            );
        }
        if declared.fields.is_empty() {
            self.error(
                Code::EmptyDeclaration,
                declared.span,
                format!("struct `{name}` has no fields, so it could never hold anything"),
            );
        }
        let mut seen = HashSet::new();
        let mut fields = Vec::new();
        for field in &declared.fields {
            if !seen.insert(field.name.clone()) {
                self.error(
                    Code::Duplicate,
                    field.span,
                    format!("`{name}.{}` is declared twice", field.name),
                );
                continue;
            }
            let ty = self.resolve_type(&field.ty);
            fields.push((field.name.clone(), ty));
        }
        fields
    }

    /// Every struct this program could name, with its fields in order, for
    /// the analysis to hand on.
    pub(crate) fn known_structs(&self) -> std::collections::BTreeMap<String, Vec<(String, Type)>> {
        self.structs
            .iter()
            .map(|(name, fields)| (name.clone(), fields.clone()))
            .collect()
    }

    /// The variable, parameter or field of this script that `expr` is, or is
    /// a field or component inside — `card`, `this.hand`, `hand.best.at` —
    /// named as written, with whether it may change. `None` for anything
    /// else: a computed value, a host path, a list's element.
    pub(super) fn place_root(&self, expr: &Expr) -> Option<(String, bool)> {
        match &expr.kind {
            decay_syntax::ExprKind::Group(inner) => self.place_root(inner),
            decay_syntax::ExprKind::Identifier(name) => self
                .lookup(name)
                .map(|symbol| (name.clone(), symbol.mutable)),
            decay_syntax::ExprKind::Member { object, field } => {
                if matches!(&object.kind, decay_syntax::ExprKind::Identifier(root) if root == "this")
                {
                    return self
                        .own_field(field)
                        .map(|symbol| (format!("this.{field}"), symbol.mutable));
                }
                if matches!(
                    self.value_members.get(&expr.span),
                    Some(ValueMember::Component(_))
                ) {
                    return self.place_root(object);
                }
                None
            }
            _ => None,
        }
    }

    /// Whether this type is a struct's.
    pub(super) fn is_struct(&self, ty: &Type) -> bool {
        matches!(ty, Type::Named(name) if self.structs.contains_key(name))
    }

    /// `Card(root: e, name: "Arc")`: every field named once, each value
    /// fitting its field.
    pub(super) fn construct_struct_type(
        &mut self,
        name: &str,
        fields: &[(String, Span, Expr)],
        span: Span,
    ) -> Type {
        let Some(declared) = self.structs.get(name).cloned() else {
            for (_, _, value) in fields {
                self.expr_type(value);
            }
            let (code, message) = if self.environment.ambiguous_structs.contains(name) {
                (
                    Code::DeclaredInSeveralFiles,
                    format!("struct `{name}` is declared in more than one file; keep one"),
                )
            } else {
                (
                    Code::NotAStruct,
                    format!("`{name}` is not a struct, so it is not built with named fields"),
                )
            };
            self.error(code, span, message);
            return Type::Unknown;
        };
        let expected: HashMap<&str, &Type> = declared
            .iter()
            .map(|(field, ty)| (field.as_str(), ty))
            .collect();
        let mut given = HashSet::new();
        for (field, field_span, value) in fields {
            let actual = self.expr_type(value);
            let Some(wanted) = expected.get(field.as_str()) else {
                self.error(
                    Code::UnknownMember,
                    *field_span,
                    missing_field(name, field, &declared),
                );
                continue;
            };
            if !given.insert(field.as_str()) {
                self.error(
                    Code::Duplicate,
                    *field_span,
                    format!("`{field}` is given twice"),
                );
                continue;
            }
            self.check_assignable(wanted, &actual, value.span);
        }
        let missing: Vec<String> = declared
            .iter()
            .filter(|(field, _)| !given.contains(field.as_str()))
            .map(|(field, _)| format!("`{field}`"))
            .collect();
        if !missing.is_empty() {
            self.error(
                Code::MissingFields,
                span,
                format!("`{name}` needs every field: missing {}", missing.join(", ")),
            );
        }
        Type::Named(name.to_owned())
    }

    /// `card.name`: the field's type, noting its position for the lowering.
    /// `None` when the object is not a struct.
    pub(super) fn struct_field_type(
        &mut self,
        object_type: &Type,
        field: &str,
        span: Span,
    ) -> Option<Type> {
        let Type::Named(name) = object_type else {
            return None;
        };
        let declared = self.structs.get(name)?;
        if let Some(index) = declared.iter().position(|(known, _)| known == field) {
            let ty = declared[index].1.clone();
            self.value_members
                .insert(span, ValueMember::Component(index));
            return Some(ty);
        }
        if self.has_method(name, field) {
            self.error(
                Code::FunctionNotCalled,
                span,
                format!("`{field}` is a method of `{name}` -- call it: `.{field}(...)`"),
            );
            return Some(Type::Unknown);
        }
        let message = missing_field(name, field, declared);
        self.error(Code::UnknownMember, span, message);
        Some(Type::Unknown)
    }
}

fn missing_field(name: &str, field: &str, declared: &[(String, Type)]) -> String {
    let fields = declared
        .iter()
        .map(|(known, _)| format!("`{known}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("`{name}` has no field `{field}`; it has {fields}")
}
