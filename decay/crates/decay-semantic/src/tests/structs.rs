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
        "`Card` needs every field: missing `weight`",
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
