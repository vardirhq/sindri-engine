//! Lists and ranges: written, walked, asked, and changed where they are held.

use crate::{Environment, analyze_with_environment};

fn messages(source: &str) -> Vec<String> {
    analyze_with_environment(source, &Environment::new())
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn lists_and_ranges_are_accepted() {
    let found = messages(
        r#"script Bag {
             var held: List<f32> = [];
             var older: Array<f32> = [1.0];
             var ids = [1.0, 2.0];
             fn update(dt: f32) {
                 for i in 0..ids.length { held.push(ids[i] + i); }
                 var names = ["a", "b"];
                 names[0] = "c";
                 names.insert(1, "d");
                 let gone: String = names.remove_at(0);
                 let last: f32 = this.held.pop();
                 let has: bool = names.contains("d") && names.index_of("d") >= 0;
                 held.clear();
                 older = held;
                 ids[0] += 1.0;
             }
         }"#,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_mistake_about_a_list_is_refused() {
    let found = messages(
        r#"script Bag {
             let fixed = [1.0];
             fn update(dt: f32) {
                 let mixed = [1.0, "two"];
                 let nums: List<f32> = ["a"];
                 fixed.push(2.0);
                 fixed[0] = 3.0;
                 var xs = [1.0];
                 xs.push("a");
                 xs.shove(1.0);
                 let f = xs.push;
                 let n = xs.length();
                 [1.0].push(2.0);
                 let r = 0..3;
                 for i in 0..true { }
                 xs["a"] = 1.0;
             }
         }"#,
    );
    for expected in [
        "a list holds one type: this is `String`, and the first element is `f32`",
        "cannot assign `List<String>` to `List<f32>`",
        "`fixed` is a `let`, so it cannot change -- declare it `var` if `push` can change it",
        "`fixed` is a `let`, so it cannot change -- declare it `var` if an element can be set",
        "cannot assign `String` to `f32`",
        "`List<f32>` has no member `shove`; it has `length`, and `push`",
        "`push` on a list is a function -- call it: `.push(...)`",
        "`length` is a property, not a function -- write `.length`",
        "only a list in a variable or a field of this script can change",
        "expected `;` after binding",
        "expected `f32`, found `bool`",
        "expected `f32`, found `String`",
    ] {
        assert!(
            found.iter().any(|message| message.contains(expected)),
            "missing {expected:?} in {found:?}"
        );
    }
}
