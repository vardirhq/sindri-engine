//! Does the language reference describe the language?
//!
//! `decay/LANGUAGE.md` makes specific, checkable claims — that `else if` is a
//! parse error, that there is no truthiness, that every number is a float.
//! Documentation of a language that is still moving goes stale silently and is
//! then believed, which is worse than having none. So the claims are assertions.
//!
//! When one of these fails, the language changed and the reference is now
//! wrong: fix the document in the same commit as the change.

use decay_ir::lower_with_environment;
use decay_semantic::{Environment, FunctionType, HostType, Type};

/// A host offering the one function the reference's example calls.
fn environment() -> Environment {
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

fn compiles(source: &str) -> bool {
    lower_with_environment(source, &environment())
        .analysis
        .diagnostics
        .is_empty()
}

#[track_caller]
fn accepted(what: &str, source: &str) {
    assert!(compiles(source), "LANGUAGE.md says {what} is accepted");
}

#[track_caller]
fn rejected(what: &str, source: &str) {
    assert!(!compiles(source), "LANGUAGE.md says {what} is rejected");
}

/// "What does not exist". An absence nothing checks is how a document starts
/// lying.
#[test]
fn nothing_the_reference_calls_absent_compiles() {
    // `for` exists, and walks a collection; what stays absent is the range it
    // used to be paired with in this list.
    rejected("ranges", "script T { fn f() { for i in 0..3 { } } }");
    rejected("`loop`", "script T { fn f() { loop { } } }");
    rejected("`match`", "script T { fn f(a: f32) { match a { } } }");
    rejected(
        "array literals",
        "script T { fn f() { let a: f32 = [1.0]; } }",
    );
    // Indexing exists, and only on a collection. Indexing something that holds
    // one value is what stays refused.
    rejected(
        "indexing a value that holds one thing",
        "script T { fn f(a: f32) { let b: f32 = a[0.0]; } }",
    );
    rejected("block comments", "script T { /* nope */ }");
    rejected(
        "exponent literals",
        "script T { fn f() { let a: f32 = 1e6; } }",
    );
    rejected("hex literals", "script T { fn f() { let a: f32 = 0xff; } }");
    rejected(
        "digit separators",
        "script T { fn f() { let a: f32 = 1_000; } }",
    );
    rejected("`import`", "import other; script T { }");
    rejected("attributes on functions", "script T { @export fn f() {} }");
}

/// "Surprising behaviour": the claims a reader is most likely to guess wrong,
/// which makes them the ones most worth holding still.
#[test]
fn the_surprises_are_still_surprising() {
    rejected(
        "`if` without a bool condition, because there is no truthiness",
        "script T { fn f() { if 1.0 { } } }",
    );
    rejected(
        "`+` on strings, because it does not concatenate",
        r#"script T { fn f() { let s: String = "a" + "b"; } }"#,
    );
    rejected(
        "a body binding reusing a parameter name, because they share one scope",
        "script T { fn f(x: f32) { let x: f32 = 1.0; } }",
    );
    rejected(
        "assignment to a `let` binding",
        "script T { fn f() { let x: f32 = 1.0; x = 2.0; } }",
    );
    accepted(
        "`this.field` and the bare name naming the same field",
        "script T { var a: f32 = 1.0; fn f() { this.a = 2.0; a = 3.0; } }",
    );
    accepted(
        "a field reading a field declared above it",
        "script T { let a: f32 = 2.0; let b: f32 = a; }",
    );
    rejected(
        "a field reading one declared below it",
        "script T { let a: f32 = b; let b: f32 = 2.0; }",
    );
    rejected("a field reading itself", "script T { let a: f32 = a; }");
    accepted(
        "any path into a type the host has not described",
        "script T { fn f() { this.transfrom.position.x = 1.0; } }",
    );
    rejected(
        "reaching for a method on a container, because there are none",
        "script T { fn helper() -> f32 { return 1.0; } fn f() { this.helper(); } }",
    );
}

/// Loops, and the statements that only mean anything inside one.
#[test]
fn the_control_flow_the_reference_describes_compiles() {
    accepted(
        "`while` with a `bool` condition",
        "script T { fn f(go: bool) { while go { } } }",
    );
    rejected(
        "a `while` condition that is not `bool`, because there is no truthiness",
        "script T { fn f(n: f32) { while n { } } }",
    );
    accepted(
        "`break` and `continue` inside a loop",
        "script T { fn f(go: bool) { while go { if go { continue; } break; } } }",
    );
    rejected("`break` outside a loop", "script T { fn f() { break; } }");
    rejected(
        "`continue` outside a loop",
        "script T { fn f() { continue; } }",
    );
    accepted(
        "a chained `else if`",
        "script T { fn f(a: bool) { if a { } else if a { } else { } } }",
    );
    accepted(
        "`%` and `%=` as arithmetic",
        "script T { fn f() { var a: f32 = 7.0 % 2.0; a %= 3.0; } }",
    );
    rejected(
        "`%` on something that is not a number",
        "script T { fn f(a: bool) { let b: f32 = a % 2.0; } }",
    );
}

/// The other half of the member rule: once a host describes a type, its members
/// are checked. Both halves matter — checking is the point, and staying
/// permissive about the undescribed is what makes describing a host gradual.
#[test]
fn a_described_type_is_checked_and_an_undescribed_one_is_not() {
    let mut described = Environment::new();
    described.add_type(
        "Vec3",
        HostType::new()
            .with_value("x", Type::F32)
            .with_value("y", Type::F32),
    );
    described.add_this_value("position", Type::Named("Vec3".to_owned()));
    // Named but never described, which is the permissive case.
    described.add_this_value("body", Type::Named("RigidBody".to_owned()));

    let check = |source: &str| {
        lower_with_environment(source, &described)
            .analysis
            .diagnostics
            .is_empty()
    };

    assert!(
        check("script T { fn f() { this.position.x = 1.0; } }"),
        "a member the host described is accepted"
    );
    assert!(
        !check("script T { fn f() { this.position.z = 1.0; } }"),
        "a member it did not describe on a type it did is refused"
    );
    assert!(
        !check("script T { fn f() { this.anything = 1.0; } }"),
        "and once `this` is described, a member it does not have is refused too"
    );
    assert!(
        check("script T { fn f() { this.body.anything.at.all = 1.0; } }"),
        "but a named type it never described stays permissive"
    );
}

/// Parentheses around a condition are allowed and do nothing, which is pinned
/// because the reference used to imply they were an error.
#[test]
fn parentheses_around_a_condition_are_allowed_and_pointless() {
    accepted(
        "a parenthesised condition",
        "script T { fn f(a: bool) { if (a) { } } }",
    );
    accepted(
        "an unparenthesised condition",
        "script T { fn f(a: bool) { if a { } } }",
    );
}

/// The reference's complete example is its claim about what a real script looks
/// like, so it has to be one.
#[test]
fn the_documented_example_compiles() {
    accepted(
        "the reference's complete example",
        r#"
        script PlayerController {
            @export
            let speed: f32 = 6.0;

            @export
            let label: String = "player";

            var elapsed: f32 = 0.0;
            var airborne: bool = false;

            fn start() {
                elapsed = 0.0;
            }

            fn update(dt: f32) {
                elapsed += dt;
                let offset: f32 = wave(elapsed) * speed;
                this.transform.position.x = offset;
                if offset > 0.0 {
                    airborne = true;
                } else {
                    airborne = false;
                }
            }

            fn wave(seconds: f32) -> f32 {
                return sin(seconds);
            }
        }
        "#,
    );
}

/// "Host references". A value of a named host type can be held, compared, and
/// reached through — the example in the reference, compiled.
#[test]
fn a_host_reference_can_be_held_and_compared() {
    let mut environment = Environment::new();
    let vector = HostType::new().with_value("x", Type::F32);
    let transform = HostType::new().with_value("position", Type::Named("Vec3".to_owned()));
    let thing = HostType::new().with_value("transform", Type::Named("Transform".to_owned()));
    let world = HostType::new().with_function(
        "find",
        FunctionType {
            params: vec![Type::String],
            return_type: Type::Named("Thing".to_owned()),
        },
    );
    environment.add_type("Vec3", vector);
    environment.add_type("Transform", transform);
    environment.add_type("Thing", thing);
    environment.add_type("World", world);
    environment.add_value("World", Type::Named("World".to_owned()));
    environment.add_this_value("thing", Type::Named("Thing".to_owned()));

    let compiles = |source: &str| {
        lower_with_environment(source, &environment)
            .analysis
            .diagnostics
            .is_empty()
    };

    assert!(
        compiles(
            r#"script T { fn f() {
                let target = World.find("Player");
                if target != null && target != this.thing {
                    target.transform.position.x = 0.0;
                }
            } }"#
        ),
        "LANGUAGE.md says a host reference can be held, compared, and reached through"
    );
    assert!(
        !compiles(
            r#"script T { fn f() {
                let target = World.find("Player");
                target.transfrom.position.x = 0.0;
            } }"#
        ),
        "LANGUAGE.md says members of a described type are checked, through a reference too"
    );
    assert!(
        !compiles(
            r#"script T { fn f() {
                let target = World.find("Player");
                let doubled = target * 2.0;
            } }"#
        ),
        "LANGUAGE.md says there is no arithmetic on a reference"
    );
}

