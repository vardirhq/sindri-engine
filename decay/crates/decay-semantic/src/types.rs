//! What a Decay value can be, and what a host says about its own types.

use std::borrow::Cow;
use std::collections::HashMap;

use decay_syntax::TypeRef;

/// How a collection type is spelled in Decay source.
pub const LIST: &str = "List";

/// The collection type's older spelling, which still means the same type.
pub const ARRAY: &str = "Array";

/// Whether a written type name is the collection type.
#[must_use]
pub fn is_list(name: &str) -> bool {
    name == LIST || name == ARRAY
}

/// A collection's size, as text's and a vector's length are spelled.
///
/// A property rather than a `len(x)` global, because Decay has no modules and
/// every global name added is one a script can no longer use for its own. A
/// length is a property of the value, and spelling it as one costs nobody a
/// name.
pub const LENGTH: &str = "length";

/// A collection's size as it was first spelled, which still works.
pub const OLD_LENGTH: &str = "len";

/// Whether a member name asks a collection its size.
#[must_use]
pub fn is_length(name: &str) -> bool {
    name == LENGTH || name == OLD_LENGTH
}

/// How the two vector types are spelled. Language types rather than host
/// ones: a vector is a value a script builds, adds and keeps, which a host
/// type — something the host owns and a script can only name — cannot be.
pub const VEC2: &str = "Vec2";
pub const VEC3: &str = "Vec3";

/// Every built-in type that takes no argument, by each spelling it is written
/// with. `List<T>` (or `Array<T>`) is the one that takes an argument; every
/// other name is a declared struct or enum, or a type the host names.
pub const BUILT_IN_TYPES: [(&str, Type); 9] = [
    ("f32", Type::F32),
    ("bool", Type::Bool),
    ("String", Type::String),
    ("string", Type::String),
    ("unit", Type::Unit),
    ("void", Type::Unit),
    (VEC2, Type::Vec2),
    (VEC3, Type::Vec3),
    (TIMER, Type::Timer),
];

/// The member an event offers: `GoalScored.emit(1.0)`.
pub const EMIT: &str = "emit";

/// The type an enum's name has in an expression — `Phase` in `Phase.Lobby` —
/// as a diagnostic names it. Its values are of type `Named(name)`.
#[must_use]
pub fn enum_type(name: &str) -> String {
    format!("enum {name}")
}

/// The type an event's name has in an expression, as a diagnostic names it.
/// Not a name a script can write, so it can never be confused with one.
#[must_use]
pub fn event_type(event: &str) -> String {
    format!("event {event}")
}

/// How a timer is spelled, as a type and as the call that starts one.
pub const TIMER: &str = "Timer";

/// Component names, in order. `x` and `y` for both; `z` only for `Vec3`.
pub const COMPONENTS: [&str; 3] = ["x", "y", "z"];

use crate::environment::ExternalSymbol;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    F32,
    Bool,
    String,
    Unit,
    Null,
    Named(String),
    /// Two numbers, `x` and `y`, that travel together: a point on the screen, a
    /// direction on a flat board, a velocity in 2D.
    Vec2,
    /// Three numbers, `x`, `y` and `z`: a position, a scale.
    Vec3,
    /// A countdown: `Timer(2.4)` runs out 2.4 seconds later. The host decides
    /// when time passes; Sindri runs down every timer a script's fields hold
    /// before each `update`.
    Timer,
    /// A fixed-length collection of one element type.
    ///
    /// The only generic type the language has, and it is not user-definable:
    /// the host hands one back, a script reads it, and nothing constructs,
    /// grows, or shrinks one. That is what keeps a collection bounded without
    /// the language having to say anything about memory.
    Array(Box<Type>),
    Unknown,
}

