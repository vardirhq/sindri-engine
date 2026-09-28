//! Does the language reference describe its enums, text, lists and structs?
//!
//! The same contract as `language_reference.rs`, split from it by size: when
//! one of these fails, the language changed and `decay/LANGUAGE.md` is now
//! wrong.

mod reference;

use reference::{accepted, rejected};

#[test]
fn enums_do_what_the_reference_says() {
    accepted(
        "an enum held, compared and matched",
        "enum Phase { Lobby, Countdown, Play }
         state Game { var phase = Phase.Lobby; }
         script T {
             var phase = Phase.Lobby;
             fn go(to: Phase) { this.phase = to; Game.phase = to; }
             fn update(dt: f32) {
                 if this.phase != Phase.Play { go(Phase.Countdown); }
                 match this.phase {
                     Phase.Lobby => { }
                     Phase.Countdown | Phase.Play => { }
                 }
                 match this.phase { Phase.Play => { } _ => { } }
             }
         }",
    );
    accepted(
        "`enum` as an ordinary name",
        "script T { fn f() { let enum = 1.0; } }",
    );
    rejected(
        "a variant without its enum's name",
        "enum P { A, B } script T { var p = A; }",
    );
    rejected(
        "arithmetic on a variant",
        "enum P { A, B } script T { fn f() { let x = P.A + 1.0; } }",
    );
    rejected(
        "ordering variants",
        "enum P { A, B } script T { fn f() -> bool { return P.A < P.B; } }",
    );
    rejected(
        "comparing variants of two enums",
        "enum P { A } enum Q { A } script T { fn f() -> bool { return P.A == Q.A; } }",
    );
    rejected(
        "a `match` that misses a variant",
        "enum P { A, B } script T { fn f(p: P) { match p { P.A => { } } } }",
    );
    rejected(
        "a variant matched twice",
        "enum P { A, B } script T { fn f(p: P) { match p { P.A => { } P.A | P.B => { } } } }",
    );
    rejected(
        "an arm after `_`",
        "enum P { A, B } script T { fn f(p: P) { match p { _ => { } P.A => { } } } }",
    );
    rejected("an enum with no variants", "enum P { } script T { }");
    rejected("a variant declared twice", "enum P { A, A } script T { }");
    rejected(
        "calling an enum",
        "enum P { A } script T { fn f() { let p = P(); } }",
    );
    rejected(
        "`match` as a name",
        "script T { fn f() { let match = 1.0; } }",
    );
}

#[test]
fn text_does_what_the_reference_says() {
    accepted(
        "text joined with what has a spelling, and asked by a dot",
        r#"enum Phase { Lobby }
         script T {
             var label = "";
             fn f(score: f32) -> bool {
                 label = "Score " + score + " " + true + " " + Phase.Lobby + " " + Vec2(1.0, 2.0);
                 label += "!";
                 let n: f32 = label.length + label.find("S");
                 let part: String = label.slice(0.0, 3.0).uppercase.lowercase.trimmed.replace("a", "b");
                 return label.contains("S") && label.starts_with("S") && label.ends_with("!");
             }
         }"#,
    );
    rejected(
        "joining something with no spelling",
        r#"script T { fn f(e: Entity) { let s = "a" + e; } }"#,
    );
    rejected(
        "`-` on text",
        r#"script T { fn f() { let s = "ab" - "b"; } }"#,
    );
    rejected(
        "ordering text",
        r#"script T { fn f() -> bool { return "a" < "b"; } }"#,
    );
    rejected(
        "calling a text property",
        r#"script T { fn f() -> f32 { return "ab".length(); } }"#,
    );
    rejected(
        "a text method that does not exist",
        r#"script T { fn f() -> f32 { return "ab".size; } }"#,
    );
}

#[test]
fn lists_and_ranges_do_what_the_reference_says() {
    accepted(
        "a list written, walked by a range, asked and changed",
        r#"script T {
             var held: List<f32> = [];
             fn f() -> f32 {
                 var names = ["a", "b"];
                 names[0] = "c";
                 names.insert(1, "d");
                 names.push("e");
                 let gone: String = names.remove_at(0) + names.pop();
                 for i in 0..names.length { held.push(i); }
                 if names.contains("d") { held.clear(); }
                 return names.index_of("d") + held.length;
             }
         }"#,
    );
    accepted(
        "`len` as the older spelling of `length`",
        "script T { fn f() -> f32 { return [1.0].len; } }",
    );
    accepted(
        "`Array<T>` as the older spelling of `List<T>`",
        "script T { var a: Array<f32> = [1.0]; }",
    );
    rejected(
        "a list where a number goes",
        "script T { fn f() { let a: f32 = [1.0]; } }",
    );
    rejected(
        "a list of two types",
        r#"script T { fn f() { let a = [1.0, "b"]; } }"#,
    );
    rejected(
        "changing a `let` list",
        "script T { fn f() { let a = [1.0]; a.push(2.0); } }",
    );
    rejected(
        "changing a list that is not in a variable",
        "script T { fn f() { [1.0].push(2.0); } }",
    );
    rejected(
        "calling `length`",
        "script T { fn f() -> f32 { return [1.0].length(); } }",
    );
    rejected(
        "a range of something but numbers",
        "script T { fn f() { for i in 0..true { } } }",
    );
}

#[test]
fn structs_do_what_the_reference_says() {
    accepted(
        "a struct built, compared, walked in a list and written through its holder",
        r#"struct Offer { index: f32, weight: f32, name: String }
         struct Slot { offer: Offer, at: Vec2, cards: List<f32> }
         script T {
             var offers: List<Offer> = [];
             var best = Offer(index: -1.0, weight: 0.0, name: "none");
             var slot = Slot(offer: Offer(name: "a", index: 1.0, weight: 2.0), at: Vec2(0.0, 0.0), cards: []);
             fn f(index: f32) -> bool {
                 offers.push(Offer(index: index, weight: 1.0, name: "module " + index));
                 for offer in offers { if offer.weight > best.weight { best = offer; } }
                 best.weight += 1.0;
                 this.slot.offer.name = "Nova";
                 slot.at.x += 1.0;
                 slot.cards.push(1.0);
                 var o = offers[0];
                 o.weight = 2.0;
                 offers[0] = o;
                 return best == slot.offer;
             }
         }"#,
    );
    rejected(
        "a field left out",
        "struct P { a: f32, b: f32 } script T { fn f() { let p = P(a: 1.0); } }",
    );
    rejected(
        "positional arguments",
        "struct P { a: f32 } script T { fn f() { let p = P(1.0); } }",
    );
    rejected(
        "writing a field of a `let`",
        "struct P { a: f32 } script T { fn f() { let p = P(a: 1.0); p.a = 2.0; } }",
    );
    rejected(
        "writing a field of a list's element in place",
        "struct P { a: f32 } script T { fn f() { var ps = [P(a: 1.0)]; ps[0].a = 2.0; } }",
    );
    rejected(
        "joining a struct to text",
        r#"struct P { a: f32 } script T { fn f() { let s = "p " + P(a: 1.0); } }"#,
    );
    rejected("a struct with no fields", "struct P { } script T { }");
}
