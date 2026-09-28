//! Structs at runtime: built with their fields in any order, read, written
//! through the variable that holds one, and copied where they are assigned.

use crate::{EmptyHost, Runtime, Value};

const SOURCE: &str = r#"struct Offer { index: f32, weight: f32, name: String }
struct Slot { offer: Offer, at: Vec2 }
struct Hand { cards: List<f32>, best: f32 }
script Chooser {
    var stash = Hand(cards: [], best: 0.0);
    fn hand() -> f32 {
        var h = Hand(cards: [], best: 0.0);
        h.cards.push(2.0);
        h.cards.push(3.0);
        h.cards[0] += 5.0;
        this.stash.cards.push(1.0);
        let copy = h;
        h.cards.clear();
        return copy.cards[0] * 10.0 + copy.cards.length + stash.cards.length + h.cards.length;
    }
    var best: Offer = Offer(index: -1.0, weight: 0.0, name: "none");
    var offers: List<Offer> = [];
    fn build() -> f32 {
        let o = Offer(name: "Arc", weight: 2.0, index: 7.0);
        return o.index * 10.0 + o.weight;
    }
    fn write() -> f32 {
        var o = Offer(index: 1.0, weight: 1.0, name: "a");
        o.weight += 4.0;
        o.name = "b";
        best = o;
        this.best.index = 9.0;
        return o.weight * 10.0 + best.index;
    }
    fn copies() -> bool {
        var a = Offer(index: 1.0, weight: 1.0, name: "a");
        var b = a;
        b.weight = 5.0;
        return a.weight == 1.0 && b.weight == 5.0 && a != b;
    }
    fn nested() -> f32 {
        var s = Slot(offer: Offer(index: 3.0, weight: 1.0, name: "c"), at: Vec2(1.0, 2.0));
        s.at.x = 5.0;
        return s.offer.index * 100.0 + s.at.x * 10.0 + s.at.y;
    }
    fn heaviest() -> String {
        offers.push(Offer(index: 0.0, weight: 1.0, name: "light"));
        offers.push(Offer(index: 1.0, weight: 3.0, name: "heavy"));
        var found = offers[0];
        for offer in offers {
            if offer.weight > found.weight { found = offer; }
        }
        return found.name + " " + offers.length;
    }
}"#;

fn call(function: &str) -> Value {
    let lowered = decay_ir::lower(SOURCE);
    let program = lowered
        .program
        .unwrap_or_else(|| panic!("{:?}", lowered.analysis.diagnostics));
    let mut runtime = Runtime::new(&program, EmptyHost);
    runtime.call("Chooser", function, vec![]).expect("runs")
}

#[test]
fn a_struct_is_built_read_written_and_copied() {
    assert_eq!(call("build"), Value::Number(72.0));
    assert_eq!(call("write"), Value::Number(59.0));
    assert_eq!(call("copies"), Value::Bool(true));
    assert_eq!(call("nested"), Value::Number(352.0));
    assert_eq!(call("heaviest"), Value::String("heavy 2".to_owned()));
    // A list inside a struct changes in place, and a copy keeps its own.
    assert_eq!(call("hand"), Value::Number(73.0));
}

#[test]
fn a_struct_field_keeps_its_value_between_calls() {
    let lowered = decay_ir::lower(SOURCE);
    let program = lowered
        .program
        .unwrap_or_else(|| panic!("{:?}", lowered.analysis.diagnostics));
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut chooser = runtime.instantiate("Chooser").expect("instance");
    runtime
        .call_instance(&mut chooser, "write", vec![])
        .expect("writes");
    let Some(Value::Struct { shape, fields }) = chooser.field("best") else {
        panic!("best is a struct: {:?}", chooser.field("best"));
    };
    assert_eq!(shape.name, "Offer");
    assert_eq!(
        **fields,
        vec![
            Value::Number(9.0),
            Value::Number(5.0),
            Value::String("b".to_owned())
        ]
    );
}
