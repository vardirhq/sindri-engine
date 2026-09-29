//! The Decay language itself, as a description something else can read.
//!
//! Where `decay-api.json` describes what the engine offers a script, this
//! describes the language the script is written in: its words, its operators,
//! its built-in types and what each can be asked, the functions the engine
//! calls, and its limits. Every list is read from the table the compiler
//! itself uses — the lexer's keywords, the parser's operators, the analyzer's
//! member signatures, the runtime's limits — so nothing here can say something
//! the compiler does not.
//!
//! What it does not carry yet is diagnostics by stable ID: the compiler still
//! reports each error by its phase, not by a name a page can be kept for.

use decay_semantic::members::{
    list_op_signature, map_op_signature, number_op_signature, string_op_signature,
    timer_property_type, vector_op_signature,
};
use decay_semantic::{
    BUILT_IN_TYPES, COMPONENTS, FunctionType, LENGTH, OLD_LENGTH, TIMER, Type, VEC2, VEC3,
};
use decay_syntax::vocabulary::{
    ASSIGNMENT_OPERATORS, ATTRIBUTES, BINARY_OPERATORS, CONTEXTUAL_KEYWORDS, ITEM_WORDS, KEYWORDS,
    UNARY_OPERATORS, VERSION,
};
use decay_syntax::{ListOp, MapOp, NumberOp, StringOp, TimerProperty, VectorOp};
use serde_json::{Value, json};

use crate::decay::type_name;

/// The shape of this file, which a reader checks before reading the rest.
pub(crate) const LANGUAGE_SCHEMA: &str = "decay-language/1";

/// The placeholder a list's element type is written as.
const ELEMENT: &str = "T";

pub(crate) fn describe() -> Value {
    json!({
        "schema": LANGUAGE_SCHEMA,
        "about": "The Decay language: its words, operators, built-in types and what each can be asked, the functions the Sindri engine calls, and its limits. Read from the tables the lexer, parser, analyzer and runtime use. decay/LANGUAGE.md explains what each means; docs/generated/decay-api.json lists what the engine offers a script.",
        "generated_by": crate::REGENERATE_COMMAND,
        "language_version": VERSION,
        "keywords": KEYWORDS.iter().map(|(word, _)| word).collect::<Vec<_>>(),
        "contextual_keywords": CONTEXTUAL_KEYWORDS
            .iter()
            .map(|word| json!({ "name": word.word, "special_at": word.position }))
            .collect::<Vec<_>>(),
        "items": ITEM_WORDS,
        "attributes": ATTRIBUTES,
        "operators": operators(),
        "types": types(),
        "constructors": constructors(),
        "members": members(),
        "lifecycle": sindri_decay::LIFECYCLE
            .iter()
            .map(|function| json!({
                "name": function.name,
                "params": function
                    .params
                    .iter()
                    .map(|(name, ty)| json!({ "name": name, "type": ty }))
                    .collect::<Vec<_>>(),
                "returns": "unit",
                "called": function.when,
            }))
            .collect::<Vec<_>>(),
        "limits": {
            "call_depth": decay_runtime::DEFAULT_CALL_DEPTH_LIMIT,
            "operations_per_call": decay_runtime::DEFAULT_OPERATION_BUDGET,
            "list_elements": decay_runtime::LIST_LIMIT,
            "text_bytes": decay_runtime::TEXT_LIMIT,
        },
        "diagnostic_phases": ["decay-syntax", "decay-semantic"],
        "diagnostics": diagnostics(),
    })
}

/// Every compiler diagnostic by its stable code: the phase and the name
/// joined, exactly as `decay-lsp` reports it, so a page can be keyed on it.
fn diagnostics() -> Vec<Value> {
    let syntax = decay_syntax::codes::SyntaxCode::ALL
        .iter()
        .map(|(_, id, summary)| ("decay-syntax", *id, *summary));
    let semantic = decay_semantic::codes::Code::ALL
        .iter()
        .map(|(_, id, summary)| ("decay-semantic", *id, *summary));
    syntax
        .chain(semantic)
        .map(|(phase, id, summary)| {
            json!({
                "code": format!("{phase}/{id}"),
                "phase": phase,
                "id": id,
                "summary": summary,
            })
        })
        .collect()
}

fn operators() -> Value {
    json!({
        "binary": BINARY_OPERATORS
            .iter()
            .map(|(_, precedence, op)| json!({
                "symbol": op.symbol(),
                "name": format!("{op:?}"),
                "precedence": precedence,
                "associativity": "left",
            }))
            .collect::<Vec<_>>(),
        "unary": UNARY_OPERATORS
            .iter()
            .map(|(_, op)| json!({
                "symbol": op.symbol(),
                "name": format!("{op:?}"),
                "associativity": "right",
                "binds": "tighter than any binary operator",
            }))
            .collect::<Vec<_>>(),
        "assignment": ASSIGNMENT_OPERATORS
            .iter()
            .map(|(_, op)| json!({
                "symbol": op.symbol(),
                "name": format!("{op:?}"),
                "associativity": "right",
                "binds": "looser than any binary operator",
            }))
            .collect::<Vec<_>>(),
    })
}

