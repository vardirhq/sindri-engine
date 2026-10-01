//! Typed factories, values and shared playback controls.
use crate::surface::tween::{CALLS, METHODS, TWEEN};
use decay_semantic::{Environment, FunctionType, HostType, Type};
pub(super) fn add_tween_surface(environment: &mut Environment) {
    let mut tween = HostType::new();
    for (name, call) in CALLS {
        let handle = Type::Named(call.handle().to_owned());
        tween = tween
            .with_function(
                *name,
                FunctionType {
                    params: vec![call.ty(), call.ty(), Type::F32, Type::String],
                    return_type: handle.clone(),
                },
            )
            .with_function(
                format!("{name}_value"),
                FunctionType {
                    params: vec![handle],
                    return_type: call.ty(),
                },
            );
        environment.add_type(call.handle(), HostType::new());
        environment.add_supertype(call.handle(), "TweenHandle");
    }
    environment.add_type("TweenHandle", HostType::new());
    for method in METHODS {
        tween = tween.with_function(
            *method,
            FunctionType {
                params: vec![Type::Named("TweenHandle".to_owned())],
                return_type: Type::Unit,
            },
        );
    }
    let handle = || Type::Named("TweenHandle".to_owned());
    for (name, second) in [
        ("set_delay", Type::F32),
        ("set_loops", Type::F32),
        ("set_yoyo", Type::Bool),
        ("after", handle()),
    ] {
        tween = tween.with_function(
            name,
            FunctionType {
                params: vec![handle(), second],
                return_type: Type::Unit,
            },
        );
    }
    for (name, ty) in [
        ("progress", Type::F32),
        ("is_done", Type::Bool),
        ("is_paused", Type::Bool),
        ("is_cancelled", Type::Bool),
    ] {
        tween = tween.with_function(
            name,
            FunctionType {
                params: vec![Type::Named("TweenHandle".to_owned())],
                return_type: ty,
            },
        );
    }
    environment.add_type(TWEEN, tween);
    environment.add_value(TWEEN, Type::Named(TWEEN.to_owned()));
}
