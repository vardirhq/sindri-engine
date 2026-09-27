//! Timers: started with `Timer(seconds)`, read through four properties, and
//! never written into.

use crate::{Environment, Type, analyze};

fn messages(source: &str) -> Vec<String> {
    analyze(source)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn a_timer_is_started_held_and_read() {
    let found = messages(
        "script Mine {
             var cooldown: Timer = Timer(0.0);
             var fuse = Timer(2.0);
             fn update(dt: f32) {
                 if !this.cooldown.done { return; }
                 this.cooldown = Timer(2.4);
                 let fraction: f32 = fuse.progress;
                 let left: f32 = fuse.left + fuse.duration;
             }
         }",
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_mistake_about_a_timer_is_refused() {
    let found = messages(
        "script Mine {
             var fuse = Timer(2.0);
             fn update(dt: f32) {
                 let a = Timer();
                 let b = Timer(\"soon\");
                 fuse.left = 1.0;
                 let c = fuse.done();
                 let d = fuse.remaining;
                 let e: f32 = fuse.done;
                 let f: f32 = fuse + 1.0;
             }
         }",
    );
    for expected in [
        "`Timer` takes the seconds it runs for, found 0 argument(s)",
        "expected `f32`, found `String`",
        "a timer's `left` cannot be set",
        "`done` is a property, not a function",
        "`Timer` has no member `remaining`; it has `done`, `left`, `duration`, `progress`",
        "cannot assign `bool` to `f32`",
    ] {
        assert!(
            found.iter().any(|message| message.contains(expected)),
            "missing {expected:?} in {found:?}"
        );
    }
    assert!(found.len() >= 7, "`fuse + 1.0` is refused too: {found:?}");
}

#[test]
fn a_binding_named_timer_shadows_the_constructor() {
    let mut environment = Environment::new();
    environment.add_value("Other", Type::F32);
    let found = crate::analyze_with_environment(
        "script T { fn f(Timer: f32) -> f32 { return Timer; } }",
        &environment,
    )
    .diagnostics;
    assert!(found.is_empty(), "{found:?}");
}
