//! Changing a script's shape from its nodes: deleting a statement, moving it
//! past its neighbor, adding one after it or first inside a branch.
//!
//! Each is still a text edit: whole lines are cut, swapped or added, at the
//! indentation the code around them already uses, with the comment lines
//! attached above a statement travelling with it, as the node shows them as
//! its note. Nothing outside those lines changes.

use decay_syntax::{Block, Item, Member as Member_, Program, Stmt, parse};

use decay_semantic::{Environment, ExternalSymbol, Type};

use super::members::{Member, add, is_name, member_span, placeholders, rename};
use super::{Span, VisualScript};

/// A statement a node menu can add, written as the code around it would be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Template {
    /// `this.field = <its declared value>;`
    Set { field: String, value: String },
    /// `if this.field { }`, or `if !this.field { }`.
    If { field: String, negated: bool },
    /// `function(…);`, one of the script's own functions, its inputs given
    /// placeholders.
    Call {
        function: String,
        arguments: Vec<String>,
    },
    /// `let name = 0;`, a value the steps after it can read.
    Let { name: String },
    /// `return;`
    Return,
    /// `// text`
    Comment { text: String },
    /// One of the engine's calls with placeholder arguments: `Audio.play("", 0);`.
    Engine { call: String },
}

impl Template {
    /// The template a node menu names `id`: `set:speed`, `if:lit`,
    /// `ifnot:lit`, `call:ignite`, `return` or `comment`, for `script`.
    #[must_use]
    pub fn named(id: &str, script: &VisualScript, source: &str) -> Option<Self> {
        let (kind, name) = id.split_once(':').unwrap_or((id, ""));
        let field = || {
            script
                .containers
                .iter()
                .flat_map(|container| &container.fields)
                .find(|field| field.name == name)
        };
        match kind {
            "set" => {
                let value = field()?.value.as_ref()?.span();
                Some(Self::Set {
                    field: name.to_owned(),
                    value: source.get(value[0]..value[1])?.to_owned(),
                })
            }
            "if" | "ifnot" => field().map(|_| Self::If {
                field: name.to_owned(),
                negated: kind == "ifnot",
            }),
            "call" => Some(Self::Call {
                function: name.to_owned(),
                arguments: placeholders(source, name)?,
            }),
            "let" => is_name(name).then(|| Self::Let {
                name: name.to_owned(),
            }),
            "return" => Some(Self::Return),
            "comment" => Some(Self::Comment {
                text: "Note".to_owned(),
            }),
            _ => None,
        }
    }

    /// The engine call `engine:World.find` names, from
    /// [`engine_calls`] for `environment`.
    #[must_use]
    pub fn engine(id: &str, environment: &Environment) -> Option<Self> {
        engine_calls(environment)
            .into_iter()
            .find(|(choice, _)| choice == id)
            .map(|(_, call)| Self::Engine { call })
    }

    /// Its lines, before indentation.
    fn lines(&self) -> Vec<String> {
        match self {
            Self::Set { field, value } => vec![format!("this.{field} = {value};")],
            Self::If { field, negated } => vec![
                format!("if {}this.{field} {{", if *negated { "!" } else { "" }),
                "}".to_owned(),
            ],
            Self::Call {
                function,
                arguments,
            } => vec![format!("{function}({});", arguments.join(", "))],
            Self::Let { name } => vec![format!("let {name} = 0;")],
            Self::Return => vec!["return;".to_owned()],
            Self::Comment { text } => vec![format!("// {text}")],
            Self::Engine { call } => vec![format!("{call};")],
        }
    }
}

