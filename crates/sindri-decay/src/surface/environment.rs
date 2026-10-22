//! Live authored environment fields, shared by typing and runtime access.
use super::{Leaf, Node, names::RGBA};

pub(crate) const ENVIRONMENT: &str = "SceneEnvironment";
pub(crate) const FOG: &str = "SceneFog";
pub(crate) const POST_PROCESS: &str = "ScenePostProcess";
pub(crate) const BLOOM: &str = "SceneBloom";
pub(crate) const COMPONENT: &str = "sindri.environment";

#[derive(Clone, Copy, Debug)]
pub(crate) enum EnvironmentField {
    AmbientIntensity,
    AmbientColor(usize),
    Background(usize),
    /// A number one section deep in the payload, such as `fog.density`.
    Number(&'static str, &'static str),
    /// A switch one section deep, such as `bloom.enabled`.
    Flag(&'static str, &'static str),
    FogColor(usize),
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
const fn number(section: &'static str, key: &'static str) -> Node {
    Node::Leaf(Leaf::Environment(EnvironmentField::Number(section, key)))
}
const fn flag(section: &'static str, key: &'static str) -> Node {
    Node::Leaf(Leaf::Environment(EnvironmentField::Flag(section, key)))
}
const fn fog_color(index: usize) -> Node {
    Node::Leaf(Leaf::Environment(EnvironmentField::FogColor(index)))
}
const FOG_COLOR: &[(&str, Node)] = &[
    ("r", fog_color(0)),
    ("g", fog_color(1)),
    ("b", fog_color(2)),
    ("a", fog_color(3)),
];
const FOG_MEMBERS: &[(&str, Node)] = &[
    ("enabled", flag("fog", "enabled")),
    ("color", Node::Group(RGBA, FOG_COLOR)),
    ("start", number("fog", "start")),
    ("distance", number("fog", "distance")),
    ("density", number("fog", "density")),
    ("height", number("fog", "height")),
    ("height_falloff", number("fog", "height_falloff")),
];
const POST_PROCESS_MEMBERS: &[(&str, Node)] = &[
    ("exposure", number("post_process", "exposure")),
    ("contrast", number("post_process", "contrast")),
    ("saturation", number("post_process", "saturation")),
    ("vignette", number("post_process", "vignette")),
];
const BLOOM_MEMBERS: &[(&str, Node)] = &[
    ("enabled", flag("bloom", "enabled")),
    ("threshold", number("bloom", "threshold")),
    ("knee", number("bloom", "knee")),
    ("intensity", number("bloom", "intensity")),
];
pub(crate) const MEMBERS: &[(&str, Node)] = &[
    (
        "ambient_intensity",
        Node::Leaf(Leaf::Environment(EnvironmentField::AmbientIntensity)),
    ),
    ("ambient_color", Node::Group(RGBA, AMBIENT)),
    ("background", Node::Group(RGBA, BACKGROUND)),
    ("fog", Node::Group(FOG, FOG_MEMBERS)),
    (
        "post_process",
        Node::Group(POST_PROCESS, POST_PROCESS_MEMBERS),
    ),
    ("bloom", Node::Group(BLOOM, BLOOM_MEMBERS)),
];
