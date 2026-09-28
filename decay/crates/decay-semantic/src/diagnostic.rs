//! What analysis reports, and where it points.

use std::collections::HashMap;

use decay_syntax::{Program, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticPhase {
    Syntax,
    Semantic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub phase: DiagnosticPhase,
    pub message: String,
    pub span: Span,
    pub line: usize,
    pub column: usize,
}

/// A member the analyzer resolved on a *value* rather than along a path.
///
/// Most of `a.b.c` is a path the host gives a meaning to, and the IR forwards
/// it whole. A few members belong to a value the language itself understands,
/// and those cannot be forwarded — there is no host to ask what the length of
/// a collection is. The analyzer knows which is which because it knows the
/// types, so it says so here rather than leaving the lowering to guess from a
/// member's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueMember {
    /// How many elements a collection holds.
    Length,
    /// One component of a vector the script holds, by position: `x` is 0.
    ///
    /// Only for a vector that is a value — a local, a parameter, a field, or
    /// the result of an expression. `this.transform.position.x` stays a path
    /// the host answers, which is what keeps every script written before
    /// vectors existed working unchanged.
    Component(usize),
    /// Building a vector: `Vec2(x, y)` or `Vec3(x, y, z)`, by its size.
    Construct(usize),
    /// A vector property or method: `v.length`, `a.dot(b)`.
    Vector(decay_syntax::VectorOp),
    /// A text property or method: `s.length`, `s.contains(part)`.
    Text(decay_syntax::StringOp),
    /// Starting a timer: `Timer(seconds)`.
    StartTimer,
    /// A timer's property: `t.done`, `t.left`.
    Timer(decay_syntax::TimerProperty),
    /// An enum's variant, `Phase.Lobby`: a value the language knows, pushed
    /// as a constant rather than asked of the host. The names are the
    /// expression's own.
    Variant,
}

/// Which member read, call or construction the language performs itself, by
/// where it was written.
pub type ValueMembers = HashMap<Span, ValueMember>;

#[derive(Debug, Clone, PartialEq)]
pub struct Analysis {
    pub program: Program,
    pub diagnostics: Vec<Diagnostic>,
    /// Where the program reads a member of a value, and which one.
    ///
    /// Keyed by the member expression's span, which is unique per expression
    /// and is what the lowering walk has in hand when it reaches one.
    pub value_members: ValueMembers,
    /// Every enum the program could name, with its variants in order: its
    /// own and the host's. What a host needs to author one of its fields by a
    /// variant's name, without reading the project again.
    pub enums: std::collections::BTreeMap<String, Vec<String>>,
}

/// Decay has no methods, and the mistake of reaching for one is worth naming
/// precisely rather than leaving as a runtime `FunctionNotFound`.
pub(crate) fn container_function_message(field: &str) -> String {
    format!(
        "`{field}` is this script's own function; call it as `{field}(...)` rather than `this.{field}(...)`"
    )
}

pub(crate) fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let prefix = &source[..offset.min(source.len())];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or(prefix.len() + 1, |(_, tail)| tail.len() + 1);
    (line, column)
}
