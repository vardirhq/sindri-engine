//! A program's own constants, and a name that means one.

use std::collections::{HashMap, HashSet};

use decay_syntax::{ConstDecl, Item, Program, Span};

use crate::codes::Code;
use crate::constant::fold_constants;
use crate::types::Type;

use super::Analyzer;

impl Analyzer<'_, '_> {
    /// Works out this file's constants, on top of the host's shared ones.
    ///
    /// Collected before any body is walked, like enums and functions, so a
    /// constant declared at the bottom of a file is usable at the top.
    pub(super) fn collect_constants(&mut self, program: &Program) {
        let mut own = HashSet::new();
        let mut declarations: Vec<&ConstDecl> = Vec::new();
        for item in &program.items {
            let Item::Const(declared) = item else {
                continue;
            };
            let name = &declared.name;
            if !own.insert(name.clone()) {
                self.error(
                    Code::Duplicate,
                    declared.name_span,
                    format!("duplicate declaration `{name}`"),
                );
                continue;
            }
            if declared.shared && self.environment.ambiguous_constants.contains(name) {
                self.error(
                    Code::DeclaredInSeveralFiles,
                    declared.name_span,
                    format!("`{name}` is declared in more than one file; keep one"),
                );
            }
            if self.name_is_taken_for_constant(name) {
                self.error(
                    Code::NameTaken,
                    declared.name_span,
                    format!("`{name}` is already a name; a constant needs one of its own"),
                );
            }
            declarations.push(declared);
        }

        let known: HashMap<_, _> = self
            .environment
            .constants
            .iter()
            .filter(|(name, _)| !own.contains(*name))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        let enums = &self.enums;
        let variants = |name: &str| enums.get(name).cloned();
        let (folded, errors) = fold_constants(declarations, &known, &variants);
        for error in errors {
            self.error(error.code, error.span, error.message);
        }
        self.constants = known;
        self.constants.extend(
            folded
                .iter()
                .map(|(name, value)| (name.clone(), value.clone())),
        );
        self.broken_constants = own
            .into_iter()
            .filter(|name| !folded.contains_key(name))
            .collect();
        self.own_constants = folded;
    }

    /// Whether a constant of this name would hide something else a script
    /// can name, which would make the name mean one thing here and another
    /// everywhere else.
    fn name_is_taken_for_constant(&self, name: &str) -> bool {
        self.environment.globals.contains_key(name)
            || self.is_container(name)
            || self.events.contains_key(name)
            || self.states.contains_key(name)
            || self.enums.contains_key(name)
            || self.structs.contains_key(name)
            || self.functions.contains_key(name)
    }

    /// A name that means a constant: its type, noting the value for the
    /// lowering. `None` when it is no constant's name.
    pub(super) fn constant_type(&mut self, name: &str, span: Span) -> Option<Type> {
        if let Some(value) = self.constants.get(name) {
            let ty = value.ty();
            self.constant_uses.insert(span, value.clone());
            return Some(ty);
        }
        // Its declaration was reported already; every use of it is not
        // another mistake.
        if self.broken_constants.contains(name) {
            return Some(Type::Unknown);
        }
        if self.environment.ambiguous_constants.contains(name) {
            self.error(
                Code::DeclaredInSeveralFiles,
                span,
                format!("constant `{name}` is declared in more than one file; keep one"),
            );
            return Some(Type::Unknown);
        }
        None
    }
}
