//! Contact fields shared by the typed environment and runtime snapshots.

pub(crate) const CONTACT: &str = "Contact2d";
pub(crate) const CONTACT_FIELDS: [&str; 8] = [
    "entity",
    "point",
    "normal",
    "normal_impulse",
    "tangent_impulse",
    "force",
    // Which piece of this body's collider touches, and of the other's, by
    // name; `""` for a piece with none (`docs/object-parts.md`).
    "piece",
    "other_piece",
];
