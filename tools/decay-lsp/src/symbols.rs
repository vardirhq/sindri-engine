//! A file's outline: what `textDocument/documentSymbol` answers with.

use decay_syntax::{Item, Member, Span, parse};
use serde_json::{Value, json};

use crate::support::span_range;

/// One symbol, with its children when it has any. `kind` is the LSP
/// `SymbolKind` number.
fn symbol(source: &str, name: &str, kind: u8, span: Span, children: Option<Vec<Value>>) -> Value {
    let mut symbol = json!({
        "name": name,
        "kind": kind,
        "range": span_range(source, span),
        "selectionRange": span_range(source, span)
    });
    if let Some(children) = children {
        symbol["children"] = Value::Array(children);
    }
    symbol
}

/// Every item the file declares, with a container's members, a state's
/// fields, an enum's variants and a struct's fields beneath it.
pub(crate) fn outline(source: &str) -> Value {
    let parsed = parse(source);
    let symbols = parsed
        .program
        .items
        .into_iter()
        .map(|item| match item {
            Item::Script(container) | Item::Component(container) => {
                let children = container
                    .members
                    .into_iter()
                    .map(|member| match member {
                        Member::Field(field) => symbol(source, &field.name, 8, field.span, None),
                        Member::Function(function) => {
                            symbol(source, &function.name, 12, function.span, None)
                        }
                    })
                    .collect();
                symbol(source, &container.name, 5, container.span, Some(children))
            }
            Item::Enum(declared) => {
                let children = declared
                    .variants
                    .iter()
                    .map(|(variant, span)| symbol(source, variant, 22, *span, None))
                    .collect();
                symbol(source, &declared.name, 10, declared.span, Some(children))
            }
            Item::Struct(declared) => {
                let children = declared
                    .fields
                    .iter()
                    .map(|field| symbol(source, &field.name, 8, field.span, None))
                    .collect();
                symbol(source, &declared.name, 23, declared.span, Some(children))
            }
            Item::Function(function) => symbol(source, &function.name, 12, function.span, None),
            Item::State(state) => {
                let children = state
                    .fields
                    .iter()
                    .map(|field| symbol(source, &field.name, 8, field.span, None))
                    .collect();
                symbol(source, &state.name, 23, state.span, Some(children))
            }
            Item::Event(event) => symbol(source, &event.name, 24, event.span, None),
            Item::Const(constant) => symbol(source, &constant.name, 14, constant.span, None),
        })
        .collect();
    Value::Array(symbols)
}
