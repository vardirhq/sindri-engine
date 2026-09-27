//! Shared state: declared once with a type and a starting value, then
//! reached by name from anywhere, checked.

use crate::{Environment, FunctionType, HostType, StateField, Type, analyze_with_environment};

fn messages(source: &str, environment: &Environment) -> Vec<String> {
    analyze_with_environment(source, environment)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

/// A host with a `Game` namespace offering `get`, as Sindri's does.
fn with_game() -> Environment {
    let mut environment = Environment::new();
    environment.add_value("Game", Type::Named("Game".to_owned()));
    environment.add_type(
        "Game",
        HostType::new().with_function(
            "get",
            FunctionType {
                params: vec![Type::String, Type::F32],
                return_type: Type::F32,
            },
        ),
    );
    environment
}

#[test]
fn a_declared_state_is_read_and_written_by_name() {
    let found = messages(
        "state Game { var score: f32 = 0.0; var won: bool = false; let lives: f32 = 3.0; }
         script Ball {
             fn update(dt: f32) {
                 Game.score += 1.0;
                 if Game.score > Game.lives { Game.won = true; }
                 let old = Game.get(\"score\", 0.0);
             }
         }",
        &with_game(),
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_state_of_its_own_name_needs_no_host_namespace() {
    let found = messages(
        "state Tuning { let gravity = 30.0; var paused = false; }
         script Body { fn update(dt: f32) { if !Tuning.paused { let g: f32 = Tuning.gravity; } } }",
        &Environment::new(),
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_state_from_the_host_is_known_without_a_declaration() {
    let mut environment = with_game();
    environment.add_state_field(
        "Game",
        "phase",
        StateField {
            ty: Type::F32,
            mutable: true,
        },
    );
    let found = messages(
        "script Ball { fn update(dt: f32) { Game.phase = 2.0; Game.phse = 1.0; } }",
        &environment,
    );
    assert_eq!(
        found,
        vec!["`Game` has no member `phse`".to_owned()],
        "{found:?}"
    );
}

#[test]
fn every_mistake_about_state_is_refused() {
    let found = messages(
        "state Game {
             var score: f32 = 0.0;
             var score: f32 = 1.0;
             var name: String = \"x\";
             var half: f32 = 1.0 / 2.0;
             var flag: bool = 1.0;
             var get: f32 = 0.0;
             let lives: f32 = 3.0;
         }
         state Ball { var x: f32 = 0.0; }
         script Ball {
             fn update(dt: f32) {
                 Game.score = true;
                 Game.lives = 2.0;
                 Game.scroe = 1.0;
             }
         }",
        &with_game(),
    );
    for expected in [
        "duplicate state field `Game.score`",
        "`Game.name` is a `String`; a state holds `f32` and `bool` for now",
        "`Game.half` needs a starting value written as a number",
        "`Game.flag` is a `bool` but starts as a `f32`",
        "`Game` already has a `get`",
        "`Ball` is already a name; a state needs one of its own",
        "cannot assign `bool` to `f32`",
        "`Game.lives` is a `let` and cannot be changed",
        "`Game` has no member `scroe`",
    ] {
        assert!(
            found.iter().any(|message| message.contains(expected)),
            "missing {expected:?} in {found:?}"
        );
    }
}

#[test]
fn a_state_field_declared_twice_across_files_is_refused_where_declared_and_used() {
    let mut environment = with_game();
    environment.add_ambiguous_state_field("Game", "score");
    let found = messages(
        "state Game { var score: f32 = 0.0; }
         script Ball { fn update(dt: f32) { Game.score += 1.0; } }",
        &environment,
    );
    // Where it is declared, and where it is used: the declaring file may be
    // one nothing compiles.
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found
            .iter()
            .all(|message| message.contains("declared more than once")),
        "{found:?}"
    );
}

#[test]
fn a_local_shadows_a_state() {
    let found = messages(
        "state Tuning { let gravity: f32 = 30.0; }
         script Body { fn update(dt: f32) { let Tuning = 2.0; let twice = Tuning * 2.0; } }",
        &Environment::new(),
    );
    assert!(found.is_empty(), "{found:?}");
}
