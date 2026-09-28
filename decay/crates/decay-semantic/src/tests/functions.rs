//! Shared functions: one declaration outside any container, called by name
//! from every script, with no `this`.

use crate::{Environment, FunctionType, Type, analyze_with_environment};

fn messages(source: &str, environment: &Environment) -> Vec<String> {
    analyze_with_environment(source, environment)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn a_shared_function_is_called_by_name_from_any_script() {
    let found = messages(
        "fn half(size: f32) -> f32 { return size * 0.5; }
         fn quarter(size: f32) -> f32 { return half(half(size)); }
         script Enemy { let view_size: f32 = 11.0;
             fn update(dt: f32) { let edge: f32 = quarter(this.view_size); } }",
        &Environment::new(),
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn one_from_another_file_is_known_through_the_host() {
    let mut environment = Environment::new();
    environment.add_shared_function(
        "half",
        FunctionType {
            params: vec![Type::F32],
            return_type: Type::F32,
        },
    );
    let found = messages(
        "script Enemy { fn update(dt: f32) { let a: f32 = half(2.0); let b: bool = half(\"x\"); } }",
        &environment,
    );
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found
            .iter()
            .any(|m| m.contains("cannot assign `f32` to `bool`")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|m| m.contains("cannot assign `String` to `f32`")),
        "{found:?}"
    );
}

#[test]
fn a_script_s_own_function_shadows_a_shared_one() {
    let found = messages(
        "fn size() -> f32 { return 1.0; }
         script Enemy { fn size() -> bool { return true; } fn update(dt: f32) { let big: bool = size(); } }",
        &Environment::new(),
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_mistake_about_a_shared_function_is_refused() {
    let mut environment = Environment::new();
    environment.add_function(
        "sin",
        FunctionType {
            params: vec![Type::F32],
            return_type: Type::F32,
        },
    );
    environment.add_ambiguous_function("twice");
    let found = messages(
        "fn half(size: f32) -> f32 { return this.size * 0.5; }
         fn half(size: f32) -> f32 { return size; }
         fn sin(x: f32) -> f32 { return x; }
         fn Enemy() { }
         fn twice() { }
         script Enemy { fn update(dt: f32) { twice(); } }",
        &environment,
    );
    for expected in [
        "a shared function has no `this`",
        "duplicate declaration `half`",
        "`sin` is already a name",
        "`Enemy` is already a name",
        "`twice` is declared in more than one file",
    ] {
        assert!(
            found.iter().any(|message| message.contains(expected)),
            "missing {expected:?} in {found:?}"
        );
    }
}
