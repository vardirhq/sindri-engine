//! A struct's methods at runtime: asked of one value, with `this` that value.

use crate::{EmptyHost, Runtime, Value};

const SOURCE: &str = r#"struct Offer {
    name: String,
    weight: f32,
    at: Vec2,

    fn heavier(other: Offer) -> bool { return this.weight > other.weight; }
    fn label() -> String { return this.name + " x" + this.weight.fixed(1); }
    fn doubled() -> Offer {
        return Offer(name: this.name, weight: this.weight * 2.0, at: this.at);
    }
    fn reach() -> f32 { return this.at.length + this.doubled().weight; }
}
fn best(offers: List<Offer>) -> Offer {
    var found = offers[0];
    for offer in offers { if offer.heavier(found) { found = offer; } }
    return found;
}
script Chooser {
    var held = Offer(name: "held", weight: 1.0, at: Vec2(0.0, 0.0));
    fn compare() -> bool {
        let a = Offer(name: "a", weight: 3.0, at: Vec2(3.0, 4.0));
        return a.heavier(held) && !held.heavier(a);
    }
    fn label() -> String { return Offer(name: "Arc", weight: 2.5, at: Vec2(0.0, 0.0)).label(); }
    fn chain() -> f32 { return held.doubled().doubled().weight; }
    fn nested() -> f32 { return Offer(name: "n", weight: 2.0, at: Vec2(3.0, 4.0)).reach(); }
    fn unchanged() -> f32 {
        let twice = held.doubled();
        return held.weight * 10.0 + twice.weight;
    }
    fn from_shared() -> String {
        var offers: List<Offer> = [];
        offers.push(Offer(name: "light", weight: 1.0, at: Vec2(0.0, 0.0)));
        offers.push(Offer(name: "heavy", weight: 4.0, at: Vec2(0.0, 0.0)));
        return best(offers).name;
    }
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
fn a_method_is_asked_of_one_value() {
    assert_eq!(call("compare"), Value::Bool(true));
    assert_eq!(call("label"), Value::String("Arc x2.5".to_owned()));
    assert_eq!(call("chain"), Value::Number(4.0));
    // `this.at.length` is 5, and a method may call another on `this`.
    assert_eq!(call("nested"), Value::Number(9.0));
    assert_eq!(call("from_shared"), Value::String("heavy".to_owned()));
}

#[test]
fn a_method_works_on_a_copy_and_leaves_the_value_alone() {
    assert_eq!(call("unchanged"), Value::Number(12.0));
}
