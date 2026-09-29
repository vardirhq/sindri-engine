//! Maps at runtime: written, read by key, changed in place through the
//! variable or field that holds one, and copied where they are assigned.

use crate::{EmptyHost, Runtime, RuntimeError, Value};

const SOURCE: &str = r#"enum Kind { Drone, Blade }
struct Stash { counts: Map<String, f32> }
script Ledger {
    var kills: Map<Kind, f32> = [:];
    var stash = Stash(counts: ["cores": 1.0]);
    fn written() -> f32 {
        let prices = ["arc": 3.0, "nova": 5.0, "arc": 4.0];
        return prices["arc"] * 10.0 + prices.length;
    }
    fn counted() -> f32 {
        kills[Kind.Drone] = 1.0;
        kills[Kind.Drone] += 2.0;
        kills[Kind.Blade] = 7.0;
        return kills[Kind.Drone] * 10.0 + kills.length;
    }
    fn asked() -> String {
        var m = ["b": 2.0, "a": 1.0];
        m["c"] = 3.0;
        let found = m.get("a", 0.0) + m.get("zzz", 100.0);
        return "" + m.keys()[0] + m.keys()[2] + " " + found + " " + m.contains("b") + " " + m.values()[1];
    }
    fn removed() -> String {
        var m = ["a": 1.0, "b": 2.0, "c": 3.0];
        let gone = m.remove("b");
        let again = m.remove("b");
        let order = m.keys()[0] + m.keys()[1];
        m.clear();
        return order + " " + gone + " " + again + " " + m.length;
    }
    fn copies() -> bool {
        var a = ["x": 1.0];
        var b = a;
        b["x"] = 5.0;
        b["y"] = 6.0;
        return a["x"] == 1.0 && a.length == 1.0 && b["x"] == 5.0 && a != b;
    }
    fn nested() -> f32 {
        this.stash.counts["cores"] += 4.0;
        stash.counts["shards"] = 2.0;
        return stash.counts["cores"] * 10.0 + stash.counts["shards"];
    }
    fn missing() -> f32 { let m = ["a": 1.0]; return m["b"]; }
    fn empty_fits() -> f32 { var m: Map<String, f32> = [:]; m["x"] = 2.0; return m.length; }
}"#;

fn call(function: &str) -> Result<Value, RuntimeError> {
    let lowered = decay_ir::lower(SOURCE);
    let program = lowered
        .program
        .unwrap_or_else(|| panic!("{:?}", lowered.analysis.diagnostics));
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut ledger = runtime.instantiate("Ledger").expect("instance");
    runtime.call_instance(&mut ledger, function, vec![])
}

#[test]
fn a_map_is_written_read_and_counted() {
    // A key written twice keeps its last value and its first place.
    assert_eq!(call("written"), Ok(Value::Number(42.0)));
    assert_eq!(call("counted"), Ok(Value::Number(32.0)));
    assert_eq!(call("empty_fits"), Ok(Value::Number(1.0)));
}

#[test]
fn a_map_answers_in_the_order_its_keys_were_set() {
    assert_eq!(call("asked"), Ok(Value::String("bc 101 true 1".to_owned())));
    assert_eq!(
        call("removed"),
        Ok(Value::String("ac true false 0".to_owned()))
    );
}

#[test]
fn a_map_is_a_value_copied_where_it_is_assigned() {
    assert_eq!(call("copies"), Ok(Value::Bool(true)));
    assert_eq!(call("nested"), Ok(Value::Number(52.0)));
}

#[test]
fn a_key_the_map_does_not_have_is_named_when_it_is_read() {
    assert_eq!(
        call("missing"),
        Err(RuntimeError::MissingKey("\"b\"".to_owned()))
    );
}
