//! What stops a script, and what it says about why.

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    /// A path was rooted at something the script holds that is not a reference,
    /// so there is nothing for the rest of the path to be about.
    NotAReference(String),
    /// A path was rooted at a reference that is empty. Reaching through nothing
    /// is a mistake worth naming, rather than silently doing nothing.
    NullReference(String),
    ContainerNotFound(String),
    FunctionNotFound(String),
    Arity {
        function: String,
        expected: usize,
        found: usize,
    },
    UnknownPath(String),
    Immutable(String),
    StackUnderflow,
    InvalidUnary,
    InvalidBinary,
    ExpectedBool,
    /// Something that is not a collection was indexed, walked, or asked for
    /// its length. Named by what it actually is, because "not a collection" on
    /// its own does not say which value was wrong.
    NotACollection(String),
    /// Something that is not a vector was asked for a component or a vector
    /// operation, named by what it actually is.
    NotAVector(String),
    /// Something that is not a timer was read as one, or a timer was started
    /// from something that is not a number of seconds, named by what it was.
    NotATimer(String),
    /// Something that is not text was asked a text question, or joined to
    /// text when it has no spelling, named by what it was.
    NotText(String),
    /// Something that is not a number was written as one, `n.fixed(2)`, or
    /// given as a count of digits, named by what it was.
    NotANumber(String),
    /// Joining or replacing would make text longer than
    /// [`crate::TEXT_LIMIT`] bytes.
    TextTooLong {
        limit: usize,
    },
    /// Adding to a list would make it longer than
    /// [`crate::LIST_LIMIT`] elements.
    ListTooLong {
        limit: usize,
    },
    /// An index that is not a number at all.
    IndexNotANumber(String),
    /// An index that is a number and not a position: fractional, negative, or
    /// not finite. Decay has one numeric type, so this is a property of the
    /// value rather than of its type, and it is checked where the value is.
    IndexNotWhole(f64),
    /// `map[key]` for a key the map does not have, named as written.
    MissingKey(String),
    /// `Color(text)` for text that spells no colour: `#rrggbb` or
    /// `#rrggbbaa`.
    InvalidColor(String),
    /// `n.fixed(digits)` or `n.padded(width)` asked for a count of digits
    /// that is not whole, is negative, or is more than `most`.
    DigitsOutOfRange {
        asked: f64,
        most: usize,
    },
    IndexOutOfRange {
        index: usize,
        length: usize,
    },
    InvalidJump(usize),
    /// A script called deeper than [`Runtime::call_depth_limit`] allows.
    ///
    /// Without it, unbounded recursion overflowed the host's own stack and
    /// aborted the process — which, for a runtime meant to execute author
    /// scripts inside the editor, takes the editor and any unsaved work with
    /// it. A limit turns that into a value a caller can report.
    CallDepthExceeded {
        function: String,
        limit: usize,
    },
    /// A script ran longer than [`Runtime::operation_budget`] allows.
    ///
    /// The call-depth limit bounds recursion, which was the only way to run
    /// forever before loops existed. `while` removed that guarantee: a loop
    /// that never ends uses no extra stack and would simply never return, which
    /// inside the editor means a frame that never finishes. The budget is what
    /// makes a loop safe to offer at all — it turns a runaway script into a
    /// reported failure for one entity, the way every other runtime error is.
    OperationBudgetExceeded {
        limit: usize,
    },
    Host(String),
}
