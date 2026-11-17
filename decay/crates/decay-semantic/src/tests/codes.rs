//! Each diagnostic carries its stable code, whichever phase raised it.

use crate::{DiagnosticPhase, analyze};

fn codes(source: &str) -> Vec<(DiagnosticPhase, &'static str)> {
    analyze(source)
        .diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.phase, diagnostic.code))
        .collect()
}

#[test]
fn a_semantic_diagnostic_names_what_went_wrong() {
    let cases = [
        ("fn start() { missing(); }", "unknown-name"),
        ("fn start() { let x = 1; x = 2; }", "immutable"),
        ("fn start() { if 1 { } }", "condition-not-bool"),
        ("fn start() { break; }", "outside-loop"),
        ("fn start() { let x: f32 = true; }", "type-mismatch"),
    ];
    for (body, expected) in cases {
        let source = format!("script Probe {{ {body} }}");
        assert!(
            codes(&source).contains(&(DiagnosticPhase::Semantic, expected)),
            "`{source}` should report `{expected}`: {:?}",
            codes(&source)
        );
    }
}

#[test]
fn a_syntax_diagnostic_carries_its_own_code() {
    let found = codes("script Probe { fn start( { }");
    assert!(
        found
            .iter()
            .all(|(phase, _)| *phase == DiagnosticPhase::Syntax),
        "{found:?}"
    );
    assert!(
        found.contains(&(DiagnosticPhase::Syntax, "expected-token")),
        "{found:?}"
    );
}

#[test]
fn a_let_field_cannot_be_assigned_through_this() {
    for body in [
        "let speed: number = 1.0; fn update(dt: number) { this.speed = 2.0; }",
        "@export let speed: number = 1.0; fn update(dt: number) { this.speed += dt; }",
        "let speed: number = 1.0; fn update(dt: number) { speed = 2.0; }",
    ] {
        let source = format!("script Probe {{ {body} }}");
        assert!(
            codes(&source).contains(&(DiagnosticPhase::Semantic, "immutable")),
            "`{source}` should be refused: {:?}",
            codes(&source)
        );
    }
    let fine =
        "script Probe { var speed: number = 1.0; fn update(dt: number) { this.speed = 2.0; } }";
    assert!(codes(fine).is_empty(), "{:?}", codes(fine));
}
