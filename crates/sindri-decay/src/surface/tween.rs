//! Tween factories and typed handles share the analyzer/runtime contract.
use decay_semantic::Type;
pub(crate) const TWEEN: &str = "Tween";
#[derive(Clone, Copy)]
pub(crate) enum TweenCall {
    Number,
    Vec2,
    Vec3,
    Color,
}
pub(crate) const CALLS: &[(&str, TweenCall)] = &[
    ("number", TweenCall::Number),
    ("vec2", TweenCall::Vec2),
    ("vec3", TweenCall::Vec3),
    ("color", TweenCall::Color),
];
pub(crate) const METHODS: &[&str] = &["pause", "resume", "cancel", "restart", "dispose"];
/// How a tween plays, set on its handle: a delay, a loop count, a yoyo and a
/// tween it waits for.
pub(crate) const COMPOSE: &[&str] = &["set_delay", "set_loops", "set_yoyo", "after"];
impl TweenCall {
    pub(crate) fn ty(self) -> Type {
        match self {
            Self::Number => Type::F32,
            Self::Vec2 => Type::Vec2,
            Self::Vec3 => Type::Vec3,
            Self::Color => Type::Color,
        }
    }
    pub(crate) fn handle(self) -> &'static str {
        match self {
            Self::Number => "NumberTween",
            Self::Vec2 => "Vec2Tween",
            Self::Vec3 => "Vec3Tween",
            Self::Color => "ColorTween",
        }
    }
    pub(crate) fn accepts(self, value: &decay_runtime::Value) -> bool {
        matches!(
            (self, value),
            (Self::Number, decay_runtime::Value::Number(_))
                | (Self::Vec2, decay_runtime::Value::Vec2(_))
                | (Self::Vec3, decay_runtime::Value::Vec3(_))
                | (Self::Color, decay_runtime::Value::Color(_))
        )
    }
}
