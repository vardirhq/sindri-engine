//! `match` at runtime: the first arm whose pattern the subject is runs, and
//! nothing else does.

use crate::{EmptyHost, Runtime, Value};

const SOURCE: &str = "enum Phase { Lobby, Countdown, Play }
script Board {
    var phase = Phase.Lobby;
    fn advance() {
        match phase {
            Phase.Lobby => { phase = Phase.Countdown; }
            Phase.Countdown => { phase = Phase.Play; }
            Phase.Play => { phase = Phase.Lobby; }
        }
    }
    fn score(of: Phase) -> f32 {
        var total: f32 = 0.0;
        match of {
            Phase.Lobby | Phase.Countdown => { total = 1.0; }
            _ => { total = 2.0; }
        }
        return total;
    }
    fn first_playing() -> f32 {
        var seen: f32 = 0.0;
        while seen < 10.0 {
            seen += 1.0;
            match phase { Phase.Play => { break; } _ => { advance(); } }
        }
        return seen;
    }
    fn none() -> f32 {
        let nothing: Phase = null;
        var ran: f32 = 0.0;
        match nothing { Phase.Lobby => { ran = 1.0; } Phase.Countdown | Phase.Play => { ran = 2.0; } }
        return ran;
    }
}";

#[test]
fn a_match_runs_the_one_arm_that_fits() {
    let program = decay_ir::lower(SOURCE).program.expect("valid program");
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut board = runtime.instantiate("Board").expect("instance");

    assert_eq!(
        board.field("phase"),
        Some(&Value::Variant("Phase.Lobby".into()))
    );
    runtime
        .call_instance(&mut board, "advance", vec![])
        .expect("advances");
    assert_eq!(
        board.field("phase"),
        Some(&Value::Variant("Phase.Countdown".into()))
    );
    runtime
        .call_instance(&mut board, "advance", vec![])
        .expect("advances");
    runtime
        .call_instance(&mut board, "advance", vec![])
        .expect("advances");
    assert_eq!(
        board.field("phase"),
        Some(&Value::Variant("Phase.Lobby".into()))
    );

    let score =
        |runtime: &mut Runtime<'_, EmptyHost>, board: &mut crate::ScriptInstance, of: &str| {
            runtime.call_instance(board, "score", vec![Value::Variant(of.into())])
        };
    assert_eq!(
        score(&mut runtime, &mut board, "Phase.Lobby"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        score(&mut runtime, &mut board, "Phase.Countdown"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        score(&mut runtime, &mut board, "Phase.Play"),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn break_leaves_a_loop_from_inside_a_match_and_null_runs_no_arm() {
    let program = decay_ir::lower(SOURCE).program.expect("valid program");
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut board = runtime.instantiate("Board").expect("instance");
    // Lobby, then Countdown, then Play is seen on the third turn.
    assert_eq!(
        runtime.call_instance(&mut board, "first_playing", vec![]),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        runtime.call_instance(&mut board, "none", vec![]),
        Ok(Value::Number(0.0))
    );
}
