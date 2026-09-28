//! Text: joined with `+`, and asked what it contains.

use crate::{Environment, analyze_with_environment};

fn messages(source: &str) -> Vec<String> {
    analyze_with_environment(source, &Environment::new())
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn text_joins_and_answers() {
    let found = messages(
        r#"enum Phase { Lobby, Play }
         script Label {
             var shown = "";
             fn update(dt: f32) {
                 shown = "score " + 3.0 + ", " + true + " " + Phase.Play + " " + Vec2(1.0, 2.0);
                 shown += "!";
                 let n: f32 = shown.length + shown.find("s");
                 let yes: bool = shown.contains("x") || shown.starts_with("s") || shown.ends_with("!");
                 let part: String = shown.slice(0.0, n).uppercase.lowercase.trimmed.replace("a", "b");
             }
         }"#,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_mistake_about_text_is_refused() {
    let found = messages(
        r#"script Board {
             var label = "hi";
             var target: Entity = null;
             fn update(dt: f32) {
                 let a = label - "i";
                 let b = label * 2.0;
                 let c = label + target;
                 let d = label.size;
                 let e = label.length();
                 let f = label.contains;
                 let g: f32 = label.slice(0.0, 1.0);
                 let h = label.contains(1.0);
             }
         }"#,
    );
    for expected in [
        "text is only joined, with `+`; found `String` - `String`",
        "text is only joined, with `+`; found `String` * `f32`",
        "`Entity` has no spelling to join to text",
        "`String` has no member `size`; it has `length`",
        "`length` is a property, not a function -- write `.length`",
        "`contains` on `String` needs arguments -- call it: `.contains(...)`",
        "cannot assign `String` to `f32`",
        "cannot assign `f32` to `String`",
    ] {
        assert!(
            found.iter().any(|message| message.contains(expected)),
            "missing {expected:?} in {found:?}"
        );
    }
}

#[test]
fn a_number_is_written_as_text_by_fixed_and_padded() {
    let analysis = crate::analyze(
        "script S { fn f(n: f32) -> String { return n.fixed(2) + \" / \" + (n * 2.0).padded(3); } }",
    );
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn every_mistake_writing_a_number_is_refused() {
    let found: Vec<String> = crate::analyze(
        r#"script S { fn f(n: f32) {
            let a = n.fixed;
            let b = n.fixed("2");
            let c = n.fixed(1.0, 2.0);
            let d = n.round(1.0);
            let e = "7".padded(2);
        } }"#,
    )
    .diagnostics
    .into_iter()
    .map(|diagnostic| diagnostic.message)
    .collect();
    let all = found.join("\n");
    for expected in [
        "`fixed` on `f32` needs arguments",
        "cannot assign `String` to `f32`",
        "expected 1 argument(s), found 2",
        "`f32` has no member `round`; it has `fixed(...)`, `padded(...)`",
        "`String` has no member `padded`",
    ] {
        assert!(all.contains(expected), "missing {expected:?} in:\n{all}");
    }
    assert_eq!(found.len(), 5, "{all}");
}
