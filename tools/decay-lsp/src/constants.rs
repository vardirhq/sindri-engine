//! Constants in the editor: listed, completed, and shown with their value.

use decay_semantic::{ConstValue, Environment};
use serde_json::{Value, json};

/// Every constant the file may name, worked out: the project's shared ones,
/// and its own, which win where both declare one.
pub(crate) fn constants(environment: &Environment, source: &str) -> Vec<(String, ConstValue)> {
    let own = decay_semantic::analyze_with_environment(source, environment).constants;
    let mut found: Vec<(String, ConstValue)> = own.into_iter().collect();
    for (name, value) in environment.constants() {
        if !found.iter().any(|(known, _)| known == name) {
            found.push((name.to_owned(), value.clone()));
        }
    }
    found.sort_by(|(a, _), (b, _)| a.cmp(b));
    found
}

/// How a constant reads on hover: its declaration, with the value it was
/// worked out to, which is what a script using it gets.
pub(crate) fn constant_hover(name: &str, value: &ConstValue) -> Value {
    json!({"contents":{"kind":"markdown","value":format!(
        "```decay\nconst {name}: {} = {}\n```",
        value.ty().display_name(),
        value.display()
    )}})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_sees_its_own_constants_and_the_projects() {
        let mut environment = Environment::new();
        environment.add_constant("ARENA", ConstValue::Number(12.0));
        environment.add_constant("GAP", ConstValue::Number(1.0));
        let found = constants(
            &environment,
            "shared const GAP: f32 = 2.0; const EDGE: f32 = ARENA - GAP;",
        );
        assert_eq!(
            found,
            vec![
                ("ARENA".to_owned(), ConstValue::Number(12.0)),
                ("EDGE".to_owned(), ConstValue::Number(10.0)),
                ("GAP".to_owned(), ConstValue::Number(2.0)),
            ]
        );
    }

    #[test]
    fn a_constant_hovers_as_its_declaration_with_its_value() {
        let hover = constant_hover("HALF", &ConstValue::Number(6.0));
        assert_eq!(
            hover.pointer("/contents/value").and_then(Value::as_str),
            Some("```decay\nconst HALF: f32 = 6\n```")
        );
        let hover = constant_hover("TITLE", &ConstValue::Text("Orbital".to_owned()));
        assert!(
            hover
                .to_string()
                .contains(r#"const TITLE: String = \"Orbital\""#),
            "{hover}"
        );
    }
}
