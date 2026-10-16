//! What a script reaches on world text (`this.world_text`, not `text`, which
//! is a field name scripts already use): its color, how big it is, how many of
//! its letters show, and which layer it is drawn in. The words themselves are
//! `Ui.set_text` and `Ui.set_number`, as for a HUD label.

use super::names::TEXT_COMPONENT;
use super::{Leaf, Node, Seg, names::RGBA};

const fn channel(index: usize) -> Node {
    Node::Leaf(Leaf::Component {
        component: TEXT_COMPONENT,
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
        component: TEXT_COMPONENT,
        pointer: match field.as_bytes() {
            b"font_size" => &[Seg::Field("font_size")],
            b"visible" => &[Seg::Field("visible")],
            _ => &[Seg::Field("layer")],
        },
    })
}

pub(crate) const TEXT_MEMBERS: &[(&str, Node)] = &[
    ("color", Node::Group(RGBA, COLOR)),
    ("font_size", number("font_size")),
    ("visible", number("visible")),
    ("layer", number("layer")),
];
