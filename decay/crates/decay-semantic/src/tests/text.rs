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
