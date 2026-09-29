//! A struct's methods: `fn`s declared after its fields, asked of one value as
//! `card.heavier(best)`, with `this` the value.
//!
//! A method is a function that takes the value first, lowered to one named
//! [`method_function`] and linked like a shared function, so the runtime calls
//! it as it calls any other.

use std::collections::HashMap;

use decay_syntax::{Expr, Item, Program, Span, StructDecl};

use crate::codes::Code;
use crate::types::{FunctionType, Type, method_function};

use super::Analyzer;
use super::function::signature_of;

impl Analyzer<'_, '_> {
    /// Every struct's methods: the host's, and this program's own.
    pub(super) fn collect_methods(&mut self, program: &Program) {
        self.methods = self.environment.struct_methods.clone();
        let mut seen = std::collections::HashSet::new();
        for item in &program.items {
            let Item::Struct(declared) = item else {
                continue;
            };
            // A struct declared twice was reported already; its first
            // declaration is the one used.
            if !seen.insert(declared.name.clone()) {
                continue;
            }
            let mut own = HashMap::new();
            for method in &declared.methods {
                let name = &method.name;
                if declared.fields.iter().any(|field| &field.name == name) {
                    self.error(
                        Code::Duplicate,
                        method.span,
                        format!("`{}.{name}` is both a field and a method", declared.name),
                    );
                    continue;
                }
                if own.insert(name.clone(), signature_of(method)).is_some() {
                    self.error(
                        Code::Duplicate,
                        method.span,
                        format!("`{}.{name}` is declared twice", declared.name),
                    );
                }
            }
            self.methods.insert(declared.name.clone(), own);
        }
    }

    /// Checks each method's body, with `this` the struct's value.
    pub(super) fn analyze_methods(&mut self, declared: &StructDecl) {
        for method in &declared.methods {
            self.analyze_free_function(method, Some(&declared.name));
        }
    }

    /// `card.heavier(best)`: the method's result, having checked the
    /// arguments. `None` when the value is not a struct with that method.
    pub(super) fn method_call_type(
        &mut self,
        object_type: &Type,
        field: &str,
        args: &[Expr],
        span: Span,
    ) -> Option<Type> {
        let Type::Named(structure) = object_type else {
            return None;
        };
        if !self.structs.contains_key(structure) {
            return None;
        }
        let Some(signature) = self
            .methods
            .get(structure)
            .and_then(|methods| methods.get(field))
            .cloned()
        else {
            let message = if self.structs[structure]
                .iter()
                .any(|(known, _)| known == field)
            {
                format!("`{field}` on `{structure}` is a field, not a method")
            } else {
                let mut methods: Vec<_> = self
                    .methods
                    .get(structure)
                    .map(|methods| methods.keys().map(|name| format!("`{name}`")).collect())
                    .unwrap_or_default();
                methods.sort();
                if methods.is_empty() {
                    format!("`{structure}` has no methods, so no `{field}`")
                } else {
                    format!(
                        "`{structure}` has no method `{field}`; it has {}",
                        methods.join(", ")
                    )
                }
            };
            self.error(Code::UnknownMember, span, message);
            for argument in args {
                self.expr_type(argument);
            }
            return Some(Type::Unknown);
        };
        self.check_call(&signature, args, span);
        self.method_calls
            .insert(span, method_function(structure, field));
        Some(signature.return_type)
    }

    /// Whether a struct has a method of this name, for a read that forgot to
    /// call it.
    pub(super) fn has_method(&self, structure: &str, name: &str) -> bool {
        self.methods
            .get(structure)
            .is_some_and(|methods| methods.contains_key(name))
    }

    /// Every struct method this program could call, for the analysis to hand
    /// on, sorted so it reads the same each time.
    pub(crate) fn known_methods(
        &self,
    ) -> std::collections::BTreeMap<String, Vec<(String, FunctionType)>> {
        self.methods
            .iter()
            .map(|(structure, methods)| {
                let mut methods: Vec<_> = methods
                    .iter()
                    .map(|(name, signature)| (name.clone(), signature.clone()))
                    .collect();
                methods.sort_by(|(a, _), (b, _)| a.cmp(b));
                (structure.clone(), methods)
            })
            .collect()
    }
}
