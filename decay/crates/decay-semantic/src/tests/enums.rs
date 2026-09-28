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
