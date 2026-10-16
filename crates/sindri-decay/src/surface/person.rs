//! Host surface for player input and camera-facing runtime values.

use super::names::{ACTION, GAMEPAD, GESTURE, INPUT, KEYBOARD, POINTER, STICK, TOUCH};

/// What `Input` gathers, each under its own name.
///
/// Each is also the name of the namespace that answers it, so `Input.Stick.x`
/// is answered by dropping `Input` rather than by a second path to the same
/// state. `Aim` and `Camera` are not here: they answer questions about the
/// world the person is looking at, not about what their hands are doing.
pub(crate) const INPUT_GROUPS: &[&str] =
    &[ACTION, GAMEPAD, GESTURE, KEYBOARD, POINTER, STICK, TOUCH];

/// The namespace a path names, with `Input` gathered away: `Input.Stick.x` is
/// `Stick.x`. A path that does not go through `Input` this way is unchanged.
pub(crate) fn ungrouped<'a, 'b>(parts: &'a [&'b str]) -> &'a [&'b str] {
    match parts {
        [root, group, _, ..] if *root == INPUT && INPUT_GROUPS.contains(group) => &parts[1..],
        _ => parts,
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum InputQuery {
    Axis,
    Down,
    Pressed,
    Released,
}
impl InputQuery {
    pub(crate) const fn keys(self) -> usize {
        match self {
            Self::Axis => 2,
            _ => 1,
        }
    }
    pub(crate) const fn is_number(self) -> bool {
        matches!(self, Self::Axis)
    }
}
pub(crate) const INPUT_QUERIES: &[(&str, InputQuery)] = &[
    ("axis", InputQuery::Axis),
    ("is_down", InputQuery::Down),
    ("just_pressed", InputQuery::Pressed),
    ("just_released", InputQuery::Released),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PointerQuery {
    Lock,
    Unlock,
    Down,
    Pressed,
    Released,
}
pub(crate) const POINTER_QUERIES: &[(&str, PointerQuery)] = &[
    ("lock", PointerQuery::Lock),
    ("unlock", PointerQuery::Unlock),
    ("is_down", PointerQuery::Down),
    ("just_pressed", PointerQuery::Pressed),
    ("just_released", PointerQuery::Released),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PointerValue {
    X,
    Y,
    Inside,
    OverUi,
    OverlayX,
    OverlayY,
    Position,
    Delta,
    Locked,
    Overlay,
}
pub(crate) const POINTER_VALUES: &[(&str, PointerValue)] = &[
    ("x", PointerValue::X),
    ("y", PointerValue::Y),
    ("overlay_x", PointerValue::OverlayX),
    ("overlay_y", PointerValue::OverlayY),
    ("position", PointerValue::Position),
    ("delta", PointerValue::Delta),
    ("locked", PointerValue::Locked),
    ("overlay", PointerValue::Overlay),
    ("inside", PointerValue::Inside),
    ("over_ui", PointerValue::OverUi),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AimValue {
    Hit,
    X,
    Y,
    Z,
    PlaceX,
    PlaceY,
    PlaceZ,
}
pub(crate) const AIM_VALUES: &[(&str, AimValue)] = &[
    ("hit", AimValue::Hit),
    ("x", AimValue::X),
    ("y", AimValue::Y),
    ("z", AimValue::Z),
    ("place_x", AimValue::PlaceX),
    ("place_y", AimValue::PlaceY),
    ("place_z", AimValue::PlaceZ),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CameraValue {
    PanX,
    PanY,
    PanZ,
}
pub(crate) const CAMERA_VALUES: &[(&str, CameraValue)] = &[
    ("pan_x", CameraValue::PanX),
    ("pan_y", CameraValue::PanY),
    ("pan_z", CameraValue::PanZ),
];

/// Gameplay intent for the engine-owned camera behavior system.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CameraCall {
    AddTrauma,
    Impact,
    Follow,
    ClearFollow,
    FollowOffset,
    DeadZone,
    Smoothing,
    MaxSpeed,
    OrthographicSize,
    PerspectiveFov,
    Orbit,
    OrbitOffset,
    OrbitSmoothing,
    OrbitCollision,
    ClearOrbit,
    Bounds,
    ClearBounds,
    Shake,
}
pub(crate) const CAMERA_CALLS: &[(&str, CameraCall)] = &[
    ("add_trauma", CameraCall::AddTrauma),
    ("impact", CameraCall::Impact),
    ("follow", CameraCall::Follow),
    ("clear_follow", CameraCall::ClearFollow),
    ("follow_offset", CameraCall::FollowOffset),
    ("dead_zone", CameraCall::DeadZone),
    ("smoothing", CameraCall::Smoothing),
    ("max_speed", CameraCall::MaxSpeed),
    ("orthographic_size", CameraCall::OrthographicSize),
    ("perspective_fov", CameraCall::PerspectiveFov),
    ("orbit", CameraCall::Orbit),
    ("orbit_offset", CameraCall::OrbitOffset),
    ("orbit_smoothing", CameraCall::OrbitSmoothing),
    ("orbit_collision", CameraCall::OrbitCollision),
    ("clear_orbit", CameraCall::ClearOrbit),
    ("bounds", CameraCall::Bounds),
    ("clear_bounds", CameraCall::ClearBounds),
    ("shake", CameraCall::Shake),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GestureValue {
    Tapped,
    TapX,
    TapY,
    Held,
    HoldX,
    HoldY,
    Dragging,
    DragX,
    DragY,
    Pinching,
    Pinch,
}
pub(crate) const GESTURE_VALUES: &[(&str, GestureValue)] = &[
    ("tapped", GestureValue::Tapped),
    ("tap_x", GestureValue::TapX),
    ("tap_y", GestureValue::TapY),
    ("held", GestureValue::Held),
    ("hold_x", GestureValue::HoldX),
    ("hold_y", GestureValue::HoldY),
    ("dragging", GestureValue::Dragging),
    ("drag_x", GestureValue::DragX),
    ("drag_y", GestureValue::DragY),
    ("pinching", GestureValue::Pinching),
    ("pinch", GestureValue::Pinch),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ViewportValue {
    Aspect,
}
pub(crate) const VIEWPORT_VALUES: &[(&str, ViewportValue)] = &[("aspect", ViewportValue::Aspect)];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StickValue {
    X,
    Y,
    Held,
    AnchorX,
    AnchorY,
    Direction,
}
pub(crate) const STICK_VALUES: &[(&str, StickValue)] = &[
    ("x", StickValue::X),
    ("y", StickValue::Y),
    ("held", StickValue::Held),
    ("anchor_x", StickValue::AnchorX),
    ("anchor_y", StickValue::AnchorY),
    ("direction", StickValue::Direction),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TouchCall {
    X,
    Y,
}
pub(crate) const TOUCH_CALLS: &[(&str, TouchCall)] = &[("x", TouchCall::X), ("y", TouchCall::Y)];
pub(crate) const TOUCH_COUNT: &str = "count";
