//! Structs: declared, built with every field named, read and written.

use crate::types::Type;
use crate::{Environment, analyze_with_environment};

fn messages(source: &str, environment: &Environment) -> Vec<String> {
    analyze_with_environment(source, environment)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn a_struct_is_accepted_wherever_a_value_goes() {
    let found = messages(
        r#"struct Card { root: Entity, name: String, weight: f32 }
         struct Hand { cards: List<Card>, best: Card }
         script Chooser {
             var hand: Hand = Hand(cards: [], best: Card(root: null, name: "", weight: 0.0));
             fn make(name: String) -> Card { return Card(weight: 1.0, name: name, root: null); }
             fn update(dt: f32) {
                 var card = make("Arc");
                 card.weight += 2.0;
                 hand.cards.push(card);
                 this.hand.best = card;
                 this.hand.best.name = "Nova";
                 let total: f32 = hand.cards[0].weight + hand.best.weight;
                 if card == hand.best { }
             }
         }"#,
        &Environment::new(),
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_struct_from_the_host_is_known_without_a_declaration() {
    let mut environment = Environment::new();
    environment.add_struct("Offer", vec![("index".to_owned(), Type::F32)]);
    let found = messages(
        "script Chooser { fn f() -> f32 { let o = Offer(index: 1.0); return o.index; } }",
        &environment,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_mistake_about_a_struct_is_refused() {
    let found = messages(
        r#"struct Card { name: String, weight: f32 }
         struct Empty { }
         struct Twice { a: f32, a: f32 }
         struct Chooser { a: f32 }
         script Chooser {
             let fixed = Card(name: "a", weight: 1.0);
             fn update(dt: f32) {
                 let a = Card(name: "a");
                 let b = Card(name: "a", weight: "heavy");
                 let c = Card(name: "a", weight: 1.0, colour: 2.0);
                 let d = Card(name: "a", name: "b", weight: 1.0);
                 let e = Card("a", 1.0);
                 let f = a.size;
                 fixed.weight = 2.0;
                 let g = Nothing(x: 1.0);
                 let h: f32 = a;
             }
         }"#,
        &Environment::new(),
    );
    for expected in [
        "`Card` needs every field without a default: missing `weight`",
        "cannot assign `String` to `f32`",
        "`Card` has no field `colour`; it has `name`, `weight`",
        "`name` is given twice",
        "`Card` is a struct: name its fields, as in `Card(name: ..., weight: ...)`",
        "`Card` has no field `size`; it has `name`, `weight`",
        "cannot assign to a component of immutable `fixed`",
        "`Nothing` is not a struct, so it is not built with named fields",
        "cannot assign `Card` to `f32`",
        "struct `Empty` has no fields, so it could never hold anything",
        "`Twice.a` is declared twice",
        "`Chooser` is already a name; a struct needs one of its own",
    ] {
        assert!(
            found.iter().any(|message| message.contains(expected)),
            "missing {expected:?} in {found:?}"
        );
    }
}

#[test]
fn a_struct_method_is_checked_like_any_call() {
    let found = messages(
        r#"struct Card {
             name: String,
             weight: f32,
             fn heavier(other: Card) -> bool { return this.weight > other.weight; }
             fn label() -> String { return this.name + " " + this.weight; }
         }
         script S {
             fn update(dt: f32) {
                 let a = Card(name: "a", weight: 1.0);
                 if a.heavier(Card(name: "b", weight: 2.0)) { }
                 let text: String = a.label();
             }
         }"#,
        &Environment::new(),
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_mistake_about_a_struct_method_is_refused() {
    let found = messages(
        r"struct Card {
             weight: f32,
             fn weight() -> f32 { return 1.0; }
             fn grow() { this.weight = 2.0; }
             fn bare() -> f32 { return weight; }
             fn twice() {}
             fn twice() {}
             fn heavier(other: Card) -> bool { return this.weight > other.weight; }
         }
         script S {
             fn update(dt: f32) {
                 let a = Card(weight: 1.0);
                 let b = a.heavier;
                 let c = a.heavier(1.0);
                 let d = a.lighter(a);
             }
         }",
        &Environment::new(),
    );
    let all = found.join("\n");
    for expected in [
        "`Card.weight` is both a field and a method",
        "a method cannot change the value it was asked of",
        "unknown name `weight`",
        "`Card.twice` is declared twice",
        "`heavier` is a method of `Card` -- call it",
        "cannot assign `f32` to `Card`",
        "`Card` has no method `lighter`; it has `bare`, `grow`, `heavier`, `twice`",
    ] {
        assert!(all.contains(expected), "missing {expected:?} in:\n{all}");
    }
}

#[test]
fn a_method_from_another_file_is_known() {
    let mut environment = Environment::new();
    environment.add_struct("Offer", vec![("index".to_owned(), Type::F32)]);
    environment.add_struct_method(
        "Offer",
        "next",
        crate::FunctionType {
            params: vec![Type::F32],
            return_type: Type::F32,
        },
    );
    let found = messages(
        "script S { fn f() -> f32 { return Offer(index: 1.0).next(2.0); } }",
        &environment,
    );
    assert!(found.is_empty(), "{found:?}");
    let analysis = crate::analyze_with_environment(
        "script S { fn f() -> f32 { return Offer(index: 1.0).next(2.0); } }",
        &environment,
    );
    assert_eq!(
        analysis.method_calls.values().collect::<Vec<_>>(),
        vec!["Offer.next"]
    );
}

#[test]
fn a_field_with_a_default_may_be_left_out() {
    let analysis = crate::analyze(
        r#"const HEAVY: f32 = 3.0;
         enum Rarity { Common, Rare }
         struct Card { name: String, weight: f32 = HEAVY, rarity: Rarity = Rarity.Common }
         script S { fn f() -> f32 { return Card(name: "a").weight + Card(name: "b", weight: 1.0).weight; } }"#,
    );
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    assert_eq!(
        analysis.struct_defaults["Card"]["weight"],
        crate::ConstValue::Number(3.0)
    );
}

#[test]
fn every_mistake_about_a_default_is_refused() {
    let found = messages(
        r"struct Card {
             weight: f32 = true,
             spot: Vec2 = Vec2(1.0, 2.0),
             size: f32 = weight,
             name: String,
         }
         script S { fn f() { let c = Card(weight: 1.0); } }",
        &Environment::new(),
    );
    let all = found.join("\n");
    for expected in [
        "`Card.weight`: field `weight` is declared `f32` but its value is `bool`",
        "`Card.spot`: a field's default must be worked out when the file compiles",
        "`Card.size`: `weight` is not a constant",
        "`Card` needs every field without a default: missing `name`",
    ] {
        assert!(all.contains(expected), "missing {expected:?} in:\n{all}");
    }
}
