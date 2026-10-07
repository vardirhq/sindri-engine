//! Contact fields shared by the typed environment and runtime snapshots.

pub(crate) const CONTACT: &str = "Contact2d";
pub(crate) const CONTACT_FIELDS: [&str; 6] = [
    "entity",
    "point",
    "normal",
    "normal_impulse",
    "tangent_impulse",
    "force",
];