#[test]
fn vectors_do_what_the_reference_says() {
    accepted(
        "vector arithmetic, components and methods",
        "script T { fn f() -> f32 {
            var v = Vec2(3.0, 4.0) + Vec2(1.0, 1.0) * 2.0 - -Vec2(0.0, 1.0) / 2.0;
            v.x += 1.0;
            v += Vec2(1.0, 1.0);
            let w = Vec3(1.0, 2.0, 3.0);
            return v.length + v.normalized.x + v.dot(v) + v.distance(v) + v.lerp(v, 0.5).y + w.z;
        } }",
    );
    accepted(
        "a vector field with no initializer",
        "script T { var v: Vec3; }",
    );
    rejected(
        "a Vec2 mixed with a Vec3",
        "script T { fn f() { let a = Vec2(1.0, 1.0) + Vec3(1.0, 1.0, 1.0); } }",
    );
    rejected(
        "a vector times a vector",
        "script T { fn f() { let a = Vec2(1.0, 1.0) * Vec2(1.0, 1.0); } }",
    );
    rejected(
        "a number divided by a vector",
        "script T { fn f() { let a = 1.0 / Vec2(1.0, 1.0); } }",
    );
    rejected(
        "`length` called",
        "script T { fn f() { let a = Vec2(1.0, 1.0).length(); } }",
    );
    rejected(
        "a component of a `let`",
        "script T { fn f() { let a = Vec2(1.0, 1.0); a.x = 2.0; } }",
    );
    rejected(
        "a component of a temporary",
        "script T { fn f() { (Vec2(1.0, 1.0) + Vec2(1.0, 1.0)).x = 2.0; } }",
    );
    rejected(
        "`z` on a Vec2",
        "script T { fn f() { let a = Vec2(1.0, 1.0).z; } }",
    );
}

