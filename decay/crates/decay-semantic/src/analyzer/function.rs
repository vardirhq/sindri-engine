//! Functions outside any container.
//!
//! `fn half(size: f32) -> f32 { ... }` at the top of a file is a function the
//! file's scripts call by name; `shared fn` makes it every script's in the
//! project, where each used to keep its own copy. Neither has a `this` —
//! there is no script it belongs to — so what it needs, it is passed. A host
//! declares the shared ones other files hold, as Sindri does for a project; a
//! file's own are added to those when it is analysed.

use std::collections::{HashMap, HashSet};

use decay_syntax::{FunctionDecl, Item, Program};

use crate::types::{FunctionType, Type};

use super::{Analyzer, Symbol};

impl Analyzer<'_, '_> {
    /// This program's own shared functions, checked as declarations.
    pub(super) fn collect_functions(&mut self, program: &Program) {
        let mut own = HashSet::new();
        for item in &program.items {
            let Item::Function(function) = item else {
                continue;
            };
            let name = &function.name;
            if !own.insert(name.clone()) {
                self.error(function.span, format!("duplicate declaration `{name}`"));
                continue;
            }
            if self.environment.ambiguous_functions.contains(name) {
                self.error(
                    function.span,
                    format!("`{name}` is declared in more than one file; keep one"),
                );
            }
            let taken_by_host = self.environment.globals.contains_key(name)
                && !self.environment.shared_functions.contains(name);
            if taken_by_host
                || self.is_container(name)
                || self.events.contains_key(name)
                || self.states.contains_key(name)
            {
                self.error(
                    function.span,
                    format!("`{name}` is already a name; a function needs one of its own"),
                );
            }
            let signature = signature_of(function);
            self.functions.insert(name.clone(), signature);
        }
    }

    /// A shared function called by name: this file's own, or one the host
    /// declared from another file.
    pub(super) fn shared_function(
        &mut self,
        name: &str,
        span: decay_syntax::Span,
    ) -> Option<FunctionType> {
        if let Some(function) = self.functions.get(name) {
            return Some(function.clone());
        }
        if self.environment.ambiguous_functions.contains(name) {
            self.error(
                span,
                format!("`{name}` is declared in more than one file; keep one"),
            );
            return Some(FunctionType {
                params: Vec::new(),
                return_type: Type::Unknown,
            });
        }
        None
    }

    /// Checks a shared function's body. There is no container, so no field
    /// and no `this`: only its parameters, the other shared functions, and
    /// what the host offers.
    pub(super) fn analyze_shared_function(&mut self, function: &FunctionDecl) {
        self.binding_scope = function.span;
        self.current_return = function
            .return_type
            .as_ref()
            .map_or(Type::Unit, |ty| self.resolve_type(ty));
        self.scopes = vec![HashMap::new(), HashMap::new()];
        self.in_shared_function = true;
        for param in &function.params {
            let ty = param
                .ty
                .as_ref()
                .map_or(Type::Unknown, |ty| self.resolve_type(ty));
            self.define_local(
                &param.name,
                Symbol {
                    ty,
                    mutable: false,
                    function: None,
                },
                param.span,
            );
        }
        self.analyze_block(&function.body, false);
        self.in_shared_function = false;
        self.scopes.clear();
        self.current_return = Type::Unit;
    }
}

/// A shared function's type, as a call to it is checked against.
pub(crate) fn signature_of(function: &FunctionDecl) -> FunctionType {
    FunctionType {
        params: function
            .params
            .iter()
            .map(|param| param.ty.as_ref().map_or(Type::Unknown, Type::from_ref))
            .collect(),
        return_type: function
            .return_type
            .as_ref()
            .map_or(Type::Unit, Type::from_ref),
    }
}
