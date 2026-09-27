//! Events at the host boundary: an emit is a call the host answers, and a
//! handler is a function the host runs by its handler name.

use decay_syntax::handler_name;

use crate::{Host, Path, Runtime, RuntimeError, Value};

/// Remembers every emit it was asked to perform.
#[derive(Default)]
struct EmittingHost {
    emitted: Vec<(String, Vec<Value>)>,
}

impl Host for EmittingHost {
    fn load(&mut self, _subject: Option<u64>, _path: &Path) -> Result<Option<Value>, RuntimeError> {
        Ok(None)
    }
    fn store(
        &mut self,
        _subject: Option<u64>,
        _path: &Path,
        _value: Value,
    ) -> Result<bool, RuntimeError> {
        Ok(false)
    }
    fn call(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        args: &[Value],
    ) -> Result<Option<Value>, RuntimeError> {
        assert_eq!(subject, None, "an event is not sent to anything");
        self.emitted.push((path.dotted(), args.to_vec()));
        Ok(Some(Value::Unit))
    }
}

#[test]
fn an_emit_reaches_the_host_and_a_handler_runs_by_its_name() {
    let lowered = decay_ir::lower(
        "event Scored(points: f32);
         script Ball { fn update(dt: f32) { Scored.emit(3.0); } }
         script Board {
             var total: f32 = 0.0;
             on Scored(points) { total += points; }
         }",
    );
    assert!(
        lowered.analysis.diagnostics.is_empty(),
        "{:?}",
        lowered.analysis.diagnostics
    );
    let program = lowered.program.expect("valid program");
    let mut runtime = Runtime::new(&program, EmittingHost::default());

    let mut ball = runtime.instantiate("Ball").expect("ball");
    runtime
        .call_instance(&mut ball, "update", vec![Value::Number(0.0)])
        .expect("emits");

    let mut board = runtime.instantiate("Board").expect("board");
    runtime
        .call_instance(
            &mut board,
            &handler_name("Scored"),
            vec![Value::Number(3.0)],
        )
        .expect("handles");
    assert_eq!(board.field("total"), Some(&Value::Number(3.0)));
    assert_eq!(
        runtime.into_host().emitted,
        vec![("Scored.emit".to_owned(), vec![Value::Number(3.0)])]
    );
}
