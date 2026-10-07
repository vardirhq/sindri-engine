//! A prefab placed in a scene: the reference, and what the scene changed.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{PrefabDocument, SceneEntityId, Transform3D};

/// What a scene entity says when it is an instance of a prefab.
///
/// The prefab is the source of everything the instance is made of; the scene
/// keeps only where the instance stands and what it changed. So an edit to the
/// prefab reaches every instance the next time the scene is opened, except
/// where an instance said otherwise.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrefabInstance {
    /// The prefab's asset ID, resolved like every other asset reference.
    pub source: String,
    /// What this instance changed, keyed by the prefab's own entity IDs.
    ///
    /// A key names an entity *inside the prefab* — its root included — and
    /// a nested instance's entities by their path, `turret/barrel`. A key the
    /// prefab no longer has is kept until the scene is saved again, and then
    /// dropped with the entity it named.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub overrides: BTreeMap<SceneEntityId, EntityOverride>,
    /// The prefab's entities this instance does without, by the same keys.
    ///
    /// Removing one removes everything under it too. The root cannot be
    /// removed: an instance without its root is not an instance.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub removed: BTreeSet<SceneEntityId>,
}

impl PrefabInstance {
    #[must_use]
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            overrides: BTreeMap::new(),
            removed: BTreeSet::new(),
        }
    }

    /// Whether this instance says nothing but which prefab it is.
    #[must_use]
    pub fn is_unchanged(&self) -> bool {
        self.overrides.is_empty() && self.removed.is_empty()
    }
}

/// What an instance changed about one of its prefab's entities.
///
/// Every field says nothing unless it is set. Components are merge patches
/// (see [`crate::apply_merge_patch`]): a component the prefab has is patched
/// field by field, one it lacks is added, and `null` removes one it has.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform_3d: Option<Transform3D>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub components: BTreeMap<String, Value>,
    /// Editor-only state of this entity in this instance — where it sits
    /// among its siblings, say — which runtimes ignore as they ignore an
    /// entity's own. Written whole, and only when there is some.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub editor: BTreeMap<String, Value>,
}

impl EntityOverride {
    /// Whether this override changes nothing, and so is not worth writing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.transform_3d.is_none()
            && self.disabled.is_none()
            && self.components.is_empty()
            && self.editor.is_empty()
    }
}

/// Which prefab an entity in a world was made from.
///
/// Carried by every entity an instance expanded into, so the editor can say
/// what an entity is an instance of, write the instance back as a reference,
/// and bring it up to date when the prefab changes. It names the outermost
/// instance only: an entity of a prefab nested inside another is part of the
/// outer instance, at a longer path, and the nesting is the prefab's business.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrefabLink {
    /// The prefab's asset ID.
    pub source: String,
    /// The entity's ID inside the expanded prefab.
    pub path: SceneEntityId,
    /// Whether this is the instance's root, which is the entity the scene
    /// writes the instance as.
    pub root: bool,
    /// Original root aliases in the containing scene's namespace.
    /// Runtime expansion metadata, omitted from serialized links.
    #[serde(skip)]
    pub aliases: BTreeSet<SceneEntityId>,
}

/// Where prefab documents come from when an instance names one.
///
/// A trait because every host already holds its prefabs somewhere of its own
/// — a browser build's decoded assets, the editor's loaded project, a test's
/// map — and core does no I/O to fetch one.
pub trait PrefabLibrary {
    fn prefab(&self, source: &str) -> Option<&PrefabDocument>;
}

/// A library with nothing in it, for a world that holds no instances.
///
/// Loading a scene that names a prefab through it fails, saying which prefab
/// was missing, rather than loading the instance as nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoPrefabs;

impl PrefabLibrary for NoPrefabs {
    fn prefab(&self, _source: &str) -> Option<&PrefabDocument> {
        None
    }
}

impl PrefabLibrary for BTreeMap<String, PrefabDocument> {
    fn prefab(&self, source: &str) -> Option<&PrefabDocument> {
        self.get(source)
    }
}

impl<S: std::hash::BuildHasher> PrefabLibrary for HashMap<String, PrefabDocument, S> {
    fn prefab(&self, source: &str) -> Option<&PrefabDocument> {
        self.get(source)
    }
}

impl<T: PrefabLibrary + ?Sized> PrefabLibrary for &T {
    fn prefab(&self, source: &str) -> Option<&PrefabDocument> {
        (**self).prefab(source)
    }
}
