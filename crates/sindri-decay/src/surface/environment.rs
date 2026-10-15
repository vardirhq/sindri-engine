//! Live authored environment fields, shared by typing and runtime access.
use super::{Leaf, Node, names::RGBA};

pub(crate) const ENVIRONMENT: &str = "SceneEnvironment";
pub(crate) const COMPONENT: &str = "sindri.environment";

#[derive(Clone, Copy, Debug)]
pub(crate) enum EnvironmentField {
    AmbientIntensity,
    AmbientColor(usize),
    Background(usize),
}

const fn ambient(index: usize) -> Node {
    Node::Leaf(Leaf::Environment(EnvironmentField::AmbientColor(index)))
}
const fn background(index: usize) -> Node {
    Node::Leaf(Leaf::Environment(EnvironmentField::Background(index)))
}
const AMBIENT: &[(&str, Node)] = &[
    ("r", ambient(0)),
    ("g", ambient(1)),
    ("b", ambient(2)),
    ("a", ambient(3)),
];
const BACKGROUND: &[(&str, Node)] = &[
    ("r", background(0)),
    ("g", background(1)),
    ("b", background(2)),
    ("a", background(3)),
];
pub(crate) const MEMBERS: &[(&str, Node)] = &[
    (
        "ambient_intensity",
        Node::Leaf(Leaf::Environment(EnvironmentField::AmbientIntensity)),
    ),
    ("ambient_color", Node::Group(RGBA, AMBIENT)),
    ("background", Node::Group(RGBA, BACKGROUND)),
];
