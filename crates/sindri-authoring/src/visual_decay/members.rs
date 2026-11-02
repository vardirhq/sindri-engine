//! What a script is made of, grown or changed from the phone: a new
//! variable, function or event, and renaming or removing one.

use decay_syntax::{Item, Member as Member_, Program, parse};

use super::structure::{GONE, functions, indentation};

/// What a script can gain from the tab bar's "+".
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Member {
    /// `var name: ty = <its type's empty value>;`, for `number`, `bool` or
    /// `String`.
    Variable { name: String, ty: String },
    /// `fn name(input: type, …) { }`, each input a `number`, `bool` or `String`.
    Function {
        name: String,
        inputs: Vec<(String, String)>,
    },
    /// One of the engine's events, by its lifecycle name: `fn update(dt: number) { }`.
    Event { name: String },
}

impl Member {
    fn name(&self) -> &str {
        match self {
            Self::Variable { name, .. } | Self::Function { name, .. } | Self::Event { name } => {
                name
            }
        }
    }

    /// Its lines, before indentation.
    fn lines(&self) -> Result<Vec<String>, String> {
        Ok(match self {
            Self::Variable { name, ty } => {
                let value = placeholder(ty)
                    .ok_or_else(|| format!("a new variable cannot be a {ty} yet"))?;
                vec![format!("var {name}: {ty} = {value};")]
            }
            Self::Function { name, inputs } => {
                for (input, ty) in inputs {
                    if !is_name(input) {
                        return Err(format!(
                            "\"{input}\" cannot be a name: use letters, digits and _"
                        ));
                    }
                    if placeholder(ty).is_none() {
                        return Err(format!("an input cannot be a {ty} yet"));
                    }
                }
                let inputs = inputs
                    .iter()
                    .map(|(input, ty)| format!("{input}: {ty}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                vec![format!("fn {name}({inputs}) {{"), "}".to_owned()]
            }
            Self::Event { name } => {
                let event = sindri_decay::LIFECYCLE
                    .iter()
                    .find(|event| event.name == name)
                    .ok_or_else(|| format!("{name} is not one of the engine's events"))?;
                let params = event
                    .params
                    .iter()
                    .map(|(param, ty)| format!("{param}: {ty}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                vec![format!("fn {name}({params}) {{"), "}".to_owned()]
            }
        })
    }
}

pub(super) fn is_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && name
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
}

/// What a new value of type `ty` starts as, for the types that have one.
fn placeholder(ty: &str) -> Option<&'static str> {
    match ty {
        "number" => Some("0"),
        "bool" => Some("false"),
        "String" => Some("\"\""),
        _ => None,
    }
}

/// A placeholder for each input of the script's function `name`.
pub(super) fn placeholders(source: &str, name: &str) -> Option<Vec<String>> {
    let program = parse(source).program;
    let function = functions(&program).find(|function| function.name == name)?;
    function
        .params
        .iter()
        .map(|param| {
            let ty = param
                .ty
                .as_ref()
                .map(|ty| ty.name.as_str())
                .unwrap_or_default();
            placeholder(ty).map(str::to_owned)
        })
        .collect()
}

pub(super) fn first_container(program: &Program) -> Option<&decay_syntax::ContainerDecl> {
    program.items.iter().find_map(|item| match item {
        Item::Script(container) | Item::Component(container) => Some(container),
        _ => None,
    })
}

/// Where the script's variable or function `name` is.
pub(super) fn member_span(program: &Program, name: &str) -> Result<decay_syntax::Span, String> {
    first_container(program)
        .into_iter()
        .flat_map(|container| &container.members)
        .find_map(|member| match member {
            Member_::Field(field) if field.name == name => Some(field.span),
            Member_::Function(function) if function.name == name => Some(function.span),
            _ => None,
        })
        .ok_or_else(|| format!("the script has no {name}"))
}

/// `source` with every name `from` written `to`. A variable shown in the
/// inspector keeps its value in scenes by name, so it is not renamed here.
pub(super) fn rename(
    source: &str,
    program: &Program,
    from: &str,
    to: &str,
) -> Result<String, String> {
    if !is_name(to) {
        return Err(format!(
            "\"{to}\" cannot be a name: use letters, digits and _"
        ));
    }
    member_span(program, from)?;
    let exported = first_container(program).is_some_and(|container| {
        container.members.iter().any(|member| {
            matches!(member, Member_::Field(field) if field.name == from && !field.attributes.is_empty())
        })
    });
    if exported {
        return Err(format!(
            "{from} is shown in the inspector, and scenes keep its value by that name"
        ));
    }
    let mut renamed = source.to_owned();
    let mut names: Vec<_> = decay_syntax::lex(source)
        .tokens
        .into_iter()
        .filter(|token| matches!(&token.kind, decay_syntax::TokenKind::Identifier(name) if name == from))
        .collect();
    names.reverse();
    for token in names {
        renamed.replace_range(token.span.start..token.span.end, to);
    }
    Ok(renamed)
}

/// `source` with `member` written into its first script or component.
pub(super) fn add(source: &str, program: &Program, member: &Member) -> Result<String, String> {
    let name = member.name();
    if !is_name(name) {
        return Err(format!(
            "\"{name}\" cannot be a name: use letters, digits and _"
        ));
    }
    let container = first_container(program).ok_or("the file has no script to add to")?;
    let outer = indentation(source, container.span.start);
    let indent = container.members.first().map_or_else(
        || format!("{outer}    "),
        |first| {
            let start = match first {
                Member_::Field(field) => field.span.start,
                Member_::Function(function) => function.span.start,
            };
            indentation(source, start)
        },
    );
    let text = member.lines()?.join(&format!("\n{indent}"));
    if matches!(member, Member::Variable { .. }) {
        // First among the fields: just inside the script's opening brace.
        let open = source[container.span.start..container.span.end]
            .find('{')
            .map(|at| container.span.start + at)
            .ok_or(GONE)?;
        let line_end = source[open..]
            .find('\n')
            .map_or(source.len(), |at| open + at + 1);
        return Ok(format!(
            "{}{indent}{text}\n{}",
            &source[..line_end],
            &source[line_end..]
        ));
    }
    // Last: on its own lines before the script's closing brace.
    let close = container.span.end - 1;
    let line_start = source[..close].rfind('\n').map_or(0, |at| at + 1);
    let blank = if container.members.is_empty() {
        ""
    } else {
        "\n"
    };
    Ok(format!(
        "{}{blank}{indent}{text}\n{}",
        &source[..line_start],
        &source[line_start..]
    ))
}
