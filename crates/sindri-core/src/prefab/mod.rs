//! The authored reusable entity definition.
//!
//! A prefab says what to create. Until one existed a script could find, reach
//! through, check, and remove another entity but could not make one, because
//! "make an entity" has no meaning without a description of what to make —
//! `docs/scripting.md` recorded that as the reason spawning stopped where it
//! did.
//!
//! It is a scene fragment and it is deliberately not a second document format.
//! The same entity shape, the same component payloads, the same identities,
//! the same migration, the same canonical serialization, the same validation:
//! the only rule a prefab adds is that it has exactly one root. A separate
//! format for "a subtree of entities" would be a copy of the scene format that
//! drifts from it, and a prefab that could not hold a component a scene can
//! hold would be a trap discovered late.
//!
//! A scene places one as an *instance*: a reference to the prefab and the
//! overrides that instance made. `expand` turns instances into the entities
//! their prefabs describe, nested instances included, and computes an
//! instance's overrides back from what it has become.

mod document;
mod expand;
mod instance;
mod patch;

#[cfg(test)]
mod tests;

pub use document::{
    PREFAB_FORMAT_VERSION, PREFAB_SUFFIX, PrefabDocument, PrefabError, PrefabJsonError,
};
pub use expand::{
    ExpandedEntity, MAX_PREFAB_NESTING, apply_override, expand_entities, instance_path,
    override_between,
};
pub use instance::{EntityOverride, NoPrefabs, PrefabInstance, PrefabLibrary, PrefabLink};
pub use patch::{LIST_ITEMS, LIST_LENGTH, apply_merge_patch, merge_patch_between};
