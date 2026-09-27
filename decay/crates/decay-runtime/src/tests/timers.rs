//! Timers at runtime: they read as started, and run down only when the host
//! says time has passed.

use crate::{EmptyHost, Runtime, RuntimeError, Value};

const MINE: &str = "script Mine {
    var fuse = Timer(2.0);
    var spent = Timer(-1.0);
    fn read() -> f32 { return fuse.left + fuse.progress * 10.0; }
    fn done() -> bool { return fuse.done && spent.done; }
    fn relight() { fuse = Timer(0.5); }
    fn local() -> f32 { let brief = Timer(1.0); return brief.left; }
}";

#[test]
fn a_timer_runs_down_only_when_the_host_advances_it() {
    let program = decay_ir::lower(MINE).program.expect("valid program");
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut mine = runtime.instantiate("Mine").expect("instance");

    assert_eq!(
        runtime.call_instance(&mut mine, "read", vec![]),
        Ok(Value::Number(2.0))
    );
    mine.advance_timers(0.5);
    // 1.5 left, a quarter of the way through.
    assert_eq!(
        runtime.call_instance(&mut mine, "read", vec![]),
        Ok(Value::Number(1.5 + 2.5))
    );
    assert_eq!(
        runtime.call_instance(&mut mine, "done", vec![]),
        Ok(Value::Bool(false))
    );
    mine.advance_timers(10.0);
    assert_eq!(
        mine.field("fuse"),
        Some(&Value::Timer {
            left: 0.0,
            duration: 2.0
        })
    );
    assert_eq!(
        runtime.call_instance(&mut mine, "done", vec![]),
        Ok(Value::Bool(true))
    );
    runtime
        .call_instance(&mut mine, "relight", vec![])
        .expect("relights");
    assert_eq!(
        runtime.call_instance(&mut mine, "read", vec![]),
        Ok(Value::Number(0.5))
    );
    assert_eq!(
        runtime.call_instance(&mut mine, "local", vec![]),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn going_backwards_or_by_nothing_changes_nothing() {
    let program = decay_ir::lower(MINE).program.expect("valid program");
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut mine = runtime.instantiate("Mine").expect("instance");
    mine.advance_timers(-3.0);
    mine.advance_timers(f64::NAN);
    assert_eq!(
        runtime.call_instance(&mut mine, "read", vec![]),
        Ok(Value::Number(2.0))
    );
    assert!(matches!(
        mine.field("spent"),
        Some(Value::Timer { left, duration }) if *left == 0.0 && *duration == 0.0
    ));
    let _ = RuntimeError::NotATimer(String::new());
}