/// What can be added from a script's node menu: each choice's id, for
/// [`Template::named`], and how the menu words it.
#[must_use]
pub fn palette(script: &VisualScript) -> Vec<(String, String)> {
    let mut choices = Vec::new();
    for container in &script.containers {
        for field in &container.fields {
            if field.value.is_some() {
                choices.push((format!("set:{}", field.name), format!("Set {}", field.name)));
            }
            if field.ty.as_deref() == Some("bool") {
                choices.push((format!("if:{}", field.name), format!("If {}", field.name)));
                choices.push((
                    format!("ifnot:{}", field.name),
                    format!("If not {}", field.name),
                ));
            }
        }
        for handler in &container.handlers {
            if handler.role == "function" {
                choices.push((
                    format!("call:{}", handler.name),
                    format!("Call {}", handler.name),
                ));
            }
        }
    }
    choices.push(("let".to_owned(), "Remember a value".to_owned()));
    choices.push(("return".to_owned(), "Return".to_owned()));
    choices.push(("comment".to_owned(), "Comment".to_owned()));
    choices
}

/// The engine's calls a step can make, for a finder: each one's id, for
/// [`Template::engine`], and its call with placeholder arguments, such as
/// `Audio.play("", 0)`. Only calls whose arguments all have a plain placeholder
/// (a number, text, switch, vector, or the current entity) are offered,
/// sorted by name. Other argument types stay unavailable until they can be
/// authored safely.
#[must_use]
pub fn engine_calls(environment: &Environment) -> Vec<(String, String)> {
    let mut calls = Vec::new();
    for (global, symbol) in environment.globals() {
        let (ExternalSymbol::Value(Type::Named(ty))
        | ExternalSymbol::ReadOnlyValue(Type::Named(ty))) = symbol
        else {
            continue;
        };
        let Some(host) = environment.get_type(ty) else {
            continue;
        };
        for (name, member) in host.members() {
            let ExternalSymbol::Function(function) = member else {
                continue;
            };
            let arguments: Option<Vec<&str>> = function
                .params
                .iter()
                .map(|param| match param {
                    Type::Number => Some("0"),
                    Type::Bool => Some("false"),
                    Type::String => Some("\"\""),
                    Type::Vec2 => Some("Vec2(0, 0)"),
                    Type::Vec3 => Some("Vec3(0, 0, 0)"),
                    Type::Named(name) if name == "Entity" => Some("this.entity"),
                    _ => None,
                })
                .collect();
            if let Some(arguments) = arguments {
                calls.push((
                    format!("engine:{global}.{name}"),
                    format!("{global}.{name}({})", arguments.join(", ")),
                ));
            }
        }
    }
    calls.sort();
    calls
}

/// A change to a script's shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    /// The statement at this span, and the comment lines attached above it.
    Delete(Span),
    /// The statement at this span swapped with the one before it, or after.
    Move { statement: Span, up: bool },
    /// `template` added after the statement at this span.
    After { statement: Span, template: Template },
    /// `template` added first in a block: a handler's body (at the handler's
    /// span), or a branch of the statement at this span, by its output's
    /// index (an If's True is 0 and False 1; a loop's Body is 0).
    Inside {
        node: Span,
        branch: usize,
        template: Template,
    },
    /// The value in a pin at this span replaced by one of the script's
    /// fields, read as `this.field`.
    Field { slot: Span, field: String },
    /// Replace an object pin with a lookup by its scene name.
    FindObject { slot: Span, name: String },
    /// A new member of the script: written first among its fields for a
    /// variable, last for a function or event.
    Add(Member),
    /// The variable or function `name` taken out of the script, with the
    /// comment lines above it.
    Remove { name: String },
    /// The variable or function `from` renamed `to`, everywhere this script
    /// names it.
    Rename { from: String, to: String },
}

