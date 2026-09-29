//! What `print` writes for a value.

use decay_runtime::Value;

/// A value as `print` writes it: text as itself, everything else as a short
/// description a reader can act on.
pub(super) fn printed(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => decay_runtime::spell_number(*number),
        Some(Value::Bool(value)) => format!("{value}"),
        Some(Value::Null) | None => "null".to_owned(),
        Some(Value::Unit) => "unit".to_owned(),
        // Named by what it is rather than by the number inside:
        // the packing is the host's business, and printing it
        // would invite a script to depend on it.
        Some(Value::Reference(_)) => "entity".to_owned(),
        // How many, not what: printing a collection of two
        // thousand entities into the console is not what anyone
        // reaching for `print` wanted, and the elements are
        // reachable one at a time anyway.
        Some(Value::Array(values)) => format!("{} entries", values.len()),
        // How many, for the same reason: its keys are reachable.
        Some(Value::Map(entries)) => format!("{} keys", entries.len()),
        // Its variant, as the script wrote it: `Phase.Play`.
        Some(Value::Variant(name)) => name.to_string(),
        Some(value @ Value::Struct { .. }) => decay_runtime::show_struct(value),
        Some(Value::Timer { left, duration }) => {
            format!("timer {left}s of {duration}s")
        }
        Some(vector @ (Value::Vec2(_) | Value::Vec3(_))) => format!(
            "({})",
            vector
                .components()
                .unwrap_or_default()
                .iter()
                .map(|c| format!("{c}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}