fn types() -> Value {
    let mut types: Vec<Value> = Vec::new();
    for (spelling, ty) in BUILT_IN_TYPES {
        let canonical = type_name(&ty);
        if let Some(existing) = types
            .iter_mut()
            .find(|entry| entry["name"] == canonical.as_str())
        {
            if let Some(spellings) = existing["spellings"].as_array_mut() {
                spellings.push(json!(spelling));
            }
            continue;
        }
        types.push(json!({ "name": canonical, "spellings": [spelling], "takes": [] }));
    }
    types.push(json!({
        "name": type_name(&Type::array_of(Type::Named(ELEMENT.to_owned()))),
        "spellings": [decay_semantic::LIST, "Array"],
        "takes": [ELEMENT],
    }));
    Value::Array(types)
}

fn constructors() -> Value {
    let vector = |name: &str, ty: Type| {
        let dimensions = ty.dimensions().unwrap_or(0);
        json!({
            "name": name,
            "params": COMPONENTS[..dimensions]
                .iter()
                .map(|component| json!({ "name": component, "type": "f32" }))
                .collect::<Vec<_>>(),
            "returns": type_name(&ty),
        })
    };
    json!([
        vector(VEC2, Type::Vec2),
        vector(VEC3, Type::Vec3),
        {
            "name": TIMER,
            "params": [{ "name": "seconds", "type": "f32" }],
            "returns": type_name(&Type::Timer),
        },
    ])
}

fn member(name: &str, signature: &FunctionType, call: bool) -> Value {
    if call {
        json!({
            "name": name,
            "kind": "method",
            "params": signature.params.iter().map(type_name).collect::<Vec<_>>(),
            "returns": type_name(&signature.return_type),
        })
    } else {
        json!({ "name": name, "kind": "property", "type": type_name(&signature.return_type) })
    }
}

fn property(name: &str, ty: &Type) -> Value {
    json!({ "name": name, "kind": "property", "type": type_name(ty) })
}

fn members() -> Value {
    let vector = |ty: Type| {
        let dimensions = ty.dimensions().unwrap_or(0);
        let mut members: Vec<Value> = COMPONENTS[..dimensions]
            .iter()
            .map(|component| property(component, &Type::F32))
            .collect();
        members.extend(
            VectorOp::ALL
                .iter()
                .map(|(op, name, arity)| member(name, &vector_op_signature(*op, &ty), *arity > 0)),
        );
        json!({ "type": type_name(&ty), "members": members })
    };
    let element = Type::Named(ELEMENT.to_owned());
    let mut list = vec![
        property(LENGTH, &Type::F32),
        json!({ "name": OLD_LENGTH, "kind": "property", "type": "f32", "alias_of": LENGTH }),
    ];
    // Every list operation is a call, `pop()` and `clear()` included.
    list.extend(
        ListOp::ALL
            .iter()
            .map(|(op, name, _)| member(name, &list_op_signature(*op, &element), true)),
    );
    // A map's key and value, named as the list's element is.
    let (key, value) = (Type::Named("K".to_owned()), Type::Named("V".to_owned()));
    let mut map = vec![property(LENGTH, &Type::F32)];
    // Every map operation is a call, `keys()` included.
    map.extend(
        MapOp::ALL
            .iter()
            .map(|(op, name, _)| member(name, &map_op_signature(*op, &key, &value), true)),
    );
    json!([
        vector(Type::Vec2),
        vector(Type::Vec3),
        {
            "type": type_name(&Type::Timer),
            "members": TimerProperty::ALL
                .iter()
                .map(|(timer, name)| property(name, &timer_property_type(*timer)))
                .collect::<Vec<_>>(),
        },
        {
            "type": type_name(&Type::String),
            "members": StringOp::ALL
                .iter()
                .map(|(op, name, arity)| member(name, &string_op_signature(*op), *arity > 0))
                .collect::<Vec<_>>(),
        },
        {
            "type": type_name(&Type::F32),
            "members": NumberOp::ALL
                .iter()
                .map(|(op, name, _)| member(name, &number_op_signature(*op), true))
                .collect::<Vec<_>>(),
        },
        { "type": type_name(&Type::array_of(element)), "members": list },
        { "type": type_name(&Type::Map(Box::new(key), Box::new(value))), "members": map },
    ])
}

#[cfg(test)]
mod tests {
    use super::{LANGUAGE_SCHEMA, describe};

    /// A reader checks the schema before anything else, so it is always
    /// there, and every table the file promises is filled.
    #[test]
    fn the_language_file_names_its_schema_and_fills_every_table() {
        let language = describe();
        assert_eq!(language["schema"], LANGUAGE_SCHEMA);
        for table in [
            "keywords",
            "contextual_keywords",
            "items",
            "attributes",
            "types",
            "constructors",
            "members",
            "lifecycle",
            "diagnostics",
        ] {
            assert!(
                language[table]
                    .as_array()
                    .is_some_and(|items| !items.is_empty()),
                "{table} is empty"
            );
        }
        for kind in ["binary", "unary", "assignment"] {
            assert!(
                language["operators"][kind]
                    .as_array()
                    .is_some_and(|items| !items.is_empty()),
                "{kind} operators are empty"
            );
        }
    }
}
