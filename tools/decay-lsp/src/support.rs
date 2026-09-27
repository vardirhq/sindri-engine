use std::path::PathBuf;

use decay_semantic::{Environment, ExternalSymbol, FunctionType, Type};
use decay_syntax::{Item, Member, Span, parse};
use serde_json::{Value, json};

pub(crate) const KEYWORDS: &[&str] = &[
    "script",
    "component",
    "fn",
    "let",
    "var",
    "if",
    "else",
    "while",
    "for",
    "in",
    "break",
    "continue",
    "return",
    "true",
    "false",
    "null",
];

pub(crate) fn type_members<'a>(
    environment: &'a Environment,
    ty: &Type,
) -> Option<&'a decay_semantic::HostType> {
    match ty {
        Type::Named(name) => environment.get_type(name),
        _ => None,
    }
}

pub(crate) fn container_members(source: &str) -> Vec<(String, ExternalSymbol)> {
    let parsed = parse(source);
    let Some(container) = parsed.program.items.iter().find_map(|item| match item {
        Item::Script(container) | Item::Component(container) => Some(container),
        Item::Event(_) | Item::State(_) => None,
    }) else {
        return Vec::new();
    };
    container
        .members
        .iter()
        // A handler is run by the host when its event is emitted; it is not
        // a name the script can reach.
        .filter(
            |member| !matches!(member, Member::Function(function) if function.handles.is_some()),
        )
        .map(|member| match member {
            Member::Field(field) => (
                field.name.clone(),
                ExternalSymbol::Value(field.ty.as_ref().map_or(Type::Unknown, Type::from_ref)),
            ),
            Member::Function(function) => (
                function.name.clone(),
                ExternalSymbol::Function(FunctionType {
                    params: function
                        .params
                        .iter()
                        .map(|param| param.ty.as_ref().map_or(Type::Unknown, Type::from_ref))
                        .collect(),
                    return_type: function
                        .return_type
                        .as_ref()
                        .map_or(Type::Unit, Type::from_ref),
                }),
            ),
        })
        .collect()
}

pub(crate) fn completion_chain(before: &str) -> (Option<Vec<String>>, String) {
    let tail = before
        .chars()
        .rev()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || *character == '_' || *character == '.'
        })
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    if !tail.contains('.') {
        return (None, tail);
    }
    let mut parts = tail.split('.').map(str::to_owned).collect::<Vec<_>>();
    let prefix = parts.pop().unwrap_or_default();
    (Some(parts), prefix)
}

pub(crate) fn string_argument(before: &str, call: &str) -> Option<String> {
    let marker = format!("{call}(\"");
    let start = before.rfind(&marker)? + marker.len();
    let tail = &before[start..];
    (!tail.contains('"')).then(|| tail.to_owned())
}

/// Every event the file may emit: the project's, and its own.
pub(crate) fn events(environment: &Environment, source: &str) -> Vec<(String, Vec<Type>)> {
    let mut events: Vec<(String, Vec<Type>)> = environment
        .events()
        .map(|(name, params)| (name.to_owned(), params.to_vec()))
        .collect();
    for item in parse(source).program.items {
        if let Item::Event(event) = item
            && !events.iter().any(|(name, _)| *name == event.name)
        {
            let params = event
                .params
                .iter()
                .map(|param| param.ty.as_ref().map_or(Type::Unknown, Type::from_ref))
                .collect();
            events.push((event.name, params));
        }
    }
    events
}

/// A state's fields, the project's and the file's own: what `Game.` offers
/// beyond the namespace itself.
pub(crate) fn state_fields(
    environment: &Environment,
    source: &str,
    state: &str,
) -> Vec<(String, ExternalSymbol)> {
    let mut fields: Vec<(String, ExternalSymbol)> = environment
        .state_fields(state)
        .map(|(name, field)| (name.to_owned(), ExternalSymbol::Value(field.ty.clone())))
        .collect();
    for item in parse(source).program.items {
        let Item::State(declared) = item else {
            continue;
        };
        if declared.name != state {
            continue;
        }
        for field in declared.fields {
            if !fields.iter().any(|(name, _)| *name == field.name) {
                let ty = field.ty.as_ref().map_or(Type::Unknown, Type::from_ref);
                fields.push((field.name, ExternalSymbol::Value(ty)));
            }
        }
    }
    fields
}

/// Every state name the file may reach.
pub(crate) fn states(environment: &Environment, source: &str) -> Vec<String> {
    let mut names: Vec<String> = environment.states().map(str::to_owned).collect();
    for item in parse(source).program.items {
        if let Item::State(declared) = item
            && !names.contains(&declared.name)
        {
            names.push(declared.name);
        }
    }
    names
}

/// `emit`, the one member an event has.
pub(crate) fn emit_symbol(params: &[Type]) -> ExternalSymbol {
    ExternalSymbol::Function(FunctionType {
        params: params.to_vec(),
        return_type: Type::Unit,
    })
}

