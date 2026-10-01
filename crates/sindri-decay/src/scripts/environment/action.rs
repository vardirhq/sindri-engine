//! `Action.*`: the scene's input actions, by name.
use crate::surface::ACTION;
use decay_semantic::{Environment, FunctionType, HostType, Type};

pub(super) fn add_action_surface(environment: &mut Environment) {
    let call = |params: Vec<Type>, return_type: Type| FunctionType {
        params,
        return_type,
    };
    let mut action = HostType::new();
    for name in ["held", "pressed", "released"] {
        action = action.with_function(name, call(vec![Type::String], Type::Bool));
    }
    action = action
        .with_function("axis", call(vec![Type::String], Type::F32))
        .with_function("vector", call(vec![Type::String], Type::Vec2))
        .with_function(
            "bindings",
            call(vec![Type::String], Type::array_of(Type::String)),
        )
        .with_function(
            "rebind",
            call(vec![Type::String, Type::F32, Type::String], Type::Unit),
        )
        .with_function("last_pressed", call(Vec::new(), Type::String));
    environment.add_type(ACTION, action);
    environment.add_value(ACTION, Type::Named(ACTION.to_owned()));
}
