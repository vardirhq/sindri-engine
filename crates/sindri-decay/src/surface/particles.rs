//! What a script reaches on a particle emitter: how many it throws and how
//! fast, and whether it is throwing at all.
//!
//! `this.particles` is the object's own emitter and
//! `this.entity.particles("surge")` a named one (`docs/object-parts.md`).
//! `Effects.play(entity)` still plays an object's own emitter; the methods
//! here reach any part.

use decay_semantic::{FunctionType, HostType, Type};

use super::names::PARTICLES_COMPONENT;
use super::{Leaf, Node, Seg};

const fn number(field: &'static str) -> Node {
    Node::Leaf(Leaf::Component {
        component: PARTICLES_COMPONENT,
        pointer: match field.as_bytes() {
            b"rate" => &[Seg::Field("rate")],
            b"speed" => &[Seg::Field("speed")],
            b"lifetime" => &[Seg::Field("lifetime")],
            _ => &[Seg::Field("size")],
        },
    })
}

pub(crate) const MEMBERS: &[(&str, Node)] = &[
    ("rate", number("rate")),
    ("speed", number("speed")),
    ("lifetime", number("lifetime")),
    ("size", number("size")),
];

/// The emitter's methods.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ParticlesCall {
    Play,
    Stop,
    Emit,
}

pub(crate) const CALLS: &[(&str, ParticlesCall)] = &[
    ("play", ParticlesCall::Play),
    ("stop", ParticlesCall::Stop),
    ("emit", ParticlesCall::Emit),
];

/// The emitter type with its methods, for the analyzer.
pub(crate) fn add_calls(mut ty: HostType) -> HostType {
    for (name, call) in CALLS {
        ty = ty.with_function(
            *name,
            FunctionType {
                params: match call {
                    ParticlesCall::Emit => vec![Type::Number],
                    ParticlesCall::Play | ParticlesCall::Stop => Vec::new(),
                },
                return_type: Type::Number,
            },
        );
    }
    ty
}
