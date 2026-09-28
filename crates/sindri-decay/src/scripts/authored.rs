//! A scene's authored value for an `@export` field, read by the field's type.
//!
//! A number, a flag, text and a vector read the same whatever field they are
//! for. A list and a struct do not: `[1, 2]` is a `List<f32>` or a `Vec2`, and
//! `{"name": "Arc"}` is a struct only a type can say the fields of. So these
//! are read against the type the script declared — or, for a field written
//! without one, the shape its default already has.

use std::rc::Rc;

use decay_ir::{IrProgram, StructShape};
use decay_runtime::Value;
use decay_semantic::Type;

use super::run::to_value;

/// How deep a struct may hold structs before a blank stops building one: a
/// struct that holds itself would otherwise never finish.
const DEPTH: usize = 8;

/// The type a field holds: as written, or from the default it starts as.
pub(crate) fn field_type(written: Option<&Type>, default: Option<&Value>) -> Option<Type> {
    if let Some(written) = written {
        return Some(written.clone());
    }
    default.and_then(type_of)
}

fn type_of(value: &Value) -> Option<Type> {
    Some(match value {
        Value::Struct { shape, .. } => Type::Named(shape.name.clone()),
        Value::Array(values) => {
            Type::array_of(values.first().and_then(type_of).unwrap_or(Type::Unknown))
        }
        Value::Number(_) => Type::F32,
        Value::Bool(_) => Type::Bool,
        Value::String(_) => Type::String,
        Value::Vec2(_) => Type::Vec2,
        Value::Vec3(_) => Type::Vec3,
        Value::Variant(name) => Type::Named(name.split('.').next()?.to_owned()),
        _ => return None,
    })
}

/// Whether authoring this type needs its type at all: a list or a struct.
pub(crate) fn is_compound(program: &IrProgram, ty: &Type) -> bool {
    match ty {
        Type::Array(_) => true,
        Type::Named(name) => program.struct_fields(name).is_some(),
        _ => false,
    }
}

/// A scene's value for a field of type `ty`, where `base` is what the field
/// holds already: a struct field the scene leaves out keeps its part of it.
pub(crate) fn authored(
    program: &IrProgram,
    ty: &Type,
    value: &serde_json::Value,
    base: Option<&Value>,
) -> Result<Value, String> {
    match ty {
        Type::Array(element) => {
            let serde_json::Value::Array(items) = value else {
                return Err(format!(
                    "{value} is not a list; a `{}` is written `[...]`",
                    ty.display_name()
                ));
            };
            let items = items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    authored(program, element, item, None)
                        .map_err(|reason| format!("item {index}: {reason}"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::array(items))
        }
        Type::Named(name) if program.struct_fields(name).is_some() => {
            authored_struct(program, name, value, base)
        }
        Type::Named(name) if program.variants(name).is_some() => {
            super::run::authored_variant(program, name, value)
        }
        _ => to_value(value)
            .ok_or_else(|| format!("{value} is not a number, string, boolean or vector")),
    }
}

fn authored_struct(
    program: &IrProgram,
    name: &str,
    value: &serde_json::Value,
    base: Option<&Value>,
) -> Result<Value, String> {
    let declared = program.struct_fields(name).unwrap_or_default();
    let serde_json::Value::Object(given) = value else {
        return Err(format!(
            "{value} is not a `{name}`; one is written {{\"field\": value, ...}}"
        ));
    };
    if let Some(unknown) = given
        .keys()
        .find(|key| !declared.iter().any(|(field, _)| field == *key))
    {
        let fields = declared
            .iter()
            .map(|(field, _)| field.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "`{name}` has no field `{unknown}`; it has {fields}"
        ));
    }
    // A field the scene leaves out keeps what the script gave it, or starts
    // blank where nothing did, as a new one in the inspector does.
    let kept = match base {
        Some(Value::Struct { fields, .. }) => Some(fields),
        _ => None,
    };
    let fields = declared
        .iter()
        .enumerate()
        .map(|(position, (field, ty))| {
            let held = kept.and_then(|fields| fields.get(position));
            match given.get(field) {
                Some(value) => authored(program, ty, value, held)
                    .map_err(|reason| format!("`{field}`: {reason}")),
                None => Ok(held.cloned().unwrap_or_else(|| blank(program, ty))),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Value::Struct {
        shape: Rc::new(StructShape {
            name: name.to_owned(),
            fields: declared.iter().map(|(field, _)| field.clone()).collect(),
        }),
        fields: Rc::new(fields),
    })
}

/// What a fresh value of this type is: zero, false, empty, the first variant,
/// a struct of blanks.
pub(crate) fn blank(program: &IrProgram, ty: &Type) -> Value {
    blank_at(program, ty, 0)
}

fn blank_at(program: &IrProgram, ty: &Type, depth: usize) -> Value {
    match ty {
        Type::F32 => Value::Number(0.0),
        Type::Bool => Value::Bool(false),
        Type::String => Value::String(String::new()),
        Type::Vec2 => Value::Vec2([0.0; 2]),
        Type::Vec3 => Value::Vec3([0.0; 3]),
        Type::Array(_) => Value::array(Vec::new()),
        Type::Named(name) if depth < DEPTH => {
            if let Some(first) = program.variants(name).and_then(<[String]>::first) {
                return Value::Variant(format!("{name}.{first}").into());
            }
            let Some(declared) = program.struct_fields(name) else {
                return Value::Null;
            };
            Value::Struct {
                shape: Rc::new(StructShape {
                    name: name.clone(),
                    fields: declared.iter().map(|(field, _)| field.clone()).collect(),
                }),
                fields: Rc::new(
                    declared
                        .iter()
                        .map(|(_, ty)| blank_at(program, ty, depth + 1))
                        .collect(),
                ),
            }
        }
        _ => Value::Null,
    }
}