/// `source` with `shape` made, before it is checked.
pub(super) fn reshape(source: &str, shape: &Shape) -> Result<String, String> {
    let program = parse(source).program;
    match shape {
        Shape::Delete(statement) => {
            let found = locate(&program, *statement).ok_or(GONE)?;
            let [start, end] = lines_of(source, found.siblings[found.index], true);
            Ok(format!("{}{}", &source[..start], &source[end..]))
        }
        Shape::Move { statement, up } => {
            let found = locate(&program, *statement).ok_or(GONE)?;
            let other = if *up {
                found.index.checked_sub(1)
            } else {
                Some(found.index + 1).filter(|next| *next < found.siblings.len())
            }
            .ok_or(if *up {
                "it is already first"
            } else {
                "it is already last"
            })?;
            let (first, second) = if *up {
                (other, found.index)
            } else {
                (found.index, other)
            };
            let [a_start, a_end] = lines_of(source, found.siblings[first], true);
            let [b_start, b_end] = lines_of(source, found.siblings[second], true);
            Ok(format!(
                "{}{}{}{}{}",
                &source[..a_start],
                &source[b_start..b_end],
                &source[a_end..b_start],
                &source[a_start..a_end],
                &source[b_end..]
            ))
        }
        Shape::After {
            statement,
            template,
        } => {
            let found = locate(&program, *statement).ok_or(GONE)?;
            let [_, end] = lines_of(source, found.siblings[found.index], false);
            let indent = indentation(source, statement[0]);
            let text = indented(template, &indent);
            Ok(if source[..end].ends_with('\n') {
                format!("{}{indent}{text}\n{}", &source[..end], &source[end..])
            } else {
                format!("{}\n{indent}{text}{}", &source[..end], &source[end..])
            })
        }
        Shape::Field { slot, field } => use_field(source, *slot, field),
        Shape::FindObject { slot, name } => find_object(source, *slot, name),
        Shape::Add(member) => add(source, &program, member),
        Shape::Remove { name } => {
            let span = member_span(&program, name)?;
            let [start, end] = lines_of(source, span, true);
            Ok(format!("{}{}", &source[..start], &source[end..]))
        }
        Shape::Rename { from, to } => rename(source, &program, from, to),
        Shape::Inside {
            node,
            branch,
            template,
        } => {
            let block = block_inside(&program, *node, *branch).ok_or(GONE)?;
            let [open, close] = [block.span.start, block.span.end];
            let outer = indentation(source, open);
            let indent = block.statements.first().map_or_else(
                || format!("{outer}    "),
                |first| indentation(source, first.span().start),
            );
            let text = indented(template, &indent);
            let inner = &source[open + 1..close - 1];
            Ok(if inner.trim().is_empty() {
                format!(
                    "{}{{\n{indent}{text}\n{outer}}}{}",
                    &source[..open],
                    &source[close..]
                )
            } else {
                format!(
                    "{}\n{indent}{text}{}",
                    &source[..=open],
                    &source[open + 1..]
                )
            })
        }
    }
}

pub(super) const GONE: &str = "that part of the script is no longer there";

/// `source` with the pin at `slot` reading `this.field`.
fn find_object(source: &str, slot: Span, name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Choose a named object".to_owned());
    }
    let [start, end] = slot;
    let old = source.get(start..end).ok_or(GONE)?;
    let Some(field) = old.strip_prefix("this.") else {
        return Err("that is no longer an object pin".to_owned());
    };
    if field.is_empty() || !field.chars().all(|character| character.is_alphanumeric() || character == '_') {
        return Err("that is no longer an object pin".to_owned());
    }
    let quoted = serde_json::to_string(name).map_err(|error| error.to_string())?;
    Ok(format!("{}World.find({quoted}){}", &source[..start], &source[end..]))
}

fn use_field(source: &str, slot: Span, field: &str) -> Result<String, String> {
    let named = !field.is_empty()
        && field
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_');
    if !named {
        return Err(format!("{field} is not a field's name"));
    }
    let [start, end] = slot;
    if start > end || source.get(start..end).is_none() {
        return Err(GONE.to_owned());
    }
    Ok(format!(
        "{}this.{field}{}",
        &source[..start],
        &source[end..]
    ))
}

