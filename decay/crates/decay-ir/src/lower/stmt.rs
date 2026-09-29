//! Statements and blocks, and the scopes a block opens.

use decay_syntax::{BinaryOp, Block, Pattern, Stmt};

use crate::ir::{Constant, Instruction, Path};

use super::Lowerer;

/// One enclosing `while`, and what the statements inside it need to know.
struct LoopContext {
    /// Where `continue` goes, which for a `while` is its condition. Known
    /// before the body is lowered, so it needs no patching.
    continue_target: usize,
    /// The `break` sites, patched once the loop's end exists.
    breaks: Vec<usize>,
    /// How many scopes were open when the loop began. A `break` or `continue`
    /// closes the difference before it jumps.
    scope_depth: usize,
    /// Whether leaving this loop early has a walk to close.
    ///
    /// True for `for`, false for `while`. A `break` out of a `for` has to let
    /// go of the collection it was walking; a `break` out of a `while` has
    /// nothing to let go of, and emitting the close anyway would unbalance an
    /// enclosing loop's walk.
    walks: bool,
}

/// The scope depth and the open loops, carried down the statement tree.
///
/// Both are needed for one reason: a jump out of a loop skips the `ScopeExit`
/// instructions between it and the loop, and unlike a `Return` — which takes
/// the whole frame with it — a `break` leaves the frame running. So the exits
/// it skips are emitted before it jumps, and that requires knowing how many
/// there are.
#[derive(Default)]
pub(super) struct Loops {
    depth: usize,
    open: Vec<LoopContext>,
}

impl Loops {
    /// Closes every scope opened since the innermost loop began. Used before a
    /// `break` or `continue` jumps out of them.
    fn unwind_to_loop(&self, instructions: &mut Vec<Instruction>) -> Option<usize> {
        let context = self.open.last()?;
        for _ in context.scope_depth..self.depth {
            instructions.push(Instruction::ScopeExit);
        }
        Some(context.continue_target)
    }
}

