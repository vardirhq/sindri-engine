//! Shared functions at runtime: called by name, from a script or from each
//! other, and linked in from another file's program.

use crate::{EmptyHost, Runtime, Value};

#[test]
fn a_shared_function_runs_without_a_this() {
    let program = decay_ir::lower(
        "fn half(size: f32) -> f32 { return size * 0.5; }
         fn quarter(size: f32) -> f32 { return half(half(size)); }
         script Enemy { let view_size: f32 = 12.0; fn edge() -> f32 { return quarter(view_size); } }",
    )
    .program
    .expect("valid program");
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut enemy = runtime.instantiate("Enemy").expect("instance");
    assert_eq!(
        runtime.call_instance(&mut enemy, "edge", vec![]),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn a_function_from_another_file_runs_once_linked() {
    let helpers = decay_ir::lower("fn half(size: f32) -> f32 { return size * 0.5; }")
        .program
        .expect("valid helpers");
    let mut environment = decay_semantic::Environment::new();
    environment.add_shared_function(
        "half",
        decay_semantic::FunctionType {
            params: vec![decay_semantic::Type::F32],
            return_type: decay_semantic::Type::F32,
        },
    );
    let mut program = decay_ir::lower_with_environment(
        "script Enemy { fn edge() -> f32 { return half(8.0); } }",
        &environment,
    )
    .program
    .expect("valid script");
    program.link(helpers.shared().expect("helpers").functions.clone());
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut enemy = runtime.instantiate("Enemy").expect("instance");
    assert_eq!(
        runtime.call_instance(&mut enemy, "edge", vec![]),
        Ok(Value::Number(4.0))
    );
}