/// How an event reads on hover: its declaration.
pub(crate) fn event_hover(name: &str, params: &[Type]) -> Value {
    let carried = params
        .iter()
        .map(Type::display_name)
        .collect::<Vec<_>>()
        .join(", ");
    json!({"contents":{"kind":"markdown","value":format!(
        "```decay\nevent {name}({carried})\n```\nSend one with `{name}.emit(...)`; handle it with `on {name}(...)`."
    )}})
}

pub(crate) fn completion_item(label: &str, kind: u8, detail: Option<&str>) -> Value {
    json!({"label":label,"kind":kind,"detail":detail})
}

pub(crate) fn symbol_completion(name: &str, symbol: &ExternalSymbol) -> Value {
    match symbol {
        ExternalSymbol::Value(ty) => {
            json!({"label":name,"kind":6,"detail":ty.display_name()})
        }
        ExternalSymbol::Function(function) => json!({
            "label":name,
            "kind":3,
            "detail":signature(name, function),
            "insertText":function_snippet(name, function),
            "insertTextFormat":2
        }),
    }
}

pub(crate) fn hover_symbol(name: &str, symbol: &ExternalSymbol) -> Value {
    let value = match symbol {
        ExternalSymbol::Value(ty) => {
            format!("```decay\n{name}: {}\n```", ty.display_name())
        }
        ExternalSymbol::Function(function) => {
            format!("```decay\n{}\n```", signature(name, function))
        }
    };
    json!({"contents":{"kind":"markdown","value":value}})
}

fn function_snippet(name: &str, function: &FunctionType) -> String {
    if function.params.is_empty() {
        return format!("{name}()");
    }
    let params = (1..=function.params.len())
        .map(|placeholder| format!("${placeholder}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{name}({params})")
}

fn signature(name: &str, function: &FunctionType) -> String {
    let params = function
        .params
        .iter()
        .map(Type::display_name)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "fn {name}({params}) -> {}",
        function.return_type.display_name()
    )
}

pub(crate) fn initialization_root(params: &Value) -> Option<PathBuf> {
    params
        .get("rootPath")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| {
            params
                .get("rootUri")
                .and_then(Value::as_str)
                .and_then(file_uri_to_path)
        })
}

fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let value = uri.strip_prefix("file://")?;
    Some(PathBuf::from(percent_decode(value)))
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let Ok(hex) = u8::from_str_radix(&value[index + 1..index + 3], 16)
        {
            output.push(hex);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}

pub(crate) fn span_range(source: &str, span: Span) -> Value {
    let (start_line, start_character) = position_at(source, span.start);
    let end = span.end.max(span.start.saturating_add(1)).min(source.len());
    let (end_line, end_character) = position_at(source, end);
    json!({
        "start":{"line":start_line,"character":start_character},
        "end":{"line":end_line,"character":end_character}
    })
}

fn position_at(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let character = source[line_start..offset].encode_utf16().count();
    (line, character)
}

pub(crate) fn offset_at(source: &str, line: usize, character: usize) -> usize {
    let mut offset = 0;
    for (index, part) in source.split_inclusive('\n').enumerate() {
        if index == line {
            let without_newline = part.strip_suffix('\n').unwrap_or(part);
            let mut utf16 = 0;
            for (byte, ch) in without_newline.char_indices() {
                if utf16 >= character {
                    return offset + byte;
                }
                utf16 += ch.len_utf16();
            }
            return offset + without_newline.len();
        }
        offset += part.len();
    }
    source.len()
}

pub(crate) fn word_at(source: &str, offset: usize) -> Option<&str> {
    let offset = offset.min(source.len());
    let start = source[..offset]
        .rfind(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .map_or(0, |index| index + 1);
    let end = source[offset..]
        .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .map_or(source.len(), |index| offset + index);
    (start < end).then(|| &source[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_loop_keywords_are_exposed() {
        assert!(KEYWORDS.contains(&"for"));
        assert!(KEYWORDS.contains(&"in"));
    }

    #[test]
    fn function_snippets_follow_arity() {
        let zero = FunctionType {
            params: vec![],
            return_type: Type::Unit,
        };
        let three = FunctionType {
            params: vec![Type::F32, Type::Bool, Type::String],
            return_type: Type::Unit,
        };
        assert_eq!(function_snippet("tick", &zero), "tick()");
        assert_eq!(function_snippet("mix", &three), "mix($1, $2, $3)");
    }

    #[test]
    fn utf16_positions_round_trip() {
        let source = "script Cafe {\n  fn tick() { let label = \"🚀\"; }\n}";
        let rocket = source.find('🚀').expect("rocket exists");
        let (line, character) = position_at(source, rocket);
        assert_eq!(offset_at(source, line, character), rocket);
    }
}
