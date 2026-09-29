//! A struct field's default: what the field holds when a struct is built
//! without naming it.

use crate::{EmptyHost, Runtime, Value};

const SOURCE: &str = r#"enum Rarity { Common, Rare }
const BASE: f32 = 2.0;
struct Offer {
    name: String,
    weight: f32 = BASE * 1.5,
    rarity: Rarity = Rarity.Common,
    label: String = "Module",
    taken: bool = false,
}
script Chooser {
    var held = Offer(name: "held");
    fn plain() -> f32 { return Offer(name: "a").weight; }
    fn named() -> f32 { return Offer(weight: 9.0, name: "b").weight; }
    fn rarity() -> bool { return Offer(name: "c").rarity == Rarity.Common; }
    fn text() -> String { return Offer(name: "d").label + " " + Offer(name: "e", label: "Card").label; }
    fn flag() -> bool { return Offer(name: "f").taken; }
    fn field() -> f32 { return held.weight; }
}"#;

fn call(function: &str) -> Value {
    let lowered = decay_ir::lower(SOURCE);
    let program = lowered
        .program
        .unwrap_or_else(|| panic!("{:?}", lowered.analysis.diagnostics));
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut chooser = runtime.instantiate("Chooser").expect("instance");
    runtime
        .call_instance(&mut chooser, function, vec![])
        .expect("runs")
}

#[test]
fn a_field_left_out_holds_its_default() {
    assert_eq!(call("plain"), Value::Number(3.0));
    assert_eq!(call("rarity"), Value::Bool(true));
    assert_eq!(call("flag"), Value::Bool(false));
    assert_eq!(call("field"), Value::Number(3.0));
}

#[test]
fn a_field_named_holds_what_it_was_given() {
    assert_eq!(call("named"), Value::Number(9.0));
    assert_eq!(call("text"), Value::String("Module Card".to_owned()));
}
