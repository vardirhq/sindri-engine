//! One function per expression form, and the instructions it becomes.

use decay_semantic::ValueMember;
use decay_syntax::{AssignOp, BinaryOp, Expr, ExprKind};

use crate::ir::{Constant, Instruction, Path};

use super::Lowerer;

/// The local a computed reference is held in while a path is walked from it.
/// Not a name a script can write, so it shadows nothing; it is what a
/// runtime error about a null reference names.
const HELD: &str = "(reference)";

impl Lowerer<'_> {
    /// `&&` and `||`, as branches rather than as an operation over two values
    /// already evaluated.
    ///
    /// This is the whole of short-circuiting: the right operand is lowered
    /// behind a jump the left operand can take, so a left that already decides
    /// the answer skips it. It matters most where it is least visible — a
    /// guard such as `held != null && World.exists(held)` reads as protecting
    /// the call to its right, and only does when the call is skipped.
    ///
    /// The answer is pushed as a constant rather than left as whichever
    /// operand survived, because `&&` and `||` are typed `bool` and should go
    /// on producing one. `JumpIfFalse` pops what it tested and refuses a value
    /// that is not a `bool`, so testing each operand that is reached keeps the
    /// check `Instruction::Binary` used to perform on both.
    pub(super) fn lower_short_circuit(
        &self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
        instructions: &mut Vec<Instruction>,
    ) {
        if op == BinaryOp::Fallback {
            self.lower_fallback(left, right, instructions);
            return;
        }
        // Filled with the sites that cannot know their target until the shared
        // tail below exists, and patched once it does.
        let mut to_false = Vec::new();
        let mut to_true = Vec::new();

        self.lower_expr(left, instructions);
        if matches!(op, BinaryOp::And) {
            to_false.push(instructions.len());
            instructions.push(Instruction::JumpIfFalse(usize::MAX));
        } else {
            // `||` is decided by a *true* left operand, and the only test
            // available jumps on false — so the jump that skips the right
            // operand is the one taken by falling through.
            let past = instructions.len();
            instructions.push(Instruction::JumpIfFalse(usize::MAX));
            to_true.push(instructions.len());
            instructions.push(Instruction::Jump(usize::MAX));
            let right_start = instructions.len();
            instructions[past] = Instruction::JumpIfFalse(right_start);
        }

        self.lower_expr(right, instructions);
        to_false.push(instructions.len());
        instructions.push(Instruction::JumpIfFalse(usize::MAX));

        let true_target = instructions.len();
        instructions.push(Instruction::Push(Constant::Bool(true)));
        let to_end = instructions.len();
        instructions.push(Instruction::Jump(usize::MAX));

        let false_target = instructions.len();
        instructions.push(Instruction::Push(Constant::Bool(false)));
        let end = instructions.len();

        for site in to_false {
            instructions[site] = Instruction::JumpIfFalse(false_target);
        }
        for site in to_true {
            instructions[site] = Instruction::Jump(true_target);
        }
        instructions[to_end] = Instruction::Jump(end);
    }

    /// `value ?? fallback`: the value is worked out once and held; the
    /// fallback is worked out only when the value is `null`.
    fn lower_fallback(&self, value: &Expr, fallback: &Expr, instructions: &mut Vec<Instruction>) {
        const VALUE: &str = "(value)";
        instructions.push(Instruction::ScopeEnter);
        self.lower_expr(value, instructions);
        instructions.push(Instruction::Declare {
            name: VALUE.to_owned(),
            mutable: false,
        });
        instructions.push(Instruction::Load(Path(vec![VALUE.to_owned()])));
        instructions.push(Instruction::Push(Constant::Null));
        instructions.push(Instruction::Binary(BinaryOp::Equal));
        let to_value = instructions.len();
        instructions.push(Instruction::JumpIfFalse(usize::MAX));
        self.lower_expr(fallback, instructions);
        let to_end = instructions.len();
        instructions.push(Instruction::Jump(usize::MAX));
        instructions[to_value] = Instruction::JumpIfFalse(instructions.len());
        instructions.push(Instruction::Load(Path(vec![VALUE.to_owned()])));
        instructions[to_end] = Instruction::Jump(instructions.len());
        instructions.push(Instruction::ScopeExit);
    }

    pub(super) fn lower_expr(&self, expr: &Expr, instructions: &mut Vec<Instruction>) {
        match &expr.kind {
            ExprKind::Identifier(name) => {
                if let Some(constant) = self.constant_at(expr.span) {
                    instructions.push(Instruction::Push(constant));
                } else {
                    instructions.push(Instruction::Load(Path(vec![name.clone()])));
                }
            }
            ExprKind::Number(value) => {
                instructions.push(Instruction::Push(Constant::Number(*value)));
            }
            ExprKind::String(value) => {
                instructions.push(Instruction::Push(Constant::String(value.clone())));
            }
            ExprKind::Bool(value) => instructions.push(Instruction::Push(Constant::Bool(*value))),
            // A range is refused by the analysis anywhere but a `for`, which
            // lowers its own, so nothing reaches here from a program that
            // compiled.
            ExprKind::Null | ExprKind::Range { .. } => {
                instructions.push(Instruction::Push(Constant::Null));
            }
            ExprKind::Group(inner) => self.lower_expr(inner, instructions),
            ExprKind::Match { subject, arms } => {
                self.lower_match_value(subject, arms, instructions);
            }
            ExprKind::Unary { op, expr } => {
                self.lower_expr(expr, instructions);
                instructions.push(Instruction::Unary(*op));
            }
            ExprKind::Binary { left, op, right } => {
                if matches!(op, BinaryOp::And | BinaryOp::Or | BinaryOp::Fallback) {
                    self.lower_short_circuit(left, *op, right, instructions);
                } else {
                    self.lower_expr(left, instructions);
                    self.lower_expr(right, instructions);
                    instructions.push(Instruction::Binary(*op));
                }
            }
            ExprKind::Assign { target, op, value } => {
                self.lower_assign(target, *op, value, instructions);
            }
            ExprKind::Member { object, .. } => {
                // The analysis says whether this reads a property of a value or
                // walks a path the host answers. Only it knows: both are
                // spelled `a.b`, and telling them apart by the member's name
                // would reserve that name against every host type there is.
                match self.value_member(expr.span) {
                    Some(ValueMember::Length) => {
                        self.lower_expr(object, instructions);
                        instructions.push(Instruction::Length);
                    }
                    Some(ValueMember::Component(index)) => {
                        self.lower_expr(object, instructions);
                        instructions.push(Instruction::Component(index));
                    }
                    Some(ValueMember::Vector(op)) => {
                        self.lower_expr(object, instructions);
                        instructions.push(Instruction::Vector(op));
                    }
                    Some(ValueMember::Text(op)) => {
                        self.lower_expr(object, instructions);
                        instructions.push(Instruction::Text(op));
                    }
                    Some(ValueMember::Variant) => {
                        let variant = Self::path_from_expr(expr)
                            .map_or_else(|| "<invalid-variant>".to_owned(), |path| path.dotted());
                        instructions.push(Instruction::Push(Constant::Variant(variant)));
                    }
                    Some(ValueMember::Timer(property)) => {
                        self.lower_expr(object, instructions);
                        instructions.push(Instruction::Timer(property));
                    }
                    // Only a call is ever noted as a construction.
                    Some(
                        ValueMember::Construct(_)
                        | ValueMember::StartTimer
                        | ValueMember::List(_)
                        | ValueMember::Map(_)
                        | ValueMember::Number(_),
                    )
                    | None => {
                        let (path, held) =
                            self.path_or_held(expr, "<invalid-member>", instructions);
                        instructions.push(Instruction::Load(path));
                        Self::release(held, instructions);
                    }
                }
            }
            ExprKind::Index { object, index } => {
                self.lower_expr(object, instructions);
                self.lower_expr(index, instructions);
                instructions.push(Instruction::Index);
            }
            ExprKind::Call { callee, args } => self.lower_call(expr, callee, args, instructions),
            ExprKind::Construct { name, fields } => {
                self.lower_construct(name, fields, instructions);
            }
            ExprKind::List(elements) => {
                for element in elements {
                    self.lower_expr(element, instructions);
                }
                instructions.push(Instruction::MakeList(elements.len()));
            }
            ExprKind::Map(entries) => self.lower_map(entries, instructions),
        }
    }

    /// `["a": 1.0]`: each key then its value, then the map of them.
    fn lower_map(&self, entries: &[(Expr, Expr)], instructions: &mut Vec<Instruction>) {
        for (key, value) in entries {
            self.lower_expr(key, instructions);
            self.lower_expr(value, instructions);
        }
        instructions.push(Instruction::MakeMap(entries.len()));
    }

    /// A list's or map's operation: a change is made in place to the one the
    /// place holds, and a question is asked of the value.
    fn lower_collection_call(
        &self,
        member: ValueMember,
        object: &Expr,
        args: &[Expr],
        instructions: &mut Vec<Instruction>,
    ) {
        let changes = match member {
            ValueMember::List(op) => op.changes(),
            ValueMember::Map(op) => op.changes(),
            _ => return,
        };
        let place = if changes {
            Some(self.list_place(object))
        } else {
            self.lower_expr(object, instructions);
            None
        };
        for argument in args {
            self.lower_expr(argument, instructions);
        }
        instructions.push(match (member, place) {
            (ValueMember::List(op), Some((path, fields))) => {
                Instruction::ListChange { path, fields, op }
            }
            (ValueMember::Map(op), Some((path, fields))) => {
                Instruction::MapChange { path, fields, op }
            }
            (ValueMember::List(op), None) => Instruction::ListRead(op),
            (ValueMember::Map(op), None) => Instruction::MapRead(op),
            _ => return,
        });
    }

    /// A call: building a vector, a vector method, or a call by path.
    fn lower_call(
        &self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        instructions: &mut Vec<Instruction>,
    ) {
        // A struct's method: the value it is asked of, then the arguments,
        // to the function it was lowered to.
        if let Some(function) = self.method_call(expr.span)
            && let ExprKind::Member { object, .. } = &callee.kind
        {
            self.lower_expr(object, instructions);
            for argument in args {
                self.lower_expr(argument, instructions);
            }
            instructions.push(Instruction::Call {
                callee: Path(vec![function.to_owned()]),
                argument_count: args.len() + 1,
            });
            return;
        }
        match self.value_member(expr.span) {
            Some(ValueMember::Construct(dimensions)) => {
                for argument in args {
                    self.lower_expr(argument, instructions);
                }
                instructions.push(Instruction::Construct(dimensions));
                return;
            }
            Some(ValueMember::StartTimer) => {
                for argument in args {
                    self.lower_expr(argument, instructions);
                }
                instructions.push(Instruction::StartTimer);
                return;
            }
            Some(ValueMember::Vector(op)) => {
                if let ExprKind::Member { object, .. } = &callee.kind {
                    self.lower_expr(object, instructions);
                }
                for argument in args {
                    self.lower_expr(argument, instructions);
                }
                instructions.push(Instruction::Vector(op));
                return;
            }
            Some(member @ (ValueMember::List(_) | ValueMember::Map(_))) => {
                if let ExprKind::Member { object, .. } = &callee.kind {
                    self.lower_collection_call(member, object, args, instructions);
                }
                return;
            }
            Some(ValueMember::Text(op)) => {
                if let ExprKind::Member { object, .. } = &callee.kind {
                    self.lower_expr(object, instructions);
                }
                for argument in args {
                    self.lower_expr(argument, instructions);
                }
                instructions.push(Instruction::Text(op));
                return;
            }
            Some(ValueMember::Number(op)) => {
                if let ExprKind::Member { object, .. } = &callee.kind {
                    self.lower_expr(object, instructions);
                }
                for argument in args {
                    self.lower_expr(argument, instructions);
                }
                instructions.push(Instruction::Number(op));
                return;
            }
            _ => {}
        }
        // The receiver before the arguments: it is evaluated first, and held
        // before anything else is on the stack.
        let (callee, held) = self.path_or_held(callee, "<invalid-call>", instructions);
        for argument in args {
            self.lower_expr(argument, instructions);
        }
        instructions.push(Instruction::Call {
            callee,
            argument_count: args.len(),
        });
        Self::release(held, instructions);
    }

    /// The path an expression walks, when it walks one from something that
    /// is not a name: `Bolt.on(hit).damage`, `find().kick()`.
    ///
    /// The value it starts from is computed once and held in a local nobody
    /// can name, and the path starts there instead, so the runtime treats it
    /// as it treats any local holding a reference. `true` when a local was
    /// opened, which [`Self::release`] then closes.
    pub(super) fn path_or_held(
        &self,
        expr: &Expr,
        invalid: &str,
        instructions: &mut Vec<Instruction>,
    ) -> (Path, bool) {
        if let Some(path) = Self::path_from_expr(expr) {
            return (path, false);
        }
        let mut fields = Vec::new();
        let mut root = expr;
        loop {
            match &root.kind {
                ExprKind::Member { object, field } => {
                    fields.push(field.clone());
                    root = object;
                }
                ExprKind::Group(inner) => root = inner,
                _ => break,
            }
        }
        if fields.is_empty() {
            return (Path(vec![invalid.into()]), false);
        }
        fields.push(HELD.to_owned());
        fields.reverse();
        instructions.push(Instruction::ScopeEnter);
        self.lower_expr(root, instructions);
        instructions.push(Instruction::Declare {
            name: HELD.to_owned(),
            mutable: false,
        });
        (Path(fields), true)
    }

    /// Closes the local [`Self::path_or_held`] opened, if it opened one.
    pub(super) fn release(held: bool, instructions: &mut Vec<Instruction>) {
        if held {
            instructions.push(Instruction::ScopeExit);
        }
    }

    /// `v.x = value`, or `v.x += value`: the vector with one component
    /// replaced, stored back, and the component left as the expression's value.
    /// `Card(name: n, root: e)`: the values in the order written, and the
    /// struct built with each in its declared place.
    fn lower_construct(
        &self,
        name: &str,
        fields: &[(String, decay_syntax::Span, Expr)],
        instructions: &mut Vec<Instruction>,
    ) {
        let declared = self.struct_fields(name);
        let mut order = Vec::with_capacity(fields.len());
        for (field, _, value) in fields {
            self.lower_expr(value, instructions);
            order.push(
                declared
                    .iter()
                    .position(|known| known == field)
                    .unwrap_or(0),
            );
        }
        // Then each field left out, as its default.
        for (position, field) in declared.iter().enumerate() {
            if fields.iter().any(|(given, _, _)| given == field) {
                continue;
            }
            if let Some(default) = self.default_of(name, field) {
                instructions.push(Instruction::Push(default));
                order.push(position);
            }
        }
        instructions.push(Instruction::MakeStruct {
            shape: std::rc::Rc::new(crate::ir::StructShape {
                name: name.to_owned(),
                fields: declared,
            }),
            order,
        });
    }

    pub(super) fn path_from_expr(expr: &Expr) -> Option<Path> {
        fn collect(expr: &Expr, parts: &mut Vec<String>) -> bool {
            match &expr.kind {
                ExprKind::Identifier(name) => {
                    parts.push(name.clone());
                    true
                }
                ExprKind::Member { object, field } => {
                    if !collect(object, parts) {
                        return false;
                    }
                    parts.push(field.clone());
                    true
                }
                ExprKind::Group(inner) => collect(inner, parts),
                _ => false,
            }
        }

        let mut parts = Vec::new();
        collect(expr, &mut parts).then_some(Path(parts))
    }
}

/// The arithmetic a compound assignment performs; `None` for plain `=`.
pub(super) const fn binary_for(op: AssignOp) -> Option<BinaryOp> {
    match op {
        AssignOp::Assign => None,
        AssignOp::Add => Some(BinaryOp::Add),
        AssignOp::Subtract => Some(BinaryOp::Subtract),
        AssignOp::Multiply => Some(BinaryOp::Multiply),
        AssignOp::Divide => Some(BinaryOp::Divide),
        AssignOp::Modulo => Some(BinaryOp::Modulo),
    }
}
