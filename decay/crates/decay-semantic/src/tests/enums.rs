//! Enums: named variants in place of numbers, and a `match` that must cover
//! every one of them.

use crate::{Environment, analyze_with_environment};

fn messages(source: &str, environment: &Environment) -> Vec<String> {
    analyze_with_environment(source, environment)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn an_enum_is_held_compared_and_matched() {
    let found = messages(
        "enum Phase { Lobby, Countdown, Play }
         script Match {
             var phase = Phase.Lobby;
             let first: Phase = Phase.Lobby;
             fn update(dt: f32) {
                 if this.phase == Phase.Lobby { this.phase = Phase.Countdown; }
                 match this.phase {
                     Phase.Lobby => { }
                     Phase.Countdown | Phase.Play => { this.phase = Phase.Play; }
                 }
                 match first { Phase.Play => { } _ => { } }
             }
         }",
        &Environment::new(),
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn an_enum_from_the_host_is_known_without_a_declaration() {
    let mut environment = Environment::new();
    environment.add_enum("Kind", vec!["Grow".to_owned(), "Boost".to_owned()]);
    let found = messages(
        "script Power { var kind = Kind.Grow;
             fn update(dt: f32) { match kind { Kind.Grow => { } Kind.Boost => { } } } }",
        &environment,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_mistake_about_an_enum_is_refused() {
    let found = messages(
        "enum Phase { Lobby, Countdown, Play }
         enum Kind { Grow, Grow }
         enum Empty { }
         script Phase { }
         script Match {
             var phase = Phase.Lobby;
             var kind = Kind.Grow;
             fn update(dt: f32) {
                 let a = Phase.Lobbby;
                 let b: f32 = Phase.Play;
                 let c = Phase();
                 if phase == kind { }
                 if phase < Phase.Play { }
                 match phase { Phase.Lobby => { } }
                 match phase {
                     Phase.Lobby => { }
                     Phase.Lobby => { }
                     Kind.Grow => { }
                     _ => { }
                     Phase.Play => { }
                 }
                 match 3.0 { _ => { } }
             }
         }",
        &Environment::new(),
    );
    for expected in [
        "`Kind.Grow` is declared twice",
        "enum `Empty` has no variants",
        "`Phase` is already a name; an enum needs one of its own",
        "`Phase` has no variant `Lobbby`; it has `Lobby`, `Countdown`, `Play`",
        "cannot assign `Phase` to `f32`",
        "`Phase` is an enum: name a variant, as in `Phase.Lobby`",
        "does not say what happens for `Phase.Countdown`, `Phase.Play`",
        "`Phase.Lobby` is already matched above",
        "`Kind.Grow` is not a `Phase`",
        "this can never be reached",
        "`match` takes a value of an enum, found `f32`",
    ] {
        assert!(
            found.iter().any(|message| message.contains(expected)),
            "missing {expected:?} in {found:?}"
        );
    }
    // Comparing two enums, and ordering one, are both refused too.
    assert!(found.len() >= 13, "{found:?}");
}

#[test]
fn a_match_gives_a_value_where_one_goes() {
    let found = messages(
        r#"enum Phase { Lobby, Play, Over }
         script S {
             var phase: Phase = Phase.Lobby;
             var label: String = match Phase.Play { Phase.Play => "go", _ => "wait" };
             fn speed() -> f32 {
                 let x: f32 = match phase { Phase.Play => 2.0, Phase.Lobby | Phase.Over => 0.5 };
                 return x + match phase { _ => 1.0 };
             }
         }"#,
        &Environment::new(),
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_mistake_in_a_match_value_is_refused() {
    let found = messages(
        r#"enum Phase { Lobby, Play, Over }
         script S {
             fn f(p: Phase, n: f32) {
                 let missing = match p { Phase.Lobby => 1.0, Phase.Play => 2.0 };
                 let mixed = match p { Phase.Lobby => 1.0, _ => "two" };
                 let late = match p { _ => 1.0, Phase.Play => 2.0 };
                 let number = match n { _ => 1.0 };
                 let wrong: String = match p { _ => 1.0 };
             }
         }"#,
        &Environment::new(),
    );
    let all = found.join("\n");
    for expected in [
        "does not say what happens for `Phase.Over`",
        "every arm of a `match` gives the same type: the first gives `f32`, this one `String`",
        "this can never be reached",
        "`match` takes a value of an enum, found `f32`",
        "cannot assign `f32` to `String`",
    ] {
        assert!(all.contains(expected), "missing {expected:?} in:\n{all}");
    }
}

#[test]
fn a_match_value_must_close_its_arms_with_commas() {
    let found = messages(
        r"enum Phase { Lobby, Play }
         script S { fn f(p: Phase) -> f32 { return match p { Phase.Lobby => 1.0 Phase.Play => 2.0 }; } }",
        &Environment::new(),
    );
    assert!(
        found
            .iter()
            .any(|message| message.contains("an arm that gives a value ends with `,`")),
        "{found:?}"
    );
}
