//! Maps: written with `[key: value]`, typed `Map<K, V>`, read by key, and
//! changed only where a variable or field may change.

use crate::{Environment, analyze_with_environment};

fn messages(source: &str) -> Vec<String> {
    analyze_with_environment(source, &Environment::new())
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn a_map_is_accepted_wherever_a_value_goes() {
    let found = messages(
        r#"enum Kind { Drone, Blade }
         struct Stash { counts: Map<String, f32> }
         script S {
             var kills: Map<Kind, f32> = [:];
             var by_entity: Map<Entity, bool> = [:];
             var stash = Stash(counts: ["cores": 1.0]);
             fn update(dt: f32) {
                 kills[Kind.Drone] += 1.0;
                 let n: f32 = kills.get(Kind.Blade, 0.0) + kills.length;
                 if kills.contains(Kind.Drone) && kills.remove(Kind.Drone) { }
                 let names: List<String> = stash.counts.keys();
                 let totals: List<f32> = stash.counts.values();
                 this.stash.counts["shards"] = 2.0;
                 by_entity[this.entity] = true;
                 kills.clear();
             }
         }"#,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_mistake_about_a_map_is_refused() {
    let found = messages(
        r#"script S {
             var wrong: Map<String> = [:];
             var list_key: Map<List<f32>, f32> = [:];
             fn update(dt: f32) {
                 let fixed = ["a": 1.0];
                 var m = ["a": 1.0, 2.0: 3.0, "b": "c"];
                 let x: f32 = m[1.0];
                 fixed["b"] = 2.0;
                 fixed.clear();
                 let k = m.keys;
                 let l = m.length();
                 let z = m.size;
                 let t: String = m.get("a", 0.0);
             }
         }"#,
    );
    let all = found.join("\n");
    for expected in [
        "`Map` takes a key type and a value type, as in `Map<String, f32>`",
        "a map is keyed by text, numbers, flags, enum variants or entities, not `List<f32>`",
        "a map holds one key type: this is `f32`, and the first is `String`",
        "a map holds one value type: this is `String`, and the first is `f32`",
        "expected `String`, found `f32`",
        "`fixed` is a `let`, so it cannot change -- declare it `var` if an entry can be set",
        "`fixed` is a `let`, so it cannot change -- declare it `var` if `clear` can change it",
        "`keys` on a map is a function -- call it: `.keys(...)`",
        "`length` is a property, not a function -- write `.length`",
        "`Map<String, f32>` has no member `size`",
        "cannot assign `f32` to `String`",
    ] {
        assert!(all.contains(expected), "missing {expected:?} in:\n{all}");
    }
}
