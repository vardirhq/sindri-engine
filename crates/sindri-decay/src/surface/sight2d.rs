//! What a script reaches on a 2D viewer: how far it sees and how dim the unseen goes.

use super::Node;
use super::names::SIGHT_2D_COMPONENT;
use super::{Leaf, Seg};

const fn number(field: &'static str) -> Node {
    Node::Leaf(Leaf::Component {
        component: SIGHT_2D_COMPONENT,
        pointer: match field.as_bytes() {
            b"radius" => &[Seg::Field("radius")],
            b"unseen" => &[Seg::Field("unseen")],
            _ => &[Seg::Field("reach")],
        },
    })
}

pub(crate) const SIGHT_2D_MEMBERS: &[(&str, Node)] = &[
    ("radius", number("radius")),
    ("unseen", number("unseen")),
    ("reach", number("reach")),
];
