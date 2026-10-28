//! Questions a script can ask the 2D lighting.

use decay_semantic::{Environment, FunctionType, HostType, Type};

pub(crate) const LIGHTING: &str = "Lighting";

#[derive(Clone, Copy)]
pub(crate) enum LightingCall {
    Visible,
    Level,
}

pub(crate) const CALLS: &[(&str, LightingCall)] = &[
    ("visible", LightingCall::Visible),
    ("level", LightingCall::Level),
];

pub(crate) fn add_surface(environment: &mut Environment) {
    let mut lighting = HostType::new();
    for (name, call) in CALLS {
        let (params, return_type) = match call {
            LightingCall::Visible => (vec![Type::Vec2, Type::Vec2], Type::Bool),
            LightingCall::Level => (vec![Type::Vec2], Type::Number),
        };
        lighting = lighting.with_function(
            *name,
            FunctionType {
                params,
                return_type,
            },
        );
    }
    environment.add_type(LIGHTING, lighting);
    environment.add_value(LIGHTING, Type::Named(LIGHTING.to_owned()));
}