impl Type {
    #[must_use]
    pub fn from_ref(reference: &TypeRef) -> Self {
        if let Some((_, ty)) = BUILT_IN_TYPES
            .iter()
            .find(|(spelled, _)| *spelled == reference.name)
        {
            return ty.clone();
        }
        match reference.name.as_str() {
            // `Array` written without an argument is `Array<unknown>` rather
            // than a diagnostic. The analyzer reports the missing argument
            // where the type was written; treating it as unknown here keeps
            // one mistake from cascading into every use of the binding.
            LIST | ARRAY => Self::Array(Box::new(
                reference
                    .argument
                    .as_ref()
                    .map_or(Self::Unknown, |argument| Self::from_ref(argument)),
            )),
            other => Self::Named(other.to_owned()),
        }
    }

    /// The element type, for a type that holds several of something.
    #[must_use]
    pub fn element(&self) -> Option<&Self> {
        match self {
            Self::Array(element) => Some(element),
            _ => None,
        }
    }

    /// How many components a vector type has; `None` for anything else.
    #[must_use]
    pub const fn dimensions(&self) -> Option<usize> {
        match self {
            Self::Vec2 => Some(2),
            Self::Vec3 => Some(3),
            _ => None,
        }
    }

    /// The vector type with this many components.
    #[must_use]
    pub const fn vector(dimensions: usize) -> Option<Self> {
        match dimensions {
            2 => Some(Self::Vec2),
            3 => Some(Self::Vec3),
            _ => None,
        }
    }

    /// A collection of `element`.
    #[must_use]
    pub fn array_of(element: Self) -> Self {
        Self::Array(Box::new(element))
    }

    /// How this type is written in Decay source, and in a diagnostic about it.
    ///
    /// Public because a host describing itself — in an error, or in a manifest
    /// a tool reads — needs to name a type the same way the compiler does.
    #[must_use]
    pub fn display_name(&self) -> Cow<'_, str> {
        match self {
            Self::F32 => Cow::Borrowed("f32"),
            Self::Bool => Cow::Borrowed("bool"),
            Self::String => Cow::Borrowed("String"),
            Self::Unit => Cow::Borrowed("unit"),
            Self::Null => Cow::Borrowed("null"),
            Self::Named(name) => Cow::Borrowed(name),
            Self::Vec2 => Cow::Borrowed(VEC2),
            Self::Vec3 => Cow::Borrowed(VEC3),
            Self::Timer => Cow::Borrowed(TIMER),
            Self::Array(element) => Cow::Owned(format!("{LIST}<{}>", element.display_name())),
            Self::Unknown => Cow::Borrowed("unknown"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub return_type: Type,
}

/// A host type, described by what it offers.
///
/// The language has no way to declare one: `Transform` is a name Decay carries
/// and cannot look inside, so the host is the only thing that can say a
/// transform has a position. Until it did, every member access produced
/// `Unknown`, `Unknown` is compatible with everything, and
/// `this.transfrom.position.x` type-checked cleanly and failed at frame one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostType {
    members: HashMap<String, ExternalSymbol>,
}

impl HostType {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_value(mut self, name: impl Into<String>, ty: Type) -> Self {
        self.members.insert(name.into(), ExternalSymbol::Value(ty));
        self
    }

    #[must_use]
    pub fn with_function(mut self, name: impl Into<String>, function: FunctionType) -> Self {
        self.members
            .insert(name.into(), ExternalSymbol::Function(function));
        self
    }

    #[must_use]
    pub fn member(&self, name: &str) -> Option<&ExternalSymbol> {
        self.members.get(name)
    }

    /// Whether the host said anything about this type at all.
    ///
    /// A type with no members is treated as *undescribed* rather than as
    /// described-and-empty, so that a host which has not started describing
    /// itself behaves exactly as every host did before types existed. The two
    /// are indistinguishable and only one of them is useful.
    #[must_use]
    pub fn is_described(&self) -> bool {
        !self.members.is_empty()
    }

    /// Every member, for a host emitting a description of itself.
    pub fn members(&self) -> impl Iterator<Item = (&str, &ExternalSymbol)> {
        self.members
            .iter()
            .map(|(name, symbol)| (name.as_str(), symbol))
    }
}
