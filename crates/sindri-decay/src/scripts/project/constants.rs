//! A project's shared constants, read from every file and worked out
//! together, since one may name another file's.

use std::collections::{BTreeMap, BTreeSet};

use decay_semantic::Environment;

/// Every file's `shared const` declarations, as they are read.
#[derive(Default)]
pub(super) struct SharedConstants {
    declared: BTreeMap<String, decay_syntax::ConstDecl>,
    ambiguous: BTreeSet<String>,
}

impl SharedConstants {
    /// Keeps a shared constant; like a function, a plain `const` is its own
    /// file's and not the project's.
    pub(super) fn read(&mut self, constant: decay_syntax::ConstDecl) {
        if !constant.shared {
            return;
        }
        let name = constant.name.clone();
        if self.declared.insert(name.clone(), constant).is_some() {
            self.ambiguous.insert(name);
        }
    }

    /// Works out every file's shared constants together, since one may name
    /// another's. One declared twice, or whose name is an enum's or `taken`
    /// by something else, is left out; one that cannot be worked out is too, and the file
    /// declaring it is told why when it is compiled. Returns the constants and
    /// the names declared twice.
    ///
    /// Then works out every struct's field defaults, which may name these.
    pub(super) fn fold(
        self,
        kinds: &mut super::kinds::Kinds,
        reserved: &Environment,
        taken: impl Fn(&str) -> bool,
    ) -> (
        BTreeMap<String, decay_semantic::ConstValue>,
        BTreeSet<String>,
    ) {
        let folded = self.fold_constants(kinds, reserved, taken);
        kinds.fold_defaults(&folded, reserved);
        (folded, self.ambiguous)
    }

    fn fold_constants(
        &self,
        kinds: &super::kinds::Kinds,
        reserved: &Environment,
        taken: impl Fn(&str) -> bool,
    ) -> BTreeMap<String, decay_semantic::ConstValue> {
        let usable = self.declared.values().filter(|constant| {
            !self.ambiguous.contains(&constant.name)
                && !kinds.enums.contains_key(&constant.name)
                && !taken(&constant.name)
        });
        let variants = |name: &str| {
            kinds
                .enums
                .get(name)
                .cloned()
                .or_else(|| reserved.enum_variants(name).map(<[String]>::to_vec))
        };
        let known = std::collections::HashMap::new();
        decay_semantic::fold_constants(usable, &known, &variants).0
    }
}
