//! Checked 3D light properties shared by typing, runtime and generated docs.
use super::{Leaf, Node, names::RGBA};

pub(crate) const LIGHT: &str = "Light";
pub(crate) const COMPONENT: &str = "sindri.light";

#[derive(Clone, Copy, Debug)]
pub(crate) enum LightField {
    Color(usize),
    Intensity,
    Range,
    InnerAngle,
    OuterAngle,
    Enabled,
}

const fn field(field: LightField) -> Node {
    Node::Leaf(Leaf::Light(field))
}
const COLOR: &[(&str, Node)] = &[
    ("r", field(LightField::Color(0))),
    ("g", field(LightField::Color(1))),
    ("b", field(LightField::Color(2))),
    ("a", field(LightField::Color(3))),
];
pub(crate) const MEMBERS: &[(&str, Node)] = &[
    ("color", Node::Group(RGBA, COLOR)),
    ("intensity", field(LightField::Intensity)),
    ("range", field(LightField::Range)),
    ("inner_angle", field(LightField::InnerAngle)),
    ("outer_angle", field(LightField::OuterAngle)),
    ("enabled", field(LightField::Enabled)),
];
