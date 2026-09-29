//! Assignments: to a variable, a field, a list's element, a map's entry, a
//! vector's component or a struct's field, and the place each one changes.

use super::Lowerer;
use crate::ir::{Instruction, Path};
use decay_semantic::ValueMember;
use decay_syntax::{AssignOp, Expr, ExprKind};

use super::expr::binary_for;

impl Lowerer<'_> {
    /// `target = value`, or a compound assignment: to a list's element, a
    /// vector's component, or a path.
    pub(super) fn lower_assign(
        &self,
        target: &Expr,
        op: AssignOp,
        value: &Expr,
        instructions: &mut Vec<Instruction>,
    ) {
        if let ExprKind::Index { object, index } = &target.kind {
            let map = matches!(
                self.value_member(target.span),
                Some(ValueMember::Map(decay_syntax::MapOp::Set))
            );
            self.lower_element_assign((object, index, map), op, value, instructions);
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
        (object, index, map): (&Expr, &Expr, bool),
        op: AssignOp,
        value: &Expr,
        instructions: &mut Vec<Instruction>,
    ) {
        const INDEX: &str = "(index)";
        let (path, fields) = self.list_place(object);
        // A map's entry is set as a list's element is, by the key.
        let change = if map {
            Instruction::MapChange {
                path: path.clone(),
                fields: fields.clone(),
                op: decay_syntax::MapOp::Set,
            }
        } else {
            Instruction::ListChange {
                path: path.clone(),
                fields: fields.clone(),
                op: decay_syntax::ListOp::SetAt,
            }
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
    pub(super) fn list_place(&self, object: &Expr) -> (Path, Vec<usize>) {
        let (root, fields) = self.place_chain(object);
        let path =
            Self::path_from_expr(root).unwrap_or_else(|| Path(vec!["<invalid-list>".to_owned()]));
        (path, fields)
    }
}
