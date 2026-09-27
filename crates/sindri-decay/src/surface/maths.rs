//! The maths a script can do on numbers, and the named numbers it can use.
//!
//! Vectors have their own operations in the language itself; these are the
//! number functions the host adds.

/// A host function, by how many numbers it takes.
#[derive(Clone, Copy, Debug)]
pub(crate) enum HostFunction {
    Unary(fn(f64) -> f64),
    Binary(fn(f64, f64) -> f64),
    Ternary(fn(f64, f64, f64) -> f64),
}

/// The maths a script can do beyond arithmetic.
///
/// Decay has no modules and no imports, so each of these is a bare global name,
/// and every one added is a name a script can no longer use for its own.
pub(crate) const FUNCTIONS: &[(&str, HostFunction)] = &[
    ("abs", HostFunction::Unary(f64::abs)),
    ("sqrt", HostFunction::Unary(f64::sqrt)),
    ("sin", HostFunction::Unary(f64::sin)),
    ("cos", HostFunction::Unary(f64::cos)),
    ("exp", HostFunction::Unary(f64::exp)),
    ("atan2", HostFunction::Binary(f64::atan2)),
    ("min", HostFunction::Binary(f64::min)),
    ("max", HostFunction::Binary(f64::max)),
    ("floor", HostFunction::Unary(f64::floor)),
    ("ceil", HostFunction::Unary(f64::ceil)),
    // Halves away from zero, as `f64::round` does: `round(-2.5)` is `-3`.
    ("round", HostFunction::Unary(f64::round)),
    ("sign", HostFunction::Unary(sign)),
    ("clamp", HostFunction::Ternary(clamp)),
    ("lerp", HostFunction::Ternary(lerp)),
];

/// -1, 0 or 1. Zero for zero, which `f64::signum` is not: it answers 1 for
/// `+0.0`, and "which way is this moving" is the question a script asks.
fn sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// `value` kept between `low` and `high`. Bounds given the wrong way round are
/// taken as meant rather than panicking as `f64::clamp` would: a script that
/// swaps them should not take the game down.
fn clamp(value: f64, low: f64, high: f64) -> f64 {
    value.max(low.min(high)).min(high.max(low))
}

/// The number `t` of the way from `a` to `b`: `a` at 0, `b` at 1.
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    (b - a).mul_add(t, a)
}

/// Named numbers a script can use as globals.
pub(crate) const CONSTANTS: &[(&str, f64)] =
    &[("PI", std::f64::consts::PI), ("TAU", std::f64::consts::TAU)];
