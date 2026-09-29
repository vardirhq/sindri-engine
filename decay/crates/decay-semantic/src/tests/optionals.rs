//! Values that may be missing: an optional is never used as its type until a
//! fallback or a check says which it is.

use crate::{Environment, analyze_with_environment};

fn messages(source: &str) -> Vec<String> {
    analyze_with_environment(source, &Environment::new())
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn an_optional_is_used_through_a_fallback_or_a_check() {
    let found = messages(
        r"script S {
             var best: f32? = null;
             var names: List<String>? = null;
             fn pick(a: f32?, b: f32?) -> f32 {
                 if a != null && b != null { return a + b; }
                 if a == null { return b ?? 0.0; }
                 return a * 2.0;
             }
             fn later(a: f32?) -> f32? {
                 let fallback: f32? = best ?? a;
                 best = 3.0;
                 best = null;
                 if a == null || fallback == null { return null; } else { return a + fallback; }
             }
             fn listed() -> f32 { return (names ?? []).length + (best ?? 1.0) * 2.0; }
             fn compared(a: f32?) -> bool { return a == 1.0 || a == null; }
         }",
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_misuse_of_an_optional_is_refused() {
    let found = messages(
        r#"script S {
             var best: f32? = null;
             var names: List<String>? = null;
             fn f(n: f32, text: String?) {
                 let plain: f32 = best;
                 let sum = best + 1.0;
                 let size = text.length;
                 names.push("a");
                 let first = names[0];
                 let never = n ?? 2.0;
                 let wrong = best ?? "zero";
                 if best != null { let used: f32 = best; }
             }
         }"#,
    );
    let all = found.join("\n");
    for expected in [
        "cannot assign `f32?` to `f32` -- it may be `null`: give a fallback with `?? value`",
        "expected `f32`, found `f32?` -- it may be `null`",
        "this is `String?`, which may be `null`",
        "this is `List<String>?`, which may be `null`",
        "this is `f32`, which is never `null`, so `??` never uses its fallback",
        "the fallback is `String`, but the value is `f32`",
    ] {
        assert!(all.contains(expected), "missing {expected:?} in:\n{all}");
    }
    // A `var` field is not narrowed by a check: it could change in between.
    assert!(
        found
            .iter()
            .filter(|message| message.contains("cannot assign `f32?` to `f32`"))
            .count()
            >= 2,
        "{all}"
    );
}
