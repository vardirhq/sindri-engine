//! The value types a project declares — enums and structs — read from every
//! file so each file may name the others'.

use std::collections::{BTreeMap, BTreeSet};

use decay_semantic::{Environment, Type};
use decay_syntax::Item;

/// Every enum and struct a project declares, and the names declared twice.
#[derive(Clone, Debug, Default)]
pub(crate) struct Kinds {
    /// Enums by name, with their variants in order.
    pub(crate) enums: BTreeMap<String, Vec<String>>,
    /// Enums declared more than once, which nothing may name.
    ambiguous_enums: BTreeSet<String>,
    /// Structs by name, with their fields in order.
    structs: BTreeMap<String, Vec<(String, Type)>>,
    /// Structs declared more than once, which nothing may name.
    ambiguous_structs: BTreeSet<String>,
}

impl Kinds {
    /// Reads an `enum` or `struct` item; anything else is left alone.
    pub(crate) fn read(&mut self, item: Item) {
        match item {
            Item::Enum(declared) => {
                let variants = declared.variants.into_iter().map(|(name, _)| name);
                if self
                    .enums
                    .insert(declared.name.clone(), variants.collect())
                    .is_some()
                {
                    self.ambiguous_enums.insert(declared.name);
                }
            }
            Item::Struct(declared) => {
                let fields = declared
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), Type::from_ref(&field.ty)))
                    .collect();
                if self.structs.insert(declared.name.clone(), fields).is_some() {
                    self.ambiguous_structs.insert(declared.name);
                }
            }
            _ => {}
        }
    }

    /// Drops the ones declared twice, and the ones whose name something else
    /// already has; the file declaring one is told so by the analyzer.
    pub(crate) fn keep(&mut self, taken: impl Fn(&str) -> bool) {
        let ambiguous = &self.ambiguous_enums;
        self.enums
            .retain(|name, _| !ambiguous.contains(name) && !taken(name));
        let enums = &self.enums;
        let ambiguous = &self.ambiguous_structs;
        self.structs.retain(|name, _| {
            !ambiguous.contains(name) && !enums.contains_key(name) && !taken(name)
        });
    }

    /// Describes them all to a file's environment.
    pub(crate) fn describe(&self, environment: &mut Environment) {
        for (name, variants) in &self.enums {
            environment.add_enum(name.clone(), variants.clone());
        }
        for name in &self.ambiguous_enums {
            environment.add_ambiguous_enum(name.clone());
        }
        for (name, fields) in &self.structs {
            environment.add_struct(name.clone(), fields.clone());
        }
        for name in &self.ambiguous_structs {
            environment.add_ambiguous_struct(name.clone());
        }
    }
}
