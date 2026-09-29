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
    /// Each struct's methods, by name, with their signatures.
    methods: BTreeMap<String, BTreeMap<String, decay_semantic::FunctionType>>,
    /// The text of every struct that has methods. A caller links a copy of
    /// each method, as of each shared function, so it must recompile when a
    /// method's body changes and not only its signature; this is part of the
    /// project's key for that.
    method_text: String,
    /// Each struct's fields that declare a default, as written.
    written_defaults: BTreeMap<String, Vec<decay_syntax::StructField>>,
    /// Each struct's field defaults, worked out, by struct and then field.
    defaults: BTreeMap<String, BTreeMap<String, decay_semantic::ConstValue>>,
}

impl Kinds {
    /// Works out every struct's field defaults, which may name the project's
    /// shared constants. One that cannot be worked out is left out; the file
    /// declaring it is told why when it is compiled.
    pub(crate) fn fold_defaults(
        &mut self,
        constants: &BTreeMap<String, decay_semantic::ConstValue>,
        reserved: &Environment,
    ) {
        let known: std::collections::HashMap<_, _> = constants
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        let enums = &self.enums;
        let variants = |name: &str| {
            enums
                .get(name)
                .cloned()
                .or_else(|| reserved.enum_variants(name).map(<[String]>::to_vec))
        };
        for (structure, fields) in &self.written_defaults {
            if !self.structs.contains_key(structure) {
                continue;
            }
            let mut folded = BTreeMap::new();
            for field in fields {
                let Some(value) = &field.default else {
                    continue;
                };
                let declared = decay_syntax::ConstDecl {
                    name: field.name.clone(),
                    shared: false,
                    ty: field.ty.clone(),
                    value: value.clone(),
                    name_span: field.span,
                    span: field.span,
                };
                let (worked, _) =
                    decay_semantic::fold_constants(std::iter::once(&declared), &known, &variants);
                folded.extend(worked);
            }
            self.defaults.insert(structure.clone(), folded);
        }
    }

    /// Reads an `enum` or `struct` item from `source`; anything else is left
    /// alone.
    pub(crate) fn read(&mut self, item: Item, source: &str) {
        if let Item::Struct(declared) = &item
            && !declared.methods.is_empty()
            && let Some(text) = source.get(declared.span.start..declared.span.end)
        {
            self.method_text.push_str(text);
        }
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
                let methods = declared
                    .methods
                    .iter()
                    .map(|method| (method.name.clone(), super::signature_of(method)))
                    .collect();
                self.methods.insert(declared.name.clone(), methods);
                let written: Vec<_> = declared
                    .fields
                    .iter()
                    .filter(|field| field.default.is_some())
                    .cloned()
                    .collect();
                self.written_defaults.insert(declared.name.clone(), written);
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
            for (method, signature) in self.methods.get(name).into_iter().flatten() {
                environment.add_struct_method(name.clone(), method.clone(), signature.clone());
            }
            for (field, value) in self.defaults.get(name).into_iter().flatten() {
                environment.add_struct_default(name.clone(), field.clone(), value.clone());
            }
        }
        for name in &self.ambiguous_structs {
            environment.add_ambiguous_struct(name.clone());
        }
    }
}
