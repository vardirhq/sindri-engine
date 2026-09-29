use crate::Span;

mod value_ops;

pub use value_ops::{
    ColorOp, ListOp, MapOp, NumberOp, StringOp, TimerProperty, VectorOp, parse_hex,
};

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
    /// `const ARENA: f32 = 12.0;`: a name for a value worked out when the
    /// file compiles. Written `shared const`, every file in the project may
    /// use it.
    Const(ConstDecl),
}

/// A declared constant: its name, its type, and the expression that gives its
/// value, which may use only literals, operators and other constants.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub name: String,
    pub shared: bool,
    pub ty: TypeRef,
    pub value: Expr,
    /// Where the name was written, for a diagnostic about it.
    pub name_span: Span,
    pub span: Span,
}

/// A declared enum: its name, and its variants in order.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<(String, Span)>,
    pub span: Span,
}

/// A declared struct: its name, its fields in order, and its methods.
#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
    /// `fn heavier(other: Card) -> bool { ... }` after the fields: a function
    /// asked of one value, `card.heavier(best)`, with `this` the value.
    pub methods: Vec<FunctionDecl>,
    pub span: Span,
}

/// One field of a struct: a name and the type it holds.
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub ty: TypeRef,
    /// `weight: f32 = 1.0`: what the field holds when a struct is built
    /// without naming it. Worked out when the file compiles, like a constant.
    pub default: Option<Expr>,
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
    /// The type argument a name may carry, as in `Array<Entity>`, or a map's
    /// key type in `Map<String, f32>`.
    ///
    /// Named slots rather than a list, and deliberately: the only generic
    /// types the language has are its collections, a list taking one type and
    /// a map two. A list would be a promise of user-defined generics, which
    /// `LANGUAGE.md` says plainly the language does not have.
    pub argument: Option<Box<TypeRef>>,
    /// A map's value type, the second in `Map<String, f32>`.
    pub second: Option<Box<TypeRef>>,
    /// `f32?`: a value of the type, or `null`.
    pub optional: bool,
    pub span: Span,
}

impl TypeRef {
    /// A plain named type with no argument.
    #[must_use]
    pub const fn plain(name: String, span: Span) -> Self {
        Self {
            name,
            argument: None,
            second: None,
            optional: false,
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

/// One arm of a `match` written as a value: the patterns it accepts, and the
/// value it gives.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchValueArm {
    pub patterns: Vec<Pattern>,
    pub value: Expr,
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
    /// `match phase { Phase.Lobby => "Waiting", _ => "Go" }`: the value of
    /// the first arm whose pattern the subject is. Written where a value goes;
    /// at the start of a statement, `match` runs blocks instead.
    Match {
        subject: Box<Expr>,
        arms: Vec<MatchValueArm>,
    },
    /// `["a": 1.0, "b": 2.0]`, or `[:]` for an empty one: a map's entries,
    /// each key then its value, in the order written.
    Map(Vec<(Expr, Expr)>),
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
    /// `value ?? fallback`: the value, or the fallback when it is `null`.
    /// Short-circuits: the fallback is only worked out when it is needed.
    Fallback,
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
