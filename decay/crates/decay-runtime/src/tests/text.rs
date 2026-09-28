//! Text at runtime: joined with `+`, spelled the way a script would write
//! it, and asked questions by character.

use crate::{EmptyHost, Runtime, RuntimeError, TEXT_LIMIT, Value};

const SOURCE: &str = r#"enum Phase { Lobby, Play }
script Label {
    fn score(blue: f32, red: f32) -> String { return "Blue " + blue + " - " + red + " Red"; }
    fn many(n: f32) -> String { return "" + n; }
    fn parts() -> String {
        var s = "phase: ";
        s += Phase.Play;
        s += ", ready " + true;
        s += " at " + Vec2(1.5, -2.0);
        return s;
    }
    fn length(s: String) -> f32 { return s.length; }
    fn shout(s: String) -> String { return s.trimmed.uppercase; }
    fn quiet(s: String) -> String { return s.lowercase; }
    fn has(s: String, part: String) -> bool { return s.contains(part); }
    fn edges(s: String) -> bool { return s.starts_with("d") && s.ends_with("9"); }
    fn find(s: String, part: String) -> f32 { return s.find(part); }
    fn slice(s: String, start: f32, end: f32) -> String { return s.slice(start, end); }
    fn replace(s: String) -> String { return s.replace("o", "0"); }
    fn grow() -> String {
        var s = "0123456789";
        while true { s = s + s; }
        return s;
    }
}"#;

fn call(function: &str, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let program = decay_ir::lower(SOURCE).program.expect("valid program");
    let mut runtime = Runtime::new(&program, EmptyHost);
    runtime.call("Label", function, args)
}

fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

#[test]
fn text_joins_with_what_has_a_spelling() {
    assert_eq!(
        call("score", vec![Value::Number(3.0), Value::Number(0.0)]),
        Ok(text("Blue 3 - 0 Red"))
    );
    assert_eq!(call("many", vec![Value::Number(-2.0)]), Ok(text("-2")));
    assert_eq!(call("many", vec![Value::Number(-0.0)]), Ok(text("0")));
    assert_eq!(call("many", vec![Value::Number(2.5)]), Ok(text("2.5")));
    // What a script wrote as `0.1` reads back as `0.1`, not its f64 widening.
    assert_eq!(
        call("many", vec![Value::Number(f64::from(0.1_f32))]),
        Ok(text("0.1"))
    );
    assert_eq!(
        call("parts", vec![]),
        Ok(text("phase: Play, ready true at (1.5, -2)"))
    );
}

#[test]
fn text_answers_by_character() {
    assert_eq!(call("length", vec![text("héllo")]), Ok(Value::Number(5.0)));
    assert_eq!(call("shout", vec![text("  go! ")]), Ok(text("GO!")));
    assert_eq!(call("quiet", vec![text("GO")]), Ok(text("go")));
    assert_eq!(
        call("has", vec![text("fireball"), text("ball")]),
        Ok(Value::Bool(true))
    );
    assert_eq!(call("edges", vec![text("d9")]), Ok(Value::Bool(true)));
    assert_eq!(call("edges", vec![text("d8")]), Ok(Value::Bool(false)));
    assert_eq!(
        call("find", vec![text("héllo"), text("l")]),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        call("find", vec![text("hello"), text("z")]),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(call("replace", vec![text("foo")]), Ok(text("f00")));
}

#[test]
fn a_slice_is_held_to_the_text_and_refuses_a_fraction() {
    let slice = |start: f64, end: f64| {
        call(
            "slice",
            vec![text("héllo"), Value::Number(start), Value::Number(end)],
        )
    };
    assert_eq!(slice(1.0, 3.0), Ok(text("él")));
    assert_eq!(slice(-4.0, 99.0), Ok(text("héllo")));
    assert_eq!(slice(3.0, 1.0), Ok(text("")));
    assert_eq!(slice(0.5, 2.0), Err(RuntimeError::IndexNotWhole(0.5)));
}

#[test]
fn text_cannot_grow_without_bound() {
    assert_eq!(
        call("grow", vec![]),
        Err(RuntimeError::TextTooLong { limit: TEXT_LIMIT })
    );
}
