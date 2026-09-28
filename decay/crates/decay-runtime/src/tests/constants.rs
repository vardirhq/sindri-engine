//! Constants at runtime: every use is the value worked out when the file
//! compiled.

use crate::{EmptyHost, Runtime, Value};

const SOURCE: &str = r#"enum Phase { Lobby, Play }
const ARENA: f32 = 12.0;
const HALF: f32 = ARENA / 2.0;
const START: Phase = Phase.Play;
const TITLE: String = "Orbital" + " Baked";
const WIDE: bool = HALF > 4.0;
fn edge() -> f32 { return HALF - 1.0; }
script Director {
    var size: f32 = ARENA * 2.0;
    var phase: Phase = START;
    fn numbers() -> f32 { return size + edge() + HALF; }
    fn variant() -> bool { return phase == Phase.Play && START == phase; }
    fn text() -> String { return TITLE + " " + ARENA; }
    fn flag() -> bool { return WIDE; }
}"#;

fn call(function: &str) -> Value {
    let lowered = decay_ir::lower(SOURCE);
    let program = lowered
        .program
        .unwrap_or_else(|| panic!("{:?}", lowered.analysis.diagnostics));
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut director = runtime.instantiate("Director").expect("instance");
    runtime
        .call_instance(&mut director, function, vec![])
        .expect("runs")
}

#[test]
fn a_constant_is_its_value_wherever_it_is_used() {
    assert_eq!(call("numbers"), Value::Number(35.0));
    assert_eq!(call("variant"), Value::Bool(true));
    assert_eq!(call("text"), Value::String("Orbital Baked 12".to_owned()));
    assert_eq!(call("flag"), Value::Bool(true));
}

#[test]
fn a_constant_is_never_looked_up() {
    let lowered = decay_ir::lower(SOURCE);
    let program = lowered.program.expect("compiles");
    let loads = format!("{:?}", program.containers);
    for name in ["ARENA", "HALF", "START", "TITLE", "WIDE"] {
        assert!(
            !loads.contains(&format!("\"{name}\"")),
            "{name} is loaded by name rather than written in place"
        );
    }
}
