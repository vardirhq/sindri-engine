//! A script's tree, as blocks.

use decay_syntax::{AssignOp, ContainerDecl, ExprKind, Item, Member, Stmt, parse};

use super::slots::{slot, span, text};
use super::{Arm, Block, Container, Field, Handler, Problem, VisualScript};

/// `source`, a `.decay` script, as blocks. Always answers: a script with
/// syntax errors shows what the parser could read, and its problems.
#[must_use]
pub fn blocks(source: &str) -> VisualScript {
    let parsed = parse(source);
    let containers = parsed
        .program
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Script(container) => Some(("script", container)),
            Item::Component(container) => Some(("component", container)),
            _ => None,
        })
        .map(|(kind, container)| self::container(source, kind, container))
        .collect();
    VisualScript {
        containers,
        problems: parsed
            .diagnostics
            .iter()
            .map(|diagnostic| Problem {
                message: diagnostic.message.clone(),
                line: diagnostic.line,
                column: diagnostic.column,
            })
            .collect(),
    }
}

fn container(source: &str, kind: &'static str, container: &ContainerDecl) -> Container {
    let mut fields = Vec::new();
    let mut handlers = Vec::new();
    // Comments above a member are its note: from the end of the one before
    // (or the container's opening brace) to where it starts.
    let mut after = source
        .get(container.span.start..container.span.end)
        .and_then(|text| text.find('{'))
        .map_or(container.span.start, |brace| {
            container.span.start + brace + 1
        });
    for member in &container.members {
        match member {
            Member::Field(field) => {
                fields.push(Field {
                    name: field.name.clone(),
                    exported: field
                        .attributes
                        .iter()
                        .any(|attribute| attribute.name == "export"),
                    mutable: field.mutable,
                    ty: field.ty.as_ref().map(|ty| ty.name.clone()),
                    value: field.initializer.as_ref().map(|value| slot(source, value)),
                    note: note(source, after, field.span.start),
                    span: span(field.span),
                });
                after = field.span.end;
            }
            Member::Function(function) => {
                let lifecycle = sindri_decay::LIFECYCLE
                    .iter()
                    .any(|known| known.name == function.name);
                let (label, role) = match (&function.handles, function.name.as_str()) {
                    (Some((event, _)), _) => (format!("When {event}"), "event"),
                    (None, "start") => ("When it starts".to_owned(), "event"),
                    (None, "update") => ("Every frame".to_owned(), "event"),
                    (None, name) if lifecycle => (format!("When {name}"), "event"),
                    (None, name) => (name.to_owned(), "function"),
                };
                handlers.push(Handler {
                    name: function.name.clone(),
                    label,
                    role,
                    params: function
                        .params
                        .iter()
                        .map(|param| param.name.clone())
                        .collect(),
                    body: statements(source, &function.body),
                    note: note(source, after, function.span.start),
                    span: span(function.span),
                });
                after = function.span.end;
            }
        }
    }
    Container {
        kind,
        name: container.name.clone(),
        fields,
        handlers,
        span: span(container.span),
    }
}

/// A block's statements as blocks, with the comments between them.
fn statements(source: &str, block: &decay_syntax::Block) -> Vec<Block> {
    let mut out = Vec::new();
    let mut after = block.span.start + 1;
    for statement in &block.statements {
        let at = statement.span();
        comments(source, after, at.start, &mut out);
        out.push(self::statement(source, statement));
        after = at.end;
    }
    comments(source, after, block.span.end.saturating_sub(1), &mut out);
    out
}

fn statement(source: &str, statement: &Stmt) -> Block {
    let at = span(statement.span());
    match statement {
        Stmt::Expr { expr, .. } => match &expr.kind {
            ExprKind::Assign {
                target,
                op: AssignOp::Assign,
                value,
            } => Block::Set {
                target: slot(source, target),
                value: slot(source, value),
                span: at,
            },
            ExprKind::Assign { target, op, value } => Block::Change {
                target: slot(source, target),
                op: match op {
                    AssignOp::Add => "+=",
                    AssignOp::Subtract => "-=",
                    AssignOp::Multiply => "*=",
                    AssignOp::Divide => "/=",
                    AssignOp::Modulo => "%=",
                    AssignOp::Assign => "=",
                },
                value: slot(source, value),
                span: at,
            },
            ExprKind::Call { .. } => Block::Do {
                call: slot(source, expr),
                span: at,
            },
            _ => code(source, statement),
        },
        Stmt::Binding {
            mutable,
            name,
            initializer,
            ..
        } => Block::Let {
            name: name.clone(),
            mutable: *mutable,
            value: initializer.as_ref().map(|value| slot(source, value)),
            span: at,
        },
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => Block::If {
            condition: slot(source, condition),
            then: statements(source, then_branch),
            otherwise: else_branch.as_ref().map(|block| else_blocks(source, block)),
            span: at,
        },
        Stmt::While {
            condition, body, ..
        } => Block::While {
            condition: slot(source, condition),
            body: statements(source, body),
            span: at,
        },
        Stmt::For {
            name,
            iterable,
            body,
            ..
        } => Block::For {
            name: name.clone(),
            items: slot(source, iterable),
            body: statements(source, body),
            span: at,
        },
        Stmt::Match { subject, arms, .. } => Block::Match {
            subject: slot(source, subject),
            arms: arms
                .iter()
                .map(|arm| Arm {
                    patterns: arm
                        .patterns
                        .iter()
                        .map(|pattern| text(source, pattern.span()))
                        .collect(),
                    body: statements(source, &arm.body),
                })
                .collect(),
            span: at,
        },
        Stmt::Return { value, .. } => Block::Return {
            value: value.as_ref().map(|value| slot(source, value)),
            span: at,
        },
        Stmt::Break { .. } => Block::Break { span: at },
        Stmt::Continue { .. } => Block::Continue { span: at },
        Stmt::Block(_) => code(source, statement),
    }
}

/// An `else`: its block, or for `else if`, which the parser keeps as a block
/// holding one `If` with no braces of its own, that `If`.
fn else_blocks(source: &str, block: &decay_syntax::Block) -> Vec<Block> {
    match block.statements.as_slice() {
        [only @ Stmt::If { .. }] if only.span() == block.span => vec![statement(source, only)],
        _ => statements(source, block),
    }
}

fn code(source: &str, statement: &Stmt) -> Block {
    Block::Code {
        source: text(source, statement.span()),
        span: span(statement.span()),
    }
}

/// The `//` lines between `from` and `to`, as one comment block.
fn comments(source: &str, from: usize, to: usize, out: &mut Vec<Block>) {
    if let Some((text, start, end)) = comment_lines(source, from, to) {
        out.push(Block::Comment {
            text,
            span: [start, end],
        });
    }
}

/// The `//` lines between `from` and `to`, joined, for a member's note.
fn note(source: &str, from: usize, to: usize) -> Option<String> {
    comment_lines(source, from, to).map(|(text, _, _)| text)
}

fn comment_lines(source: &str, from: usize, to: usize) -> Option<(String, usize, usize)> {
    let gap = source.get(from..to.max(from))?;
    let mut lines = Vec::new();
    let (mut start, mut end) = (None, 0);
    let mut offset = from;
    for line in gap.split_inclusive('\n') {
        if let Some(at) = line.find("//") {
            let comment = line[at + 2..].trim_end_matches(['\n', '\r']);
            lines.push(comment.strip_prefix(' ').unwrap_or(comment).to_owned());
            start.get_or_insert(offset + at);
            end = offset + line.trim_end_matches(['\n', '\r']).len();
        }
        offset += line.len();
    }
    Some((lines.join("\n"), start?, end))
}
