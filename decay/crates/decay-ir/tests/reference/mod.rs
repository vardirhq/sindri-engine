//! What the reference-checking tests share: a host, and "does this compile".
//!
//! Each file using it calls only some of it.
#![allow(dead_code)]

use decay_ir::lower_with_environment;
use decay_semantic::{Environment, FunctionType, Type};

/// A host offering the one function the reference's example calls.
pub fn environment() -> Environment {
    let mut environment = Environment::new();
    environment.add_function(
        "sin",
        FunctionType {
            params: vec![Type::F32],
            return_type: Type::F32,
        },
    );
    environment
}

pub fn compiles(source: &str) -> bool {
    lower_with_environment(source, &environment())
        .analysis
        .diagnostics
        .is_empty()
}

#[track_caller]
pub fn accepted(what: &str, source: &str) {
    assert!(compiles(source), "LANGUAGE.md says {what} is accepted");
}

#[track_caller]
pub fn rejected(what: &str, source: &str) {
    assert!(!compiles(source), "LANGUAGE.md says {what} is rejected");
}