impl Lowerer<'_> {
    pub(super) fn lower_block(
        &self,
        block: &Block,
        instructions: &mut Vec<Instruction>,
        loops: &mut Loops,
    ) {
        for statement in &block.statements {
            self.lower_stmt(statement, instructions, loops);
        }
    }

    /// A block that introduces a scope of its own.
    ///
    /// Every branch is wrapped rather than only the ones that declare
    /// something, so the enter and exit stay paired however the jumps around
    /// them are patched. A `Return` leaving a scope unclosed costs nothing:
    /// the frame goes with it.
    pub(super) fn lower_scoped_block(
        &self,
        block: &Block,
        instructions: &mut Vec<Instruction>,
        loops: &mut Loops,
    ) {
        instructions.push(Instruction::ScopeEnter);
        loops.depth += 1;
        self.lower_block(block, instructions, loops);
        loops.depth -= 1;
        instructions.push(Instruction::ScopeExit);
    }

    /// `for name in items { ... }`.
    fn lower_for(
        &self,
        name: &str,
        iterable: &decay_syntax::Expr,
        body: &Block,
        instructions: &mut Vec<Instruction>,
        loops: &mut Loops,
    ) {
        // The collection is evaluated once, before the loop, and the
        // walk over it lives beside the value stack. A script that
        // reassigns the name it came from does not change what this
        // loop is walking, which is the behaviour a counted lowering
        // would have got wrong.
        if let decay_syntax::ExprKind::Range { start, end } = &iterable.kind {
            self.lower_expr(start, instructions);
            self.lower_expr(end, instructions);
            instructions.push(Instruction::IterRange);
        } else {
            self.lower_expr(iterable, instructions);
            instructions.push(Instruction::IterBegin);
        }

        // Where `continue` goes: taking the next element is both the
        // test and the step, so one target serves both.
        let next = instructions.len();
        instructions.push(Instruction::IterNext(usize::MAX));

        loops.open.push(LoopContext {
            continue_target: next,
            breaks: Vec::new(),
            scope_depth: loops.depth,
            walks: true,
        });
        instructions.push(Instruction::ScopeEnter);
        loops.depth += 1;
        instructions.push(Instruction::Declare {
            name: name.to_owned(),
            // An element is what the collection holds at that position,
            // not a place to put something.
            mutable: false,
        });
        self.lower_block(body, instructions, loops);
        loops.depth -= 1;
        instructions.push(Instruction::ScopeExit);
        let context = loops.open.pop().expect("the loop just pushed is open");
        instructions.push(Instruction::Jump(next));

        // A `break` has already closed its walk, so it lands past the
        // exhaustion path rather than sharing it.
        let broke = instructions.len();
        for site in context.breaks {
            instructions[site] = Instruction::Jump(broke);
        }
        instructions[next] = Instruction::IterNext(broke);
    }

    pub(super) fn lower_stmt(
        &self,
        statement: &Stmt,
        instructions: &mut Vec<Instruction>,
        loops: &mut Loops,
    ) {
        match statement {
            Stmt::Binding {
                mutable,
                name,
                initializer,
                ..
            } => {
                // The initializer is evaluated before the name exists, which is
                // also what the language means: a binding cannot refer to
                // itself.
                if let Some(initializer) = initializer {
                    self.lower_expr(initializer, instructions);
                } else {
                    instructions.push(Instruction::Push(Constant::Null));
                }
                instructions.push(Instruction::Declare {
                    name: name.clone(),
                    mutable: *mutable,
                });
            }
            Stmt::Expr { expr, .. } => {
                self.lower_expr(expr, instructions);
                instructions.push(Instruction::Pop);
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.lower_expr(value, instructions);
                }
                instructions.push(Instruction::Return);
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.lower_expr(condition, instructions);
                let jump_if_false = instructions.len();
                instructions.push(Instruction::JumpIfFalse(usize::MAX));

                self.lower_scoped_block(then_branch, instructions, loops);

                if let Some(else_branch) = else_branch {
                    let jump_to_end = instructions.len();
                    instructions.push(Instruction::Jump(usize::MAX));
                    let else_start = instructions.len();
                    instructions[jump_if_false] = Instruction::JumpIfFalse(else_start);
                    self.lower_scoped_block(else_branch, instructions, loops);
                    let end = instructions.len();
                    instructions[jump_to_end] = Instruction::Jump(end);
                } else {
                    let end = instructions.len();
                    instructions[jump_if_false] = Instruction::JumpIfFalse(end);
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                // The condition is re-evaluated every turn, so it is also where
                // `continue` goes: one target serves both.
                let condition_start = instructions.len();
                self.lower_expr(condition, instructions);
                let jump_if_false = instructions.len();
                instructions.push(Instruction::JumpIfFalse(usize::MAX));

                loops.open.push(LoopContext {
                    continue_target: condition_start,
                    breaks: Vec::new(),
                    scope_depth: loops.depth,
                    walks: false,
                });
                self.lower_scoped_block(body, instructions, loops);
                let context = loops.open.pop().expect("the loop just pushed is open");

                instructions.push(Instruction::Jump(condition_start));
                let end = instructions.len();
                instructions[jump_if_false] = Instruction::JumpIfFalse(end);
                for site in context.breaks {
                    instructions[site] = Instruction::Jump(end);
                }
            }
            Stmt::Break { .. } => {
                // The analyzer refuses a `break` outside a loop, so there is
                // always one to leave; nothing is emitted if it did not.
                if loops.unwind_to_loop(instructions).is_some() {
                    if loops.open.last().is_some_and(|context| context.walks) {
                        instructions.push(Instruction::IterEnd);
                    }
                    let site = instructions.len();
                    instructions.push(Instruction::Jump(usize::MAX));
                    if let Some(context) = loops.open.last_mut() {
                        context.breaks.push(site);
                    }
                }
            }
            Stmt::Continue { .. } => {
                if let Some(condition) = loops.unwind_to_loop(instructions) {
                    instructions.push(Instruction::Jump(condition));
                }
            }
            Stmt::For {
                name,
                iterable,
                body,
                ..
            } => self.lower_for(name, iterable, body, instructions, loops),
            Stmt::Block(block) => self.lower_scoped_block(block, instructions, loops),
            Stmt::Match { subject, arms, .. } => {
                self.lower_match(subject, arms, instructions, loops);
            }
        }
    }

    /// `match subject { ... }`: the subject is evaluated once and held, and
    /// each arm compares it with its patterns in turn; the first that equals
    /// runs, and nothing else does.
    ///
    /// A subject that is none of the patterns — only `null` can be, since the
    /// analysis refuses a `match` that misses a variant — runs no arm.
    fn lower_match(
        &self,
        subject: &decay_syntax::Expr,
        arms: &[decay_syntax::MatchArm],
        instructions: &mut Vec<Instruction>,
        loops: &mut Loops,
    ) {
        instructions.push(Instruction::ScopeEnter);
        loops.depth += 1;
        self.lower_expr(subject, instructions);
        instructions.push(Instruction::Declare {
            name: MATCHED.to_owned(),
            mutable: false,
        });
        let mut to_end = Vec::new();
        for arm in arms {
            let mut to_body = Vec::new();
            let mut to_next_pattern = None;
            let mut always = false;
            for pattern in &arm.patterns {
                if let Some(site) = to_next_pattern.take() {
                    instructions[site] = Instruction::JumpIfFalse(instructions.len());
                }
                match pattern {
                    Pattern::Wildcard(_) => {
                        always = true;
                        break;
                    }
                    Pattern::Variant {
                        enumeration,
                        variant,
                        ..
                    } => {
                        instructions.push(Instruction::Load(Path(vec![MATCHED.to_owned()])));
                        instructions.push(Instruction::Push(Constant::Variant(format!(
                            "{enumeration}.{variant}"
                        ))));
                        instructions.push(Instruction::Binary(BinaryOp::Equal));
                        to_next_pattern = Some(instructions.len());
                        instructions.push(Instruction::JumpIfFalse(usize::MAX));
                        to_body.push(instructions.len());
                        instructions.push(Instruction::Jump(usize::MAX));
                    }
                }
            }
            let body = instructions.len();
            for site in to_body {
                instructions[site] = Instruction::Jump(body);
            }
            self.lower_scoped_block(&arm.body, instructions, loops);
            to_end.push(instructions.len());
            instructions.push(Instruction::Jump(usize::MAX));
            // The last pattern that did not match goes to the next arm.
            if let Some(site) = to_next_pattern {
                instructions[site] = Instruction::JumpIfFalse(instructions.len());
            }
            if always {
                break;
            }
        }
        let end = instructions.len();
        for site in to_end {
            instructions[site] = Instruction::Jump(end);
        }
        loops.depth -= 1;
        instructions.push(Instruction::ScopeExit);
    }
}

