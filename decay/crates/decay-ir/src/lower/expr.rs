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
            ExprKind::Unary { op, expr } => {
                self.lower_expr(expr, instructions);
                instructions.push(Instruction::Unary(*op));
            }
            ExprKind::Binary { left, op, right } => {
                if matches!(op, BinaryOp::And | BinaryOp::Or) {
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
        }
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
            Some(ValueMember::List(op)) => {
                let ExprKind::Member { object, .. } = &callee.kind else {
                    return;
                };
                if op.changes() {
                    let (path, fields) = self.list_place(object);
                    for argument in args {
                        self.lower_expr(argument, instructions);
                    }
                    instructions.push(Instruction::ListChange { path, fields, op });
                } else {
                    self.lower_expr(object, instructions);
                    for argument in args {
                        self.lower_expr(argument, instructions);
                    }
                    instructions.push(Instruction::ListRead(op));
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
    fn path_or_held(
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
    fn release(held: bool, instructions: &mut Vec<Instruction>) {
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
        instructions.push(Instruction::MakeStruct {
            shape: std::rc::Rc::new(crate::ir::StructShape {
                name: name.to_owned(),
                fields: declared,
            }),
            order,
        });
    }

    /// `target = value`, or a compound assignment: to a list's element, a
    /// vector's component, or a path.
    fn lower_assign(
        &self,
        target: &Expr,
        op: AssignOp,
        value: &Expr,
        instructions: &mut Vec<Instruction>,
    ) {
        if let ExprKind::Index { object, index } = &target.kind {
            self.lower_element_assign(object, index, op, value, instructions);
            return;
        }
        if let ExprKind::Member { object, .. } = &target.kind
            && let Some(ValueMember::Component(index)) = self.value_member(target.span)
        {
            self.lower_component_assign(object, index, op, value, instructions);
            return;
        }
        let (path, held) = self.path_or_held(target, "<invalid>", instructions);
        if matches!(op, AssignOp::Assign) {
            self.lower_expr(value, instructions);
        } else {
            instructions.push(Instruction::Load(path.clone()));
            self.lower_expr(value, instructions);
            if let Some(binary) = binary_for(op) {
                instructions.push(Instruction::Binary(binary));
            }
        }
        instructions.push(Instruction::Store(path));
        Self::release(held, instructions);
    }

    /// `list[index] = value`, or `list[index] += value`: set in place.
    ///
    /// A compound assignment reads the element before setting it, so the
    /// index is evaluated once and held, as a `match` holds its subject.
    fn lower_element_assign(
        &self,
        object: &Expr,
        index: &Expr,
        op: AssignOp,
        value: &Expr,
        instructions: &mut Vec<Instruction>,
    ) {
        const INDEX: &str = "(index)";
        let (path, fields) = self.list_place(object);
        let change = Instruction::ListChange {
            path: path.clone(),
            fields: fields.clone(),
            op: decay_syntax::ListOp::SetAt,
        };
        let Some(binary) = binary_for(op) else {
            self.lower_expr(index, instructions);
            self.lower_expr(value, instructions);
            instructions.push(change);
            return;
        };
        let held = Path(vec![INDEX.to_owned()]);
        instructions.push(Instruction::ScopeEnter);
        self.lower_expr(index, instructions);
        instructions.push(Instruction::Declare {
            name: INDEX.to_owned(),
            mutable: false,
        });
        instructions.push(Instruction::Load(held.clone()));
        instructions.push(Instruction::Load(path));
        for field in fields {
            instructions.push(Instruction::Component(field));
        }
        instructions.push(Instruction::Load(held));
        instructions.push(Instruction::Index);
        self.lower_expr(value, instructions);
        instructions.push(Instruction::Binary(binary));
        instructions.push(change);
        instructions.push(Instruction::ScopeExit);
    }

    /// `object.field = value`, where `field` is a vector's component or a
    /// struct's field at `index`, and `object` is a place or a field or
    /// component inside one: `v.x`, `card.weight`, `slot.offer.weight`.
    ///
    /// A value is changed by making the changed one and putting it back, so
    /// each level from the place down is loaded, the innermost changed, and
    /// each level rebuilt around it on the way back up before the place is
    /// stored.
    fn lower_component_assign(
        &self,
        object: &Expr,
        index: usize,
        op: AssignOp,
        value: &Expr,
        instructions: &mut Vec<Instruction>,
    ) {
        let (root, chain) = self.place_chain(object);
        let (path, held) = self.path_or_held(root, "<invalid>", instructions);
        let load_down_to = |depth: usize, instructions: &mut Vec<Instruction>| {
            instructions.push(Instruction::Load(path.clone()));
            for step in &chain[..depth] {
                instructions.push(Instruction::Component(*step));
            }
        };
        // The place, then each level inside it down to the one changed.
        for depth in 0..=chain.len() {
            load_down_to(depth, instructions);
        }
        if let Some(binary) = binary_for(op) {
            load_down_to(chain.len(), instructions);
            instructions.push(Instruction::Component(index));
            self.lower_expr(value, instructions);
            instructions.push(Instruction::Binary(binary));
        } else {
            self.lower_expr(value, instructions);
        }
        instructions.push(Instruction::WithComponent(index));
        for step in chain.iter().rev() {
            instructions.push(Instruction::WithComponent(*step));
        }
        instructions.push(Instruction::Store(path.clone()));
        instructions.push(Instruction::Pop);
        load_down_to(chain.len(), instructions);
        instructions.push(Instruction::Component(index));
        Self::release(held, instructions);
    }

    /// The place an expression is inside, and the fields or components that
    /// lead from it down to the expression, outermost first: `slot.offer`
    /// is `slot` and the position of `offer`.
    pub(super) fn place_chain<'e>(&self, expr: &'e Expr) -> (&'e Expr, Vec<usize>) {
        let mut chain = Vec::new();
        let mut at = expr;
        loop {
            match &at.kind {
                ExprKind::Group(inner) => at = inner,
                ExprKind::Member { object, .. } => {
                    let Some(ValueMember::Component(index)) = self.value_member(at.span) else {
                        break;
                    };
                    chain.push(index);
                    at = object;
                }
                _ => break,
            }
        }
        chain.reverse();
        (at, chain)
    }

    /// The place a list change is made to, and the fields that lead from it
    /// down to the list.
    fn list_place(&self, object: &Expr) -> (Path, Vec<usize>) {
        let (root, fields) = self.place_chain(object);
        let path =
            Self::path_from_expr(root).unwrap_or_else(|| Path(vec!["<invalid-list>".to_owned()]));
        (path, fields)
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
const fn binary_for(op: AssignOp) -> Option<BinaryOp> {
    match op {
        AssignOp::Assign => None,
        AssignOp::Add => Some(BinaryOp::Add),
        AssignOp::Subtract => Some(BinaryOp::Subtract),
        AssignOp::Multiply => Some(BinaryOp::Multiply),
        AssignOp::Divide => Some(BinaryOp::Divide),
        AssignOp::Modulo => Some(BinaryOp::Modulo),
    }
}
