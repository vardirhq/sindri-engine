//! What each name on the host surface means, in words.
//!
//! The surface itself — which names exist and what types they take — is
//! [`crate::environment`], and nothing here can add to it. This is the prose
//! beside it: a sentence or three per name, and the names of a call's
//! parameters, which type-checking never needed and so the environment never
//! recorded. `docs/scripting.md` explains the design; this is what a
//! reference page, a hover or a completion shows for one name.
//!
//! Complete by test rather than by care: sindri-capabilities fails when a name
//! on the surface has no entry here, when an entry names nothing on the
//! surface, or when a call's parameter names do not match its arity. So a new
//! host call cannot ship undescribed, and a removed one cannot leave its
//! description behind.

mod character;
mod game;
mod input;
mod physics;
mod physics3d;
mod tween;
mod world;

/// One name: a value a script reads, or a call it makes.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub name: &'static str,
    /// A call's parameter names, in order; empty for a value.
    pub params: &'static [&'static str],
    pub text: &'static str,
}

/// A host type and what each of its members means.
#[derive(Clone, Copy, Debug)]
pub struct TypeEntry {
    pub name: &'static str,
    pub text: &'static str,
    pub members: &'static [Entry],
}

pub(crate) const fn value(name: &'static str, text: &'static str) -> Entry {
    Entry {
        name,
        params: &[],
        text,
    }
}

pub(crate) const fn call(
    name: &'static str,
    params: &'static [&'static str],
    text: &'static str,
) -> Entry {
    Entry { name, params, text }
}

/// The global functions and numbers. A namespace such as `World` is described
/// by its type in [`TYPES`] rather than here.
pub const GLOBALS: &[Entry] = &[
    call(
        "abs",
        &["value"],
        "The number without its minus sign: `abs(-3.0)` is `3`. Useful for distances and differences where only the size matters.",
    ),
    call(
        "atan2",
        &["y", "x"],
        "The angle pointing from the origin towards the point (x, y), in radians. Use it to turn something to face a direction: `atan2(dy, dx)`.",
    ),
    call(
        "ceil",
        &["value"],
        "Rounds up to a whole number: `ceil(2.1)` is `3`.",
    ),
    call(
        "clamp",
        &["value", "low", "high"],
        "Keeps a number between a lowest and a highest value: `clamp(hp, 0.0, 100.0)` never goes below 0 or above 100.",
    ),
    call(
        "cos",
        &["angle"],
        "The cosine of an angle in radians. With `sin`, turns an angle into a direction: `Vec2(cos(a), sin(a))`.",
    ),
    call(
        "exp",
        &["value"],
        "The number e raised to a power. Mostly used for smooth movement that looks the same at any frame rate: `1.0 - exp(-speed * dt)`.",
    ),
    call(
        "floor",
        &["value"],
        "Rounds down to a whole number: `floor(2.9)` is `2`.",
    ),
    call(
        "lerp",
        &["a", "b", "t"],
        "Blends between two numbers: `t` of 0 gives `a`, 1 gives `b`, and 0.5 gives halfway. Used for smooth movement and fades.",
    ),
    call("max", &["a", "b"], "The larger of two numbers."),
    call("min", &["a", "b"], "The smaller of two numbers."),
    value(
        "PI",
        "The number π (3.14159…): half a full turn when measuring angles in radians.",
    ),
    call(
        "print",
        &["value"],
        "Writes a message to the log, for checking what a script is doing. Accepts any value; combine text and values with `+`: `print(\"hp \" + hp)`.",
    ),
    call(
        "round",
        &["value"],
        "Rounds to the nearest whole number: `round(2.5)` is `3` and `round(-2.5)` is `-3`.",
    ),
    call(
        "sign",
        &["value"],
        "Whether a number is negative, zero or positive, as -1, 0 or 1. Handy for \"which way is this moving?\".",
    ),
    call(
        "sin",
        &["angle"],
        "The sine of an angle in radians. Good for bobbing and waving motion: `sin(Time.elapsed)`.",
    ),
    call("sqrt", &["value"], "The square root of a number."),
    value(
        "TAU",
        "The number τ (6.28318…), which is 2π: one full turn when measuring angles in radians.",
    ),
];

/// What `this` offers beyond a script's own fields.
pub const THIS: &[Entry] = &[
    value(
        "entity",
        "The object in the game this script is attached to. Pass it to calls that act on an object, such as `World.despawn(this.entity)`.",
    ),
    value(
        "shape",
        "The drawn shape on this script's object, if it has one: its colours, outline and how much of it is drawn.",
    ),
    value(
        "sprite",
        "The image (sprite) on this script's object, if it has one: its colour and drawing order.",
    ),
    value(
        "transform",
        "Where this script's object is, how it is turned and how big it is. Change it to move the object.",
    ),
    value(
        "ui_image",
        "The on-screen interface image on this script's object, if it has one: its colour and drawing order.",
    ),
];

/// Every host type, and each of its members.
pub const TYPES: &[&[TypeEntry]] = &[
    world::TYPES,
    game::TYPES,
    input::TYPES,
    tween::TYPES,
    physics3d::TYPES,
];

/// The entry for a host type, if it has one.
#[must_use]
pub fn type_entry(name: &str) -> Option<&'static TypeEntry> {
    TYPES
        .iter()
        .flat_map(|types| types.iter())
        .find(|entry| entry.name == name)
}

/// The entry for a global function or number.
#[must_use]
pub fn global_entry(name: &str) -> Option<&'static Entry> {
    GLOBALS.iter().find(|entry| entry.name == name)
}

/// The entry for a member of `this`.
#[must_use]
pub fn this_entry(name: &str) -> Option<&'static Entry> {
    THIS.iter().find(|entry| entry.name == name)
}

/// The entry for a member of a host type.
#[must_use]
pub fn member_entry(owner: &str, name: &str) -> Option<&'static Entry> {
    type_entry(owner)?
        .members
        .iter()
        .find(|entry| entry.name == name)
}