impl Lowerer<'_> {
    /// `match subject { Phase.Lobby => "Waiting", _ => "Go" }` as a value: as
    /// the statement, but each arm leaves its value on the stack and jumps to
    /// the end. A subject no pattern takes — only `null` can be, since the
    /// analysis refuses a `match` that misses a variant — gives `null`.
    pub(super) fn lower_match_value(
        &self,
        subject: &decay_syntax::Expr,
        arms: &[decay_syntax::MatchValueArm],
        instructions: &mut Vec<Instruction>,
    ) {
        instructions.push(Instruction::ScopeEnter);
        self.lower_expr(subject, instructions);
        instructions.push(Instruction::Declare {
            name: MATCHED.to_owned(),
            mutable: false,
        });
        let mut to_end = Vec::new();
        let mut always = false;
        for arm in arms {
            let mut to_value = Vec::new();
            let mut to_next_pattern = None;
            for pattern in &arm.patterns {
                if let Some(site) = to_next_pattern.take() {
                    instructions[site] = Instruction::JumpIfFalse(instructions.len());
                }
                match pattern {
                    Pattern::Wildcard(_) => {
                        always = true;
                        break;
                    }
                    Pattern::Variant {
                        enumeration,
                        variant,
                        ..
                    } => {
                        instructions.push(Instruction::Load(Path(vec![MATCHED.to_owned()])));
                        instructions.push(Instruction::Push(Constant::Variant(format!(
                            "{enumeration}.{variant}"
                        ))));
                        instructions.push(Instruction::Binary(BinaryOp::Equal));
                        to_next_pattern = Some(instructions.len());
                        instructions.push(Instruction::JumpIfFalse(usize::MAX));
                        to_value.push(instructions.len());
                        instructions.push(Instruction::Jump(usize::MAX));
                    }
                }
            }
            let value = instructions.len();
            for site in to_value {
                instructions[site] = Instruction::Jump(value);
            }
            self.lower_expr(&arm.value, instructions);
            to_end.push(instructions.len());
            instructions.push(Instruction::Jump(usize::MAX));
            if let Some(site) = to_next_pattern {
                instructions[site] = Instruction::JumpIfFalse(instructions.len());
            }
            if always {
                break;
            }
        }
        if !always {
            instructions.push(Instruction::Push(Constant::Null));
        }
        let end = instructions.len();
        for site in to_end {
            instructions[site] = Instruction::Jump(end);
        }
        instructions.push(Instruction::ScopeExit);
    }
}

/// The local a `match` holds its subject in. Not a name a script can write.
const MATCHED: &str = "(matched)";
