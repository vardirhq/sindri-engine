//! The instruction set, and the shapes a lowered program is made of.
//!
//! Symbolic and engine-agnostic on purpose: this knows about Decay
//! control flow, values, names, member paths, and calls, and nothing
//! about what a Transform or an Entity is.

use decay_syntax::{BinaryOp, TimerProperty, UnaryOp, VectorOp};

#[derive(Debug, Clone, PartialEq)]
pub struct IrProgram {
    pub containers: Vec<IrContainer>,
}

/// The name of the container a program's top-level functions are lowered
/// into: its own file's, and any `shared fn` a host linked in from another.
/// Not a name a script can write, so it can never be a script's.
pub const FUNCTIONS: &str = "(functions)";

impl IrProgram {
    /// The top-level functions this program can call: its own file's, and
    /// any `shared fn` a host linked in from other files.
    #[must_use]
    pub fn functions(&self) -> Option<&IrContainer> {
        self.containers
            .iter()
            .find(|container| container.kind == ContainerKind::Functions)
    }

    /// Adds functions another file shares, so this program can call them.
    /// One this program already has is kept: its own declaration is the one
    /// it was checked against.
    pub fn link(&mut self, functions: impl IntoIterator<Item = IrFunction>) {
        let index = if let Some(index) = self
            .containers
            .iter()
            .position(|container| container.kind == ContainerKind::Functions)
        {
            index
        } else {
            self.containers.push(IrContainer {
                kind: ContainerKind::Functions,
                name: FUNCTIONS.to_owned(),
                fields: Vec::new(),
                functions: Vec::new(),
            });
            self.containers.len() - 1
        };
        let shared = &mut self.containers[index];
        for function in functions {
            if !shared
                .functions
                .iter()
                .any(|known| known.name == function.name)
            {
                shared.functions.push(function);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    Script,
    Component,
    /// A program's top-level functions: no fields, no `this`, called by name.
    Functions,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrContainer {
    pub kind: ContainerKind,
    pub name: String,
    pub fields: Vec<IrField>,
    pub functions: Vec<IrFunction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrField {
    pub name: String,
    pub mutable: bool,
    pub exported: bool,
    pub type_name: Option<String>,
    pub initializer: Option<Vec<Instruction>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<String>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path(pub Vec<String>);

impl Path {
    #[must_use]
    pub fn dotted(&self) -> String {
        self.0.join(".")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Push(Constant),
    Load(Path),
    /// Assigns to a name that already exists, and is subject to its
    /// mutability. Binding a new name is [`Instruction::Declare`].
    Store(Path),
    /// Pops the initial value and binds it to a new name in the innermost
    /// scope.
    ///
    /// Declaring takes the value rather than leaving the slot empty for a
    /// following `Store`, because a `let` binding's own initialization would
    /// then have to be an exception to the runtime's immutability rule. It was
    /// not, so every `let` local failed at runtime; taking the value here means
    /// there is no initializing store to make an exception for.
    Declare {
        name: String,
        mutable: bool,
    },
    Unary(UnaryOp),
    Binary(BinaryOp),
    /// Pops an index and a collection, pushes the element at that index.
    ///
    /// The index is the language's one numeric type. There is no integer type,
    /// so a fractional or out-of-range index is refused here, where the value
    /// is known, rather than by a type nobody wanted to introduce.
    Index,
    /// Pops a collection, pushes how many elements it holds.
    ///
    /// An instruction rather than a path load, because there is no host to ask
    /// what the length of a value is. The analyzer says which member reads are
    /// this one; see `decay_semantic::ValueMember`.
    Length,
    /// Pops a collection and opens a walk over it.
    ///
    /// A `for` loop is lowered to this rather than to an index and a counter,
    /// so the collection is evaluated once and the loop cannot be confused by
    /// a script that reassigns the name it came from. The walk lives beside
    /// the value stack rather than on it, so no value a script could hold ever
    /// represents "part way through a loop".
    IterBegin,
    /// Takes the next element of the innermost walk and pushes it.
    ///
    /// When the collection is spent it closes the walk and jumps to the given
    /// instruction instead, so the ordinary path out of a loop needs no
    /// separate cleanup.
    IterNext(usize),
    /// Closes the innermost walk without finishing it.
    ///
    /// What a `break` emits: leaving a loop early still has to let go of what
    /// it was walking.
    IterEnd,
    /// Pops this many numbers and pushes the vector they make, `x` first.
    Construct(usize),
    /// Pops a vector, pushes its component at this position (`x` is 0).
    Component(usize),
    /// Pops a number and a vector, pushes the vector with the component at
    /// this position replaced.
    ///
    /// What `v.x = 3.0` lowers to, with a load before and a store after: a
    /// vector is a value, so changing part of one is making a new one and
    /// putting it back where the old one was.
    WithComponent(usize),
    /// A vector property or method. Pops the arguments, then the vector, and
    /// pushes the answer.
    Vector(VectorOp),
    /// Pops a number of seconds, pushes a timer that runs out after them.
    StartTimer,
    /// Pops a timer, pushes one of its properties.
    Timer(TimerProperty),
    Call {
        callee: Path,
        argument_count: usize,
    },
    Pop,
    Return,
    JumpIfFalse(usize),
    Jump(usize),
    /// Opens a nested scope, so that a name declared inside a block leaves
    /// with it.
    ///
    /// The analyzer has always scoped blocks; without these the IR did not,
    /// and a shadowing declaration overwrote the name it shadowed for the rest
    /// of the function.
    ScopeEnter,
    /// Closes the scope [`Instruction::ScopeEnter`] opened, discarding what it
    /// declared.
    ScopeExit,
}
