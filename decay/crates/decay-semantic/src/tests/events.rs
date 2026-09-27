//! Events: declared once, emitted with what they carry, handled anywhere.

use crate::{Environment, Type, analyze, analyze_with_environment};

fn messages(source: &str, environment: &Environment) -> Vec<String> {
    analyze_with_environment(source, environment)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn an_event_declared_in_the_file_is_emitted_and_handled() {
    let analysis = analyze(
        "event GoalScored(team: f32, own_goal: bool);
         script Ball {
             fn update(dt: f32) { GoalScored.emit(1.0, false); }
         }
         script Scoreboard {
             var blue: f32 = 0.0;
             on GoalScored(team, own_goal) {
                 if !own_goal { blue += team; }
             }
         }",
    );
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn a_handler_s_unwritten_parameter_takes_the_event_s_type() {
    let found = messages(
        "event Named(label: String);
         script Sign { var total: f32 = 0.0; on Named(label) { total += label; } }",
        &Environment::new(),
    );
    assert!(
        found.iter().any(|message| message.contains("String")),
        "{found:?}"
    );
}

#[test]
fn an_event_from_the_host_is_known_without_a_declaration() {
    let mut environment = Environment::new();
    environment.add_event("Wave", vec![Type::F32]);
    let found = messages(
        "script Director { fn update(dt: f32) { Wave.emit(3.0); } }
         script Hud { on Wave(number: f32) { } }",
        &environment,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_mistake_about_an_event_is_refused() {
    let found = messages(
        "event Hit(amount: f32);
         event Hit(amount: f32);
         event Loose(amount);
         script Bullet {
             fn update(dt: f32) {
                 Hit.emit(\"one\");
                 Hit.emit();
                 Hit.emt(1.0);
                 Missed.emit(1.0);
             }
             on Hit(amount: bool) { }
             on Hit(amount: f32, extra: f32) { }
             on Missed() { }
         }",
        &Environment::new(),
    );
    for expected in [
        "duplicate declaration `Hit`",
        "event parameter `amount` needs a type",
        "cannot assign `String` to `f32`",
        "expected 1 argument(s), found 0",
        "`event Hit` has no member `emt`",
        "unknown name `Missed`",
        "`Hit` carries `f32` here, not `bool`",
        "`Hit` carries 1 value(s), and this handler takes 2",
        "unknown event `Missed`",
    ] {
        assert!(
            found.iter().any(|message| message.contains(expected)),
            "missing {expected:?} in {found:?}"
        );
    }
}

#[test]
fn an_event_declared_twice_across_files_cannot_be_used() {
    let mut environment = Environment::new();
    environment.add_ambiguous_event("Hit");
    let found = messages(
        "script Bullet { fn update(dt: f32) { Hit.emit(1.0); } on Hit(amount: f32) { } }",
        &environment,
    );
    assert_eq!(
        found
            .iter()
            .filter(|message| message.contains("declared in more than one file"))
            .count(),
        2,
        "{found:?}"
    );
}

#[test]
fn an_event_cannot_take_a_name_already_in_use() {
    let mut environment = Environment::new();
    environment.add_value("World", Type::Named("World".to_owned()));
    let found = messages(
        "event World(); event Bullet(); script Bullet { }",
        &environment,
    );
    assert_eq!(
        found
            .iter()
            .filter(|message| message.contains("an event needs one of its own"))
            .count(),
        2,
        "{found:?}"
    );
}

#[test]
fn a_handler_is_not_a_function_a_script_can_call() {
    let found = messages(
        "event Tick(); script Clock { fn update(dt: f32) { Tick(); } on Tick() { } }",
        &Environment::new(),
    );
    assert!(
        found
            .iter()
            .any(|message| message.contains("send one with `Tick.emit(...)`")),
        "{found:?}"
    );
}

#[test]
fn a_local_shadows_an_event() {
    let found = messages(
        "event Hit(amount: f32);
         script Bullet { fn update(dt: f32) { let Hit = 2.0; let twice = Hit * 2.0; } }",
        &Environment::new(),
    );
    assert!(found.is_empty(), "{found:?}");
}
