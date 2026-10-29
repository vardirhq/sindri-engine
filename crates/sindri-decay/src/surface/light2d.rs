//! What a script reaches on a 2D light: its color, how bright and how far it
//! reaches, how it fades, how high it hangs, and the beam of a spot light. Switching one off is the
//! entity's own `set_active`, as for anything else in the world.

use super::names::LIGHT_2D_COMPONENT;
use super::{Leaf, Node, Seg, names::RGBA};

const fn channel(index: usize) -> Node {
    Node::Leaf(Leaf::Component {
        component: LIGHT_2D_COMPONENT,
        pointer: match index {
            0 => &[Seg::Field("color"), Seg::Index(0)],
            1 => &[Seg::Field("color"), Seg::Index(1)],
            2 => &[Seg::Field("color"), Seg::Index(2)],
            _ => &[Seg::Field("color"), Seg::Index(3)],
        },
    })
}

const COLOR: &[(&str, Node)] = &[
    ("r", channel(0)),
    ("g", channel(1)),
    ("b", channel(2)),
    ("a", channel(3)),
];

const fn number(field: &'static str) -> Node {
    Node::Leaf(Leaf::Component {
        component: LIGHT_2D_COMPONENT,
        pointer: match field.as_bytes() {
            b"intensity" => &[Seg::Field("intensity")],
            b"radius" => &[Seg::Field("radius")],
            b"falloff" => &[Seg::Field("falloff")],
            b"cone_angle" => &[Seg::Field("cone_angle")],
            b"cone_direction" => &[Seg::Field("cone_direction")],
            b"cone_softness" => &[Seg::Field("cone_softness")],
            _ => &[Seg::Field("height")],
        },
    })
}

pub(crate) const LIGHT_2D_MEMBERS: &[(&str, Node)] = &[
    ("color", Node::Group(RGBA, COLOR)),
    ("intensity", number("intensity")),
    ("radius", number("radius")),
    ("falloff", number("falloff")),
    ("height", number("height")),
    ("cone_angle", number("cone_angle")),
    ("cone_direction", number("cone_direction")),
    ("cone_softness", number("cone_softness")),
];
