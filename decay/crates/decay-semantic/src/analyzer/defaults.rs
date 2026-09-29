//! A struct field's default: `weight: f32 = 1.0`, what the field holds when a
//! struct is built without naming it.
//!
//! Worked out when the file compiles, as a constant is and by the same
//! folding, so a default may name a constant or an enum's variant and costs
//! what the literal would where it is used.

use std::collections::HashMap;

use decay_syntax::{ConstDecl, Item, Program};

use crate::constant::{ConstValue, fold_constants};

use super::Analyzer;

impl Analyzer<'_, '_> {
    /// Works out this program's own struct defaults, on top of the host's.
    ///
    /// After constants, which a default may name.
    pub(super) fn collect_struct_defaults(&mut self, program: &Program) {
        self.defaults = self.environment.struct_defaults.clone();
        let mut seen = std::collections::HashSet::new();
        for item in &program.items {
            let Item::Struct(declared) = item else {
                continue;
            };
            if !seen.insert(declared.name.clone()) {
                continue;
            }
            let written: Vec<ConstDecl> = declared
                .fields
                .iter()
                .filter_map(|field| {
                    Some(ConstDecl {
                        name: field.name.clone(),
                        shared: false,
                        ty: field.ty.clone(),
                        value: field.default.clone()?,
                        name_span: field.span,
                        span: field.span,
                    })
                })
                .collect();
            if written.is_empty() {
                continue;
            }
            // A field's default may name a constant, never another field: each
            // one is worked out on its own, against the constants alone.
            let mut own = HashMap::new();
            for default in &written {
                let enums = &self.enums;
                let variants = |name: &str| enums.get(name).cloned();
                let (folded, errors) =
                    fold_constants(std::iter::once(default), &self.constants, &variants);
                for error in errors {
                    let message = error
                        .message
                        .replace("a constant holds", "a field's default holds")
                        .replace("a constant's value", "a field's default")
                        .replace("constant `", "field `");
                    self.error(
                        error.code,
                        error.span,
                        format!("`{}.{}`: {message}", declared.name, default.name),
                    );
                }
                if let Some(value) = folded.get(&default.name) {
                    own.insert(default.name.clone(), value.clone());
                } else {
                    // Reported above; a construction leaving it out is not a
                    // second mistake.
                    self.broken_defaults
                        .insert((declared.name.clone(), default.name.clone()));
                }
            }
            self.defaults.insert(declared.name.clone(), own);
        }
    }

    /// Whether a struct's field has a default, so building one may leave it
    /// out.
    pub(super) fn has_default(&self, structure: &str, field: &str) -> bool {
        self.defaults
            .get(structure)
            .is_some_and(|defaults| defaults.contains_key(field))
            || self
                .broken_defaults
                .contains(&(structure.to_owned(), field.to_owned()))
    }

    /// Every struct default this program could use, for the analysis to hand
    /// on to the lowering.
    pub(crate) fn known_defaults(
        &self,
    ) -> std::collections::BTreeMap<String, std::collections::BTreeMap<String, ConstValue>> {
        self.defaults
            .iter()
            .map(|(structure, defaults)| {
                (
                    structure.clone(),
                    defaults
                        .iter()
                        .map(|(field, value)| (field.clone(), value.clone()))
                        .collect(),
                )
            })
            .collect()
    }
}
