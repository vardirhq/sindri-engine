//! Constants: worked out when the file compiles, usable anywhere in it, and
//! never changed.

use crate::{ConstValue, Environment, analyze, analyze_with_environment};

fn messages(source: &str, environment: &Environment) -> Vec<String> {
    analyze_with_environment(source, environment)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn a_constant_is_usable_anywhere_in_its_file() {
    let found = messages(
        r#"enum Phase { Lobby, Play }
         const ARENA: f32 = 12.0;
         const HALF: f32 = ARENA / 2.0;
         const START: Phase = Phase.Lobby;
         const TITLE: String = "Orbital" + " Baked";
         const WIDE: bool = HALF > 4.0 && !false;
         fn inside(x: f32) -> bool { return x < HALF; }
         script Director {
             var phase: Phase = START;
             var size: f32 = ARENA * 2.0;
             fn update(dt: f32) {
                 if inside(size - LATE) && WIDE { phase = START; }
                 let label: String = TITLE + " " + ARENA;
             }
         }
         const LATE: f32 = -HALF;"#,
        &Environment::new(),
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_constant_is_worked_out_once_with_its_type() {
    let analysis =
        analyze(r#"const A: f32 = 3.0; const B: f32 = A * A + 1.0; const C: String = "x" + "y";"#);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    assert_eq!(analysis.constants["B"], ConstValue::Number(10.0));
    assert_eq!(analysis.constants["C"], ConstValue::Text("xy".to_owned()));
}

#[test]
fn every_mistake_about_a_constant_is_refused() {
    let found = messages(
        r#"const LOOP_A: f32 = LOOP_B + 1.0;
         const LOOP_B: f32 = LOOP_A;
         const CALLED: f32 = abs(-1.0);
         const WRONG: bool = 1.0;
         const JOINED: String = "x" + 1.0;
         const ZERO: f32 = 1.0 / 0.0;
         const SPOT: Vec2 = Vec2(1.0, 2.0);
         const TWICE: f32 = 1.0;
         const TWICE: f32 = 2.0;
         script S {
             var x: f32 = 0.0;
             const_in_body: f32 = 0.0;
         }"#,
        &Environment::new(),
    );
    let all = found.join("\n");
    for expected in [
        "needs its own value",
        "must be worked out when the file compiles",
        "declared `bool` but its value is `f32`",
        "joins text only to text",
        "divides by zero",
        "duplicate declaration `TWICE`",
    ] {
        assert!(all.contains(expected), "missing {expected:?} in:\n{all}");
    }
}

#[test]
fn a_constant_cannot_be_assigned_or_take_another_name() {
    let found = messages(
        r"struct Card { weight: f32 }
         const Card: f32 = 1.0;
         const LIMIT: f32 = 3.0;
         script S { fn update(dt: f32) { LIMIT = 4.0; LIMIT += 1.0; } }",
        &Environment::new(),
    );
    let all = found.join("\n");
    assert!(all.contains("cannot assign to constant `LIMIT`"), "{all}");
    assert!(all.contains("a constant needs one of its own"), "{all}");
}

#[test]
fn a_broken_constant_is_reported_once_not_at_every_use() {
    let found = messages(
        "const BAD: f32 = nothing; script S { fn f() -> f32 { return BAD + BAD; } }",
        &Environment::new(),
    );
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn a_local_hides_a_constant_of_the_same_name() {
    let found = messages(
        "const SPEED: f32 = 1.0; script S { fn f() -> String { let SPEED = \"fast\"; return SPEED; } }",
        &Environment::new(),
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_shared_constant_from_another_file_is_known_and_this_file_wins() {
    let mut environment = Environment::new();
    environment.add_constant("ARENA", ConstValue::Number(12.0));
    environment.add_constant("GAP", ConstValue::Number(1.0));
    let analysis = analyze_with_environment(
        "shared const GAP: f32 = 2.0; const EDGE: f32 = ARENA - GAP;",
        &environment,
    );
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    assert_eq!(analysis.constants["EDGE"], ConstValue::Number(10.0));

    let mut environment = Environment::new();
    environment.add_ambiguous_constant("ARENA");
    let found = messages("script S { fn f() -> f32 { return ARENA; } }", &environment);
    assert!(
        found
            .iter()
            .any(|message| message.contains("more than one file")),
        "{found:?}"
    );
}
