//! Does the language reference describe its enums and its text?
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