fn indented(template: &Template, indent: &str) -> String {
    template.lines().join(&format!("\n{indent}"))
}

/// The whitespace a line starts with, for the line holding `at`.
pub(super) fn indentation(source: &str, at: usize) -> String {
    let start = source[..at].rfind('\n').map_or(0, |line| line + 1);
    source[start..at]
        .chars()
        .take_while(|character| character.is_whitespace())
        .collect()
}

/// The whole lines `span` stands on, from the start of its first to past the
/// end of its last, where it has them to itself; with `comments`, the comment
/// lines directly above it too. Just `span` where it shares a line.
fn lines_of(source: &str, span: decay_syntax::Span, comments: bool) -> [usize; 2] {
    let mut start = source[..span.start].rfind('\n').map_or(0, |line| line + 1);
    let end_of_line = source[span.end..]
        .find('\n')
        .map_or(source.len(), |line| span.end + line + 1);
    let alone = source[start..span.start].trim().is_empty() && {
        let rest = source[span.end..end_of_line].trim();
        rest.is_empty() || rest.starts_with("//")
    };
    if !alone {
        return [span.start, span.end];
    }
    while comments && start > 0 {
        let above = source[..start - 1].rfind('\n').map_or(0, |line| line + 1);
        if source[above..start].trim_start().starts_with("//") {
            start = above;
        } else {
            break;
        }
    }
    [start, end_of_line]
}

/// Where a statement stands: the spans of the statements of its block, and
/// which one it is.
struct Located {
    siblings: Vec<decay_syntax::Span>,
    index: usize,
}

pub(super) fn functions(program: &Program) -> impl Iterator<Item = &decay_syntax::FunctionDecl> {
    program.items.iter().flat_map(|item| match item {
        Item::Script(container) | Item::Component(container) => container
            .members
            .iter()
            .filter_map(|member| match member {
                Member_::Function(function) => Some(function),
                Member_::Field(_) => None,
            })
            .collect::<Vec<_>>(),
        Item::Function(function) => vec![function],
        _ => Vec::new(),
    })
}

fn locate(program: &Program, span: Span) -> Option<Located> {
    functions(program).find_map(|function| locate_in(&function.body, span))
}

fn locate_in(block: &Block, span: Span) -> Option<Located> {
    if let Some(index) = block
        .statements
        .iter()
        .position(|statement| [statement.span().start, statement.span().end] == span)
    {
        return Some(Located {
            siblings: block.statements.iter().map(Stmt::span).collect(),
            index,
        });
    }
    block
        .statements
        .iter()
        .flat_map(nested)
        .find_map(|inner| locate_in(inner, span))
}

/// The blocks a statement runs, in its outputs' order.
fn nested(statement: &Stmt) -> Vec<&Block> {
    match statement {
        Stmt::If {
            then_branch,
            else_branch,
            ..
        } => std::iter::once(then_branch).chain(else_branch).collect(),
        Stmt::While { body, .. } | Stmt::For { body, .. } => vec![body],
        Stmt::Match { arms, .. } => arms.iter().map(|arm| &arm.body).collect(),
        Stmt::Block(block) => vec![block],
        _ => Vec::new(),
    }
}

/// The block `branch` of the statement at `node`, or the body of the
/// function at `node`.
fn block_inside(program: &Program, node: Span, branch: usize) -> Option<&Block> {
    for function in functions(program) {
        if [function.span.start, function.span.end] == node {
            return Some(&function.body);
        }
        if let Some(found) = statement_at(&function.body, node) {
            return nested(found).get(branch).copied();
        }
    }
    None
}

fn statement_at(block: &Block, span: Span) -> Option<&Stmt> {
    block.statements.iter().find_map(|statement| {
        if [statement.span().start, statement.span().end] == span {
            Some(statement)
        } else {
            nested(statement)
                .into_iter()
                .find_map(|inner| statement_at(inner, span))
        }
    })
}
