//! The part of the environment that describes the person at the controls.

use decay_semantic::{Environment, FunctionType, HostType, Type};

use crate::surface::{
    AIM, AIM_VALUES, AimValue, CAMERA, CAMERA_CALLS, CAMERA_VALUES, CameraCall, ENTITY, GESTURE,
    GESTURE_VALUES, GestureValue, POINTER, POINTER_QUERIES, POINTER_VALUES, PointerValue, STICK,
    STICK_VALUES, StickValue, TOUCH, TOUCH_CALLS, TOUCH_COUNT, VIEWPORT, VIEWPORT_VALUES,
};

pub(super) fn add_viewport_surface(environment: &mut Environment) {
    let mut viewport = HostType::new();
    for (name, _) in VIEWPORT_VALUES { viewport = viewport.with_value(*name, Type::F32); }
    environment.add_type(VIEWPORT, viewport);
    environment.add_value(VIEWPORT, Type::Named(VIEWPORT.to_owned()));
}

pub(super) fn add_aim_surface(environment: &mut Environment) {
    let mut aim = HostType::new();
    for (name, value) in AIM_VALUES {
        aim = aim.with_value(*name, if matches!(value, AimValue::Hit) { Type::Bool } else { Type::F32 });
    }
    environment.add_type(AIM, aim);
    environment.add_value(AIM, Type::Named(AIM.to_owned()));
}

pub(super) fn add_camera_surface(environment: &mut Environment) {
    let mut camera = HostType::new();
    for (name, _) in CAMERA_VALUES { camera = camera.with_value(*name, Type::F32); }
    for (name, call) in CAMERA_CALLS {
        let params = match call {
            CameraCall::AddTrauma | CameraCall::Smoothing | CameraCall::MaxSpeed => vec![Type::F32],
            CameraCall::Follow => vec![Type::Named(ENTITY.to_owned())],
            CameraCall::ClearFollow | CameraCall::ClearBounds => Vec::new(),
            CameraCall::FollowOffset | CameraCall::Shake => vec![Type::F32, Type::F32, Type::F32],
            CameraCall::DeadZone => vec![Type::F32, Type::F32],
            CameraCall::Bounds => vec![Type::F32, Type::F32, Type::F32, Type::F32],
        };
        camera = camera.with_function(*name, FunctionType { params, return_type: Type::Unit });
    }
    environment.add_type(CAMERA, camera);
    environment.add_value(CAMERA, Type::Named(CAMERA.to_owned()));
}

pub(super) fn add_gesture_surface(environment: &mut Environment) {
    let mut gesture = HostType::new();
    for (name, value) in GESTURE_VALUES {
        gesture = gesture.with_value(*name, match value {
            GestureValue::Tapped | GestureValue::Held | GestureValue::Dragging | GestureValue::Pinching => Type::Bool,
            _ => Type::F32,
        });
    }
    environment.add_type(GESTURE, gesture);
    environment.add_value(GESTURE, Type::Named(GESTURE.to_owned()));
}

pub(super) fn add_pointer_surface(environment: &mut Environment) {
    let mut pointer = HostType::new();
    for (name, value) in POINTER_VALUES {
        pointer = pointer.with_value(*name, match value {
            PointerValue::X | PointerValue::Y | PointerValue::OverlayX | PointerValue::OverlayY => Type::F32,
            PointerValue::Inside | PointerValue::OverUi => Type::Bool,
            PointerValue::Position | PointerValue::Overlay => Type::Vec2,
        });
    }
    for (name, _) in POINTER_QUERIES {
        pointer = pointer.with_function(*name, FunctionType { params: vec![Type::String], return_type: Type::Bool });
    }
    environment.add_type(POINTER, pointer);
    environment.add_value(POINTER, Type::Named(POINTER.to_owned()));

    let mut touch = HostType::new().with_value(TOUCH_COUNT, Type::F32);
    for (name, _) in TOUCH_CALLS {
        touch = touch.with_function(*name, FunctionType { params: vec![Type::F32], return_type: Type::F32 });
    }
    environment.add_type(TOUCH, touch);
    environment.add_value(TOUCH, Type::Named(TOUCH.to_owned()));

    let mut stick = HostType::new();
    for (name, value) in STICK_VALUES {
        stick = stick.with_value(*name, match value {
            StickValue::Held => Type::Bool,
            StickValue::Direction => Type::Vec2,
            _ => Type::F32,
        });
    }
    environment.add_type(STICK, stick);
    environment.add_value(STICK, Type::Named(STICK.to_owned()));
}
