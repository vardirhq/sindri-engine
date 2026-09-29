//! What the language's own values can be asked, by name: the operations of a
//! list, a vector, text, a number and a timer.

/// What a list can be asked, or told to change.
///
/// A change — `push`, `pop`, `insert`, `remove_at`, `clear` — is always a
/// call, never a property, and is made to the list a variable or field holds,
/// in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListOp {
    /// `xs.push(value)`: adds `value` at the end.
    Push,
    /// `xs.pop()`: takes the last element off and gives it back.
    Pop,
    /// `xs.insert(index, value)`: puts `value` at `index`, moving the rest
    /// along.
    Insert,
    /// `xs.remove_at(index)`: takes the element at `index` out and gives it
    /// back.
    RemoveAt,
    /// `xs.clear()`: empties it.
    Clear,
    /// `xs.contains(value)`: whether any element equals `value`.
    Contains,
    /// `xs.index_of(value)`: where `value` first is, or `-1`.
    IndexOf,
    /// `xs[index] = value`: what an element assignment lowers to. Not a name
    /// a script can call.
    SetAt,
}

impl ListOp {
    /// Every operation a script can call by name, with that name and how
    /// many arguments it takes after the list itself.
    pub const ALL: [(Self, &'static str, usize); 7] = [
        (Self::Push, "push", 1),
        (Self::Pop, "pop", 0),
        (Self::Insert, "insert", 2),
        (Self::RemoveAt, "remove_at", 1),
        (Self::Clear, "clear", 0),
        (Self::Contains, "contains", 1),
        (Self::IndexOf, "index_of", 1),
    ];

    /// The operation a member name spells, and how many arguments it takes.
    #[must_use]
    pub fn named(name: &str) -> Option<(Self, usize)> {
        Self::ALL
            .into_iter()
            .find(|(_, spelled, _)| *spelled == name)
            .map(|(op, _, arity)| (op, arity))
    }

    /// How many arguments it takes after the list.
    #[must_use]
    pub const fn arity(self) -> usize {
        match self {
            Self::Pop | Self::Clear => 0,
            Self::Push | Self::RemoveAt | Self::Contains | Self::IndexOf => 1,
            Self::Insert | Self::SetAt => 2,
        }
    }

    /// Whether it changes the list rather than only reading it.
    #[must_use]
    pub const fn changes(self) -> bool {
        !matches!(self, Self::Contains | Self::IndexOf)
    }
}

/// What a map can be asked, or told to change.
///
/// A change — `remove`, `clear`, and `m[key] = value` — is made to the map a
/// variable or field holds, in place, as a list's is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapOp {
    /// `m.get(key, fallback)`: the value at `key`, or `fallback`.
    Get,
    /// `m.contains(key)`: whether `key` has a value.
    Contains,
    /// `m.keys()`: every key, in the order they were first set.
    Keys,
    /// `m.values()`: every value, in the same order.
    Values,
    /// `m.remove(key)`: takes `key` out, giving back whether it was there.
    Remove,
    /// `m.clear()`: empties it.
    Clear,
    /// `m[key] = value`: what an entry assignment lowers to. Not a name a
    /// script can call.
    Set,
}

impl MapOp {
    /// Every operation a script can call by name, with that name and how
    /// many arguments it takes after the map itself.
    pub const ALL: [(Self, &'static str, usize); 6] = [
        (Self::Get, "get", 2),
        (Self::Contains, "contains", 1),
        (Self::Keys, "keys", 0),
        (Self::Values, "values", 0),
        (Self::Remove, "remove", 1),
        (Self::Clear, "clear", 0),
    ];

    /// The operation a member name spells, and how many arguments it takes.
    #[must_use]
    pub fn named(name: &str) -> Option<(Self, usize)> {
        Self::ALL
            .into_iter()
            .find(|(_, spelled, _)| *spelled == name)
            .map(|(op, _, arity)| (op, arity))
    }

    /// How many arguments it takes after the map.
    #[must_use]
    pub const fn arity(self) -> usize {
        match self {
            Self::Keys | Self::Values | Self::Clear => 0,
            Self::Contains | Self::Remove => 1,
            Self::Get | Self::Set => 2,
        }
    }

    /// Whether it changes the map rather than only reading it.
    #[must_use]
    pub const fn changes(self) -> bool {
        matches!(self, Self::Remove | Self::Clear | Self::Set)
    }
}

/// What a colour can be asked beyond its `r`, `g`, `b` and `a` channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorOp {
    /// `c.lerp(other, t)`: the colour `t` of the way from this one to
    /// `other`, channel by channel, alpha included.
    Lerp,
    /// `c.with_alpha(a)`: the same colour at another opacity.
    WithAlpha,
    /// `Color("#ff8800")`: a colour from its hex spelling. Not a name a script
    /// calls; what building one from text lowers to.
    FromHex,
}

