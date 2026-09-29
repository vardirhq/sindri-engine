//! Values that may be missing: `f32?` holds a number or `null`, `??` falls
//! back, and a checked `let` is its plain type.

use crate::{EmptyHost, Runtime, Value};

const SOURCE: &str = r#"script Picker {
    var best: f32? = null;
    var label: String? = null;
    fn first_over(values: List<f32>, limit: f32) -> f32? {
        for value in values { if value > limit { return value; } }
        return null;
    }
    fn found() -> f32 { return first_over([1.0, 5.0, 9.0], 4.0) ?? -1.0; }
    fn missing() -> f32 { return first_over([1.0, 2.0], 4.0) ?? -1.0; }
    fn held() -> f32 {
        let before = best ?? 0.0;
        best = 7.0;
        return before * 10.0 + (best ?? 0.0);
    }
    fn narrowed(values: List<f32>) -> f32 {
        let pick = first_over(values, 0.0);
        if pick != null { return pick * 2.0; }
        return -1.0;
    }
    fn early(values: List<f32>) -> f32 {
        let pick = first_over(values, 0.0);
        if pick == null { return 0.0; }
        return pick + 100.0;
    }
    fn chained() -> String { return label ?? "none"; }
    fn only_when_needed() -> f32 {
        var calls = 0.0;
        let a: f32? = 3.0;
        let b = a ?? count(calls);
        return b;
    }
    fn count(n: f32) -> f32 { return n + 1.0; }
    fn compared() -> bool { let x: f32? = null; return x == null && best != 7.0; }
    fn binding() -> f32 { return (best ?? 1.0) + 2.0 * 3.0; }
}"#;

fn call(function: &str, args: Vec<Value>) -> Value {
    let lowered = decay_ir::lower(SOURCE);
    let program = lowered
        .program
        .unwrap_or_else(|| panic!("{:?}", lowered.analysis.diagnostics));
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut picker = runtime.instantiate("Picker").expect("instance");
    runtime
        .call_instance(&mut picker, function, args)
        .expect("runs")
}

fn numbers(values: &[f64]) -> Value {
    Value::array(values.iter().copied().map(Value::Number).collect())
}

#[test]
fn a_fallback_is_used_only_for_null() {
    assert_eq!(call("found", vec![]), Value::Number(5.0));
    assert_eq!(call("missing", vec![]), Value::Number(-1.0));
    assert_eq!(call("held", vec![]), Value::Number(7.0));
    assert_eq!(call("chained", vec![]), Value::String("none".to_owned()));
    assert_eq!(call("only_when_needed", vec![]), Value::Number(3.0));
    assert_eq!(call("compared", vec![]), Value::Bool(true));
    // `??` binds looser than arithmetic: `(best ?? 1) + 6`.
    assert_eq!(call("binding", vec![]), Value::Number(7.0));
}

#[test]
fn a_checked_value_is_used_as_its_type() {
    assert_eq!(call("narrowed", vec![numbers(&[3.0])]), Value::Number(6.0));
    assert_eq!(call("narrowed", vec![numbers(&[])]), Value::Number(-1.0));
    assert_eq!(call("early", vec![numbers(&[2.0])]), Value::Number(102.0));
    assert_eq!(call("early", vec![numbers(&[])]), Value::Number(0.0));
}
