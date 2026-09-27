//! A path through a reference: the host is told which entity it is about,
//! whether the reference is held in a local or in a field.

use crate::{Host, Path, Runtime, RuntimeError, Value};

/// Hands out entity 7, and records what is asked about it.
#[derive(Default)]
struct ReferenceHost {
    asked: Vec<(Option<u64>, String)>,
}

impl Host for ReferenceHost {
    fn load(&mut self, subject: Option<u64>, path: &Path) -> Result<Option<Value>, RuntimeError> {
        self.asked.push((subject, path.dotted()));
        Ok(Some(Value::Number(1.0)))
    }
    fn store(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        _value: Value,
    ) -> Result<bool, RuntimeError> {
        self.asked.push((subject, path.dotted()));
        Ok(true)
    }
    fn call(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        _args: &[Value],
    ) -> Result<Option<Value>, RuntimeError> {
        if path.dotted() == "World.find" {
            return Ok(Some(Value::Reference(7)));
        }
        self.asked.push((subject, path.dotted()));
        Ok(Some(Value::Unit))
    }
}

#[test]
fn a_field_holding_a_reference_is_a_subject_through_this_too() {
    let mut environment = decay_semantic::Environment::new();
    environment.add_value("World", decay_semantic::Type::Named("World".to_owned()));
    let lowered = decay_ir::lower_with_environment(
        r#"script Kicker {
            var ball: Entity = null;
            fn update(dt: f32) {
                this.ball = World.find("Ball");
                this.ball.kick(1.0);
                ball.kick(2.0);
                this.ball.transform.x = this.ball.transform.y;
            }
        }"#,
        &environment,
    );
    let program = lowered
        .program
        .unwrap_or_else(|| panic!("{:?}", lowered.analysis.diagnostics));
    let mut runtime = Runtime::new(&program, ReferenceHost::default());
    let mut kicker = runtime.instantiate("Kicker").expect("kicker");
    runtime
        .call_instance(&mut kicker, "update", vec![Value::Number(0.0)])
        .expect("runs");
    assert_eq!(
        runtime.into_host().asked,
        vec![
            (Some(7), "kick".to_owned()),
            (Some(7), "kick".to_owned()),
            (Some(7), "transform.y".to_owned()),
            (Some(7), "transform.x".to_owned()),
        ]
    );
}

#[test]
fn a_null_field_is_reported_as_a_null_reference_through_this() {
    let lowered =
        decay_ir::lower("script Kicker { var ball: Entity = null; fn go() { this.ball.kick(); } }");
    let program = lowered
        .program
        .unwrap_or_else(|| panic!("{:?}", lowered.analysis.diagnostics));
    let mut runtime = Runtime::new(&program, ReferenceHost::default());
    let mut kicker = runtime.instantiate("Kicker").expect("kicker");
    assert_eq!(
        runtime.call_instance(&mut kicker, "go", vec![]),
        Err(RuntimeError::NullReference("this.ball.kick".to_owned()))
    );
}

#[test]
fn a_path_from_a_computed_reference_is_about_that_reference() {
    let mut environment = decay_semantic::Environment::new();
    environment.add_value("World", decay_semantic::Type::Named("World".to_owned()));
    let lowered = decay_ir::lower_with_environment(
        r#"script Kicker {
            fn update(dt: f32) {
                World.find("Ball").kick(World.find("Ball").power);
                World.find("Ball").power += 2.0;
                let after = 1.0;
            }
        }"#,
        &environment,
    );
    let program = lowered
        .program
        .unwrap_or_else(|| panic!("{:?}", lowered.analysis.diagnostics));
    let mut runtime = Runtime::new(&program, ReferenceHost::default());
    let mut kicker = runtime.instantiate("Kicker").expect("kicker");
    runtime
        .call_instance(&mut kicker, "update", vec![Value::Number(0.0)])
        .expect("runs");
    assert_eq!(
        runtime.into_host().asked,
        vec![
            (Some(7), "power".to_owned()),
            (Some(7), "kick".to_owned()),
            (Some(7), "power".to_owned()),
            (Some(7), "power".to_owned()),
        ]
    );
}
