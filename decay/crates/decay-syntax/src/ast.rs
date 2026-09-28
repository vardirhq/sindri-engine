use crate::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Script(ContainerDecl),
    Component(ContainerDecl),
    /// `event GoalScored(team: f32);`: something that happens, which any
    /// script may emit and any script may handle.
    Event(EventDecl),
    /// `state Game { var score: f32 = 0.0; }`: values every script shares,
    /// reached as `Game.score`.
    State(StateDecl),
    /// `enum Phase { Lobby, Countdown, Play }`: a type whose values are the
    /// variants it names, and nothing else.
    Enum(EnumDecl),
    /// `struct Card { root: Entity, name: String }`: a value made of named,
    /// typed fields.
    Struct(StructDecl),
    /// `fn half(size: f32) -> f32 { ... }` outside any container: a function
    /// the file's scripts call by name, with no `this`. Written `shared fn`,
    /// every script in the project may call it.
    Function(FunctionDecl),
}

/// A declared enum: its name, and its variants in order.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<(String, Span)>,
    pub span: Span,
}

/// A declared struct: its name, and its fields in order.
#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
    pub span: Span,
}

/// One field of a struct: a name and the type it holds.
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub ty: TypeRef,
    pub span: Span,
}

/// Shared, typed values under one name. Several declarations may add to the
/// same name, as long as no field is declared twice.
#[derive(Debug, Clone, PartialEq)]
pub struct StateDecl {
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

/// A declared event: its name, and what it carries.
#[derive(Debug, Clone, PartialEq)]
pub struct EventDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub span: Span,
}

/// The name a handler for `event` is lowered and called under.
///
/// Not a name a script can write, because it has a space in it: a handler can
/// neither be called by name nor collide with a function the script declares.
#[must_use]
pub fn handler_name(event: &str) -> String {
    format!("on {event}")
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerDecl {
    pub name: String,
    pub members: Vec<Member>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Member {
    Field(FieldDecl),
    Function(FunctionDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub attributes: Vec<Attribute>,
    pub mutable: bool,
    pub name: String,
    pub ty: Option<TypeRef>,
    pub initializer: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeRef {
    pub name: String,
    /// The one type argument a name may carry, as in `Array<Entity>`.
    ///
    /// One rather than a list, and deliberately: the only generic type the
    /// language has is the collection, and it takes exactly one element type.
    /// A list would be a promise of user-defined generics, which
    /// `LANGUAGE.md` says plainly the language does not have.
    pub argument: Option<Box<TypeRef>>,
    pub span: Span,
}

impl TypeRef {
    /// A plain named type with no argument.
    #[must_use]
    pub const fn plain(name: String, span: Span) -> Self {
        Self {
            name,
            argument: None,
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: String,
    /// For `on GoalScored(team: f32) { ... }`, the event it handles and where
    /// that was written. Its `name` is then [`handler_name`] of the event.
    pub handles: Option<(String, Span)>,
    /// `shared fn`: a function outside any container that every file may
    /// call, rather than only the one declaring it. Always false inside one.
    pub shared: bool,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Option<TypeRef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Binding {
        mutable: bool,
        name: String,
        ty: Option<TypeRef>,
        initializer: Option<Expr>,
        span: Span,
    },
    Expr {
        expr: Expr,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    If {
        condition: Expr,
        then_branch: Block,
        /// A chained `else if` is parsed as a block holding one `If`, so the
        /// tree has one shape of conditional rather than two.
        else_branch: Option<Block>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },
    /// `for name in items { ... }` over a collection.
    ///
    /// The binding is immutable and scoped to the body: a loop variable is what
    /// the collection holds at that position, not a place to put something.
    For {
        name: String,
        name_span: Span,
        iterable: Expr,
        body: Block,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    /// `match subject { Phase.Lobby => { ... } _ => { ... } }`: the first arm
    /// whose pattern the subject is runs.
    Match {
        subject: Expr,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Block(Block),
}

/// One arm of a `match`: the patterns it accepts, and what it runs.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub patterns: Vec<Pattern>,
    pub body: Block,
    pub span: Span,
}

/// What a `match` arm accepts.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// `Phase.Lobby`: one variant, always written with its enum's name.
    Variant {
        enumeration: String,
        variant: String,
        span: Span,
    },
    /// `_`: anything the arms above did not take.
    Wildcard(Span),
}

impl Pattern {
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Variant { span, .. } | Self::Wildcard(span) => *span,
        }
    }
}

impl Stmt {
    /// Where the statement is, whichever form it takes.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Binding { span, .. }
            | Self::Expr { span, .. }
            | Self::Return { span, .. }
            | Self::If { span, .. }
            | Self::While { span, .. }
            | Self::For { span, .. }
            | Self::Match { span, .. }
            | Self::Break { span }
            | Self::Continue { span } => *span,
            Self::Block(block) => block.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Identifier(String),
    Number(f64),
    String(String),
    Bool(bool),
    Null,
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Assign {
        target: Box<Expr>,
        op: AssignOp,
        value: Box<Expr>,
    },
    Member {
        object: Box<Expr>,
        field: String,
    },
    /// `items[index]`, over a list.
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Group(Box<Expr>),
    /// `Card(root: e, name: "Arc")`: a struct, every field named.
    Construct {
        name: String,
        fields: Vec<(String, Span, Expr)>,
    },
    /// `[a, b, c]`: a list of what is written, in order.
    List(Vec<Expr>),
    /// `start..end`: the whole numbers from `start` up to, not including,
    /// `end`. Written only as what a `for` walks.
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
    },
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}
