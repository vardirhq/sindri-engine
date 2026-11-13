//! Which script on an object: its own (`sindri.script`) or a named part
//! (`sindri.script#halo`).
//!
//! An object can run several scripts, each attached as a named part
//! (`docs/object-parts.md`). Everything that belongs to one running script —
//! its instance, the values it is to start with, its tweens, the messages sent
//! to it — is kept by slot rather than by entity, so two scripts on one lantern
//! are two scripts.

use sindri_core::EntityId;

/// One script's place: the entity and, for a script attached as a part, the
/// part's name.
///
/// Ordered by entity and then by part, the unnamed one first, so a pass runs
/// an object's own script before its parts, and its parts by name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptSlot {
    pub entity: EntityId,
    pub part: Option<String>,
}

impl ScriptSlot {
    /// The entity's own script.
    #[must_use]
    pub const fn own(entity: EntityId) -> Self {
        Self { entity, part: None }
    }

    #[must_use]
    pub const fn new(entity: EntityId, part: Option<String>) -> Self {
        Self { entity, part }
    }
}
