//! `match` written where a value goes: the first arm whose pattern the
//! subject is gives the value.

use crate::{EmptyHost, Runtime, Value};

const SOURCE: &str = r#"enum Phase { Lobby, Countdown, Play, Over }
enum Size { Small, Large }
script Hud {
    var phase: Phase = Phase.Lobby;
    var banner: String = match Phase.Over { Phase.Over => "done", _ => "busy" };
    fn label(p: Phase) -> String {
        return match p {
            Phase.Lobby => "Waiting",
            Phase.Countdown | Phase.Play => "Go",
            Phase.Over => "Again?",
        };
    }
    fn speed(p: Phase) -> f32 {
        let base = match p { Phase.Play => 2.0, _ => 0.5 };
        return base * 10.0;
    }
    fn nested(p: Phase, s: Size) -> f32 {
        return match p {
            Phase.Play => match s { Size.Small => 1.0, Size.Large => 2.0 },
            _ => match s { Size.Small => 3.0, Size.Large => 4.0 },
        };
    }
    fn bigger(a: f32, b: f32) -> f32 { if a > b { return a; } return b; }
    fn argument(p: Phase) -> f32 { return bigger(match p { Phase.Over => 7.0, _ => 1.0 }, 3.0); }
    fn banner_now() -> String { return banner; }
    fn in_a_loop() -> f32 {
        var total = 0.0;
        for i in 0..3 {
            total += match phase { Phase.Lobby => i, _ => 100.0 };
        }
        return total;
    }
}"#;

fn call(function: &str, args: Vec<Value>) -> Value {
    let lowered = decay_ir::lower(SOURCE);
    let program = lowered
        .program
        .unwrap_or_else(|| panic!("{:?}", lowered.analysis.diagnostics));
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut hud = runtime.instantiate("Hud").expect("instance");
    runtime
        .call_instance(&mut hud, function, args)
        .expect("runs")
}

fn phase(variant: &str) -> Value {
    Value::Variant(format!("Phase.{variant}").into())
}

fn size(variant: &str) -> Value {
    Value::Variant(format!("Size.{variant}").into())
}

#[test]
fn each_arm_gives_its_value() {
    let label = |p| call("label", vec![phase(p)]);
    assert_eq!(label("Lobby"), Value::String("Waiting".to_owned()));
    assert_eq!(label("Countdown"), Value::String("Go".to_owned()));
    assert_eq!(label("Play"), Value::String("Go".to_owned()));
    assert_eq!(label("Over"), Value::String("Again?".to_owned()));
    assert_eq!(call("speed", vec![phase("Play")]), Value::Number(20.0));
    assert_eq!(call("speed", vec![phase("Over")]), Value::Number(5.0));
}

#[test]
fn a_match_value_goes_anywhere_a_value_does() {
    assert_eq!(
        call("nested", vec![phase("Play"), size("Large")]),
        Value::Number(2.0)
    );
    assert_eq!(
        call("nested", vec![phase("Lobby"), size("Small")]),
        Value::Number(3.0)
    );
    assert_eq!(call("argument", vec![phase("Over")]), Value::Number(7.0));
    assert_eq!(call("argument", vec![phase("Lobby")]), Value::Number(3.0));
    assert_eq!(call("banner_now", vec![]), Value::String("done".to_owned()));
    // Held in a loop, each pass its own: 0 + 1 + 2.
    assert_eq!(call("in_a_loop", vec![]), Value::Number(3.0));
}