impl ColorOp {
    /// Every operation a script can call by name, with that name and how
    /// many arguments it takes after the colour itself.
    pub const ALL: [(Self, &'static str, usize); 2] =
        [(Self::Lerp, "lerp", 2), (Self::WithAlpha, "with_alpha", 1)];

    /// The operation a member name spells, and how many arguments it takes.
    #[must_use]
    pub fn named(name: &str) -> Option<(Self, usize)> {
        Self::ALL
            .into_iter()
            .find(|(_, spelled, _)| *spelled == name)
            .map(|(op, _, arity)| (op, arity))
    }
}

/// The channels `#rrggbb` or `#rrggbbaa` spells, from 0 to 1.
#[must_use]
pub fn parse_hex(text: &str) -> Option<[f64; 4]> {
    let digits = text.strip_prefix('#')?;
    if !matches!(digits.len(), 6 | 8) || !digits.is_ascii() {
        return None;
    }
    let mut channels = [1.0; 4];
    for (slot, pair) in channels.iter_mut().zip(digits.as_bytes().chunks(2)) {
        let pair = std::str::from_utf8(pair).ok()?;
        *slot = f64::from(u8::from_str_radix(pair, 16).ok()?) / 255.0;
    }
    Some(channels)
}

/// What a vector can be asked beyond its components and arithmetic.
///
/// Here, beside the operators, because it is the same kind of thing: an
/// operation the language owns over values it owns. The analyzer decides which
/// member read or call is one of these, the IR carries it, and the runtime
/// performs it, so all three need one spelling of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorOp {
    /// `v.length`: how long it is.
    Length,
    /// `v.normalized`: the same direction, one unit long, or zero for a zero
    /// vector rather than a vector of NaN.
    Normalized,
    /// `a.dot(b)`.
    Dot,
    /// `a.distance(b)`: how far apart two points are.
    Distance,
    /// `a.lerp(b, t)`: the point `t` of the way from `a` to `b`.
    Lerp,
}

impl VectorOp {
    /// Every operation, with the name a script spells it by and how many
    /// arguments it takes after the vector itself. A property is the one that
    /// takes none.
    pub const ALL: [(Self, &'static str, usize); 5] = [
        (Self::Length, "length", 0),
        (Self::Normalized, "normalized", 0),
        (Self::Dot, "dot", 1),
        (Self::Distance, "distance", 1),
        (Self::Lerp, "lerp", 2),
    ];

    /// The operation a member name spells, and how many arguments it takes.
    #[must_use]
    pub fn named(name: &str) -> Option<(Self, usize)> {
        Self::ALL
            .into_iter()
            .find(|(_, spelled, _)| *spelled == name)
            .map(|(op, _, arity)| (op, arity))
    }
}

/// What a piece of text can be asked. Like a vector's, one that takes no
/// arguments is a property: `s.length`, `s.uppercase`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringOp {
    /// `s.length`: how many characters it has.
    Length,
    /// `s.uppercase`: the same text in capitals.
    Uppercase,
    /// `s.lowercase`: the same text in small letters.
    Lowercase,
    /// `s.trimmed`: without the spaces at either end.
    Trimmed,
    /// `s.contains(part)`.
    Contains,
    /// `s.starts_with(part)`.
    StartsWith,
    /// `s.ends_with(part)`.
    EndsWith,
    /// `s.find(part)`: where `part` first starts, in characters, or `-1`.
    Find,
    /// `s.slice(start, end)`: the characters from `start` up to, not
    /// including, `end`.
    Slice,
    /// `s.replace(old, new)`: every `old` replaced with `new`.
    Replace,
}

impl StringOp {
    /// Every operation, with the name a script spells it by and how many
    /// arguments it takes after the text itself. A property takes none.
    pub const ALL: [(Self, &'static str, usize); 10] = [
        (Self::Length, "length", 0),
        (Self::Uppercase, "uppercase", 0),
        (Self::Lowercase, "lowercase", 0),
        (Self::Trimmed, "trimmed", 0),
        (Self::Contains, "contains", 1),
        (Self::StartsWith, "starts_with", 1),
        (Self::EndsWith, "ends_with", 1),
        (Self::Find, "find", 1),
        (Self::Slice, "slice", 2),
        (Self::Replace, "replace", 2),
    ];

    /// The operation a member name spells, and how many arguments it takes.
    #[must_use]
    pub fn named(name: &str) -> Option<(Self, usize)> {
        Self::ALL
            .into_iter()
            .find(|(_, spelled, _)| *spelled == name)
            .map(|(op, _, arity)| (op, arity))
    }
}

/// What a number can be asked: how to write it as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumberOp {
    /// `n.fixed(digits)`: with exactly `digits` decimals, `12.50` for
    /// `12.5.fixed(2)`.
    Fixed,
    /// `n.padded(width)`: rounded to a whole number and led with zeros to at
    /// least `width` digits, `007` for `7.0.padded(3)`.
    Padded,
}

impl NumberOp {
    /// Every operation, with the name a script spells it by and how many
    /// arguments it takes after the number itself.
    pub const ALL: [(Self, &'static str, usize); 2] =
        [(Self::Fixed, "fixed", 1), (Self::Padded, "padded", 1)];

    /// The operation a member name spells, and how many arguments it takes.
    #[must_use]
    pub fn named(name: &str) -> Option<(Self, usize)> {
        Self::ALL
            .into_iter()
            .find(|(_, spelled, _)| *spelled == name)
            .map(|(op, _, arity)| (op, arity))
    }
}

/// What a timer can be asked. All are properties: a timer is read, and
/// replaced with a new one to start it again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimerProperty {
    /// `t.done`: whether it has run out.
    Done,
    /// `t.left`: seconds to go, never below zero.
    Left,
    /// `t.duration`: the seconds it was started with.
    Duration,
    /// `t.progress`: how far through it is, from 0 when started to 1 when done.
    Progress,
}

impl TimerProperty {
    /// Every property, with the name a script spells it by.
    pub const ALL: [(Self, &'static str); 4] = [
        (Self::Done, "done"),
        (Self::Left, "left"),
        (Self::Duration, "duration"),
        (Self::Progress, "progress"),
    ];

    /// The property a member name spells.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|(_, spelled)| *spelled == name)
            .map(|(property, _)| property)
    }
}
