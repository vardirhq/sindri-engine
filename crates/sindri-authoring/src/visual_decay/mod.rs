//! Decay, but visual: a `.decay` script seen as blocks, and edited as blocks,
//! without ever being anything but the script.
//!
//! The file is the only thing stored. There is no graph asset beside it to
//! drift out of step, and nothing is converted: [`blocks`] reads the script
//! through Decay's own parser and shows each handler as a "When …" block and
//! each statement as a block, with its expressions in slots; [`edit`] changes
//! a script by replacing exactly the text of the one node a block edit
//! touched, then parses the result and refuses it if it no longer reads.
//! Everything else in the file — comments, spacing, code the blocks have no
//! shape for — stays as it was, byte for byte. So a script written by hand on
//! desktop opens as blocks on the phone, and a script edited as blocks on the
//! phone reads on desktop exactly as it would have if typed there.
//!
//! The same reading is drawn as a node graph ([`graph`]), Blueprint style:
//! event nodes, execution wires between statement nodes, value nodes wired
//! into pins. Where nodes sit is the one thing a script does not hold, so it
//! is kept beside it ([`layout`]), never in it.
//!
//! What has no block of its own yet is shown as a code block holding its exact
//! source, so nothing is ever hidden or lost; comments between statements
//! are shown as comment blocks.

mod edit;
mod graph;
pub mod layout;
mod project;
mod slots;

use serde::Serialize;

pub use edit::{ScriptEdit, edit};
pub use graph::{Graph, Node, Pin, Wire, graph};
pub use layout::Layout;
pub use project::blocks;

/// Where something is in the script: its first byte and the byte after it.
pub type Span = [usize; 2];

/// A script, as blocks.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VisualScript {
    /// Each `script` or `component` in the file.
    pub containers: Vec<Container>,
    /// What the parser could not read. With any, the blocks show what it did
    /// read, and editing is refused until the text is fixed.
    pub problems: Vec<Problem>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Problem {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// A `script` or `component`: its tunable and private values, and its
/// handlers.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Container {
    /// `script` or `component`.
    pub kind: &'static str,
    pub name: String,
    pub fields: Vec<Field>,
    pub handlers: Vec<Handler>,
    pub span: Span,
}

/// A value the script keeps: `@export let speed: number = 5.0;`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Field {
    pub name: String,
    /// `@export`: tuned from the inspector, per object.
    pub exported: bool,
    /// `var` rather than `let`.
    pub mutable: bool,
    #[serde(rename = "type")]
    pub ty: Option<String>,
    pub value: Option<Slot>,
    /// The comment lines written just above it.
    pub note: Option<String>,
    pub span: Span,
}

/// A function of the script: an engine event (`start`, `update`), an event
/// handler (`on Lit`) or a function of its own, and what it does.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Handler {
    pub name: String,
    /// How the block reads: "When it starts", "Every frame", "When Lit".
    pub label: String,
    /// `event` for what the engine or another script calls, `function` for
    /// one the script calls itself.
    pub role: &'static str,
    pub params: Vec<String>,
    pub body: Vec<Block>,
    /// The comment lines written just above it.
    pub note: Option<String>,
    pub span: Span,
}

/// One statement, as a block.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Block {
    /// `target = value;`
    Set {
        target: Slot,
        value: Slot,
        span: Span,
    },
    /// `target += value;` and the other compound assignments: `op` is `+=` …
    Change {
        target: Slot,
        op: &'static str,
        value: Slot,
        span: Span,
    },
    /// `let name = value;` or `var name = value;`
    Let {
        name: String,
        mutable: bool,
        value: Option<Slot>,
        span: Span,
    },
    /// A call standing as a statement: `World.emit(Lit());`
    Do {
        call: Slot,
        span: Span,
    },
    If {
        condition: Slot,
        then: Vec<Block>,
        /// `else`; an `else if` is a block holding one `If`, as in the tree.
        otherwise: Option<Vec<Block>>,
        span: Span,
    },
    While {
        condition: Slot,
        body: Vec<Block>,
        span: Span,
    },
    For {
        name: String,
        items: Slot,
        body: Vec<Block>,
        span: Span,
    },
    Match {
        subject: Slot,
        arms: Vec<Arm>,
        span: Span,
    },
    Return {
        value: Option<Slot>,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    /// A statement with no block of its own yet, as its exact source.
    Code {
        source: String,
        span: Span,
    },
    /// `// …` lines between statements.
    Comment {
        text: String,
        span: Span,
    },
}

/// One arm of a `match` block.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Arm {
    /// As written: `Phase.Lobby`, `_`.
    pub patterns: Vec<String>,
    pub body: Vec<Block>,
}

/// An expression, in the slot of a block.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Slot {
    /// A number, edited in place. A negative literal (`-1.0`) is one number.
    Number {
        value: f64,
        span: Span,
    },
    /// A string, edited in place.
    Text {
        value: String,
        span: Span,
    },
    /// `true` or `false`, flipped in place.
    Bool {
        value: bool,
        span: Span,
    },
    Null {
        span: Span,
    },
    /// A name or a path to one: `dt`, `this.speed`, `Game.lit`.
    Name {
        path: String,
        span: Span,
    },
    /// `callee(args…)`, such as `max(0.0, this.grace - dt)`.
    Call {
        callee: String,
        args: Vec<Slot>,
        span: Span,
    },
    /// `left op right`, such as `this.grace - dt` or `a && b`.
    Op {
        op: &'static str,
        left: Box<Slot>,
        right: Box<Slot>,
        span: Span,
    },
    /// `!value` or `-value`.
    Unary {
        op: &'static str,
        operand: Box<Slot>,
        span: Span,
    },
    /// An expression with no slot shape of its own yet, as its exact source.
    Code {
        source: String,
        span: Span,
    },
}

impl Slot {
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Number { span, .. }
            | Self::Text { span, .. }
            | Self::Bool { span, .. }
            | Self::Null { span }
            | Self::Name { span, .. }
            | Self::Call { span, .. }
            | Self::Op { span, .. }
            | Self::Unary { span, .. }
            | Self::Code { span, .. } => *span,
        }
    }
}

#[cfg(test)]
mod graph_tests;
#[cfg(test)]
mod tests;