/// "Events", and the two words that are special only in their place.
#[test]
fn events_do_what_the_reference_says() {
    accepted(
        "the events example",
        "event GoalScored(team: f32);
         script Ball { fn update(dt: f32) { GoalScored.emit(1.0); } }
         script Match {
             var blue: f32 = 0.0;
             on GoalScored(team) { if team == 1.0 { blue += 1.0; } }
         }",
    );
    accepted(
        "`event` and `on` as ordinary names elsewhere",
        "script T { fn f() { let event = 1.0; let on = event; } }",
    );
    rejected(
        "an event parameter without a type",
        "event Hit(amount); script T { }",
    );
    rejected(
        "calling an event as a function",
        "event Hit(); script T { fn f() { Hit(); } }",
    );
    rejected(
        "a handler of the wrong type",
        "event Hit(amount: f32); script T { on Hit(amount: bool) { } }",
    );
    rejected(
        "a handler that returns something",
        "event Hit(); script T { on Hit() -> f32 { return 1.0; } }",
    );
    rejected(
        "an attribute on a handler",
        "event Hit(); script T { @export on Hit() { } }",
    );
    rejected(
        "an event with a container's name",
        "event T(); script T { }",
    );
}

/// "State".
#[test]
fn state_does_what_the_reference_says() {
    accepted(
        "the state example",
        "state Tuning { let gravity: f32 = 30.0; var paused = false; }
         script Body {
             fn update(dt: f32) {
                 if !Tuning.paused { this.transform.position.y -= Tuning.gravity * dt; }
             }
         }",
    );
    accepted(
        "`state` as an ordinary name elsewhere",
        "script T { fn f() { let state = 1.0; } }",
    );
    rejected(
        "a state field computed rather than written",
        "state S { var x: f32 = 1.0 + 1.0; } script T { }",
    );
    rejected(
        "a state field that is not a number or a flag",
        "state S { var x: String = \"a\"; } script T { }",
    );
    rejected(
        "writing a `let` state field",
        "state S { let x: f32 = 1.0; } script T { fn f() { S.x = 2.0; } }",
    );
    rejected(
        "a misspelt state field",
        "state S { var x: f32 = 1.0; } script T { fn f() { S.y = 2.0; } }",
    );
}

/// "Timers".
#[test]
fn timers_do_what_the_reference_says() {
    accepted(
        "the timers example",
        "script Mine {
             var cooldown = Timer(0.0);
             fn update(dt: f32) {
                 if !cooldown.done { return; }
                 cooldown = Timer(2.4);
             }
         }",
    );
    accepted(
        "reading every property",
        "script T { var t = Timer(1.0);
             fn f() -> f32 { if t.done { return 0.0; } return t.left + t.duration + t.progress; } }",
    );
    rejected(
        "writing into a timer",
        "script T { var t = Timer(1.0); fn f() { t.left = 0.0; } }",
    );
    rejected(
        "arithmetic on a timer",
        "script T { var t = Timer(1.0); fn f() -> f32 { return t + 1.0; } }",
    );
    rejected(
        "a property called as a function",
        "script T { var t = Timer(1.0); fn f() -> bool { return t.done(); } }",
    );
}
