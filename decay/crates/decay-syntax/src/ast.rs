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
    Block(Block),
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
    /// `items[index]`, over a collection the host handed back.
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Group(Box<Expr>),
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
