//! Vectors: built by a script, added and scaled, asked their length, and
//! passed across the host boundary whole or one component at a time.

use decay_ir::lower_with_environment;
use decay_semantic::{Environment, HostType, Type};

use crate::{Host, Path, Runtime, RuntimeError, Value};

/// A host with one transform, answering its position whole and by component,
/// and remembering every path it was asked to store.
#[derive(Default)]
struct TransformHost {
    position: [f64; 3],
    stored: Vec<String>,
}

impl Host for TransformHost {
    fn load(&mut self, _subject: Option<u64>, path: &Path) -> Result<Option<Value>, RuntimeError> {
        Ok(match path.dotted().as_str() {
            "this.transform.position" => Some(Value::Vec3(self.position)),
            "this.transform.position.x" => Some(Value::Number(self.position[0])),
            _ => None,
        })
    }

    fn store(
        &mut self,
        _subject: Option<u64>,
        path: &Path,
        value: Value,
    ) -> Result<bool, RuntimeError> {
        self.stored.push(path.dotted());
        match (path.dotted().as_str(), value) {
            ("this.transform.position", Value::Vec3(position)) => self.position = position,
            ("this.transform.position.x", Value::Number(x)) => self.position[0] = x,
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn call(
        &mut self,
        _subject: Option<u64>,
        _path: &Path,
        _args: &[Value],
    ) -> Result<Option<Value>, RuntimeError> {
        Ok(None)
    }
}

fn environment() -> Environment {
    let mut environment = Environment::new();
    environment.add_type(
        "Transform",
        HostType::new().with_value("position", Type::Vec3),
    );
    environment.add_this_value("transform", Type::Named("Transform".to_owned()));
    environment
}

fn run_on(host: &mut TransformHost, source: &str, function: &str) -> Result<Value, RuntimeError> {
    let lowered = lower_with_environment(source, &environment());
    let program = lowered
        .program
        .unwrap_or_else(|| panic!("{:?}", lowered.analysis.diagnostics));
    let mut runtime = Runtime::new(&program, std::mem::take(host));
    let result = runtime.call("Mover", function, Vec::new());
    *host = runtime.into_host();
    result
}

fn run(body: &str) -> Result<Value, RuntimeError> {
    let source = format!("script Mover {{\n{body}\n}}");
    run_on(&mut TransformHost::default(), &source, "go")
}

fn refuse(body: &str) -> Vec<String> {
    let source = format!("script Mover {{\n{body}\n}}");
    lower_with_environment(&source, &environment())
        .analysis
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

fn close(value: Result<Value, RuntimeError>, expected: &[f64]) {
    let value = value.expect("it runs");
    let got: Vec<f64> = match &value {
        Value::Number(number) => vec![*number],
        other => other.components().expect("a vector").to_vec(),
    };
    assert_eq!(got.len(), expected.len(), "{value:?}");
    for (a, b) in got.iter().zip(expected) {
        assert!((a - b).abs() < 1e-9, "{value:?} is not {expected:?}");
    }
}

#[test]
fn a_vector_is_built_and_read_by_component() {
    close(
        run("fn go() -> f32 { let v = Vec2(3.0, 4.0); return v.x + v.y * 10.0; }"),
        &[43.0],
    );
    close(
        run("fn go() -> f32 { return Vec3(1.0, 2.0, 3.0).z; }"),
        &[3.0],
    );
}

#[test]
fn vectors_add_subtract_scale_divide_and_negate() {
    close(
        run("fn go() -> Vec2 { return Vec2(1.0, 2.0) + Vec2(10.0, 20.0); }"),
        &[11.0, 22.0],
    );
    close(
        run("fn go() -> Vec2 { return Vec2(5.0, 5.0) - Vec2(1.0, 2.0); }"),
        &[4.0, 3.0],
    );
    close(
        run("fn go() -> Vec3 { return Vec3(1.0, 2.0, 3.0) * 2.0; }"),
        &[2.0, 4.0, 6.0],
    );
    close(
        run("fn go() -> Vec3 { return 2.0 * Vec3(1.0, 2.0, 3.0); }"),
        &[2.0, 4.0, 6.0],
    );
    close(
        run("fn go() -> Vec2 { return Vec2(4.0, 8.0) / 4.0; }"),
        &[1.0, 2.0],
    );
    close(
        run("fn go() -> Vec2 { return -Vec2(1.0, -2.0); }"),
        &[-1.0, 2.0],
    );
}

#[test]
fn a_vector_knows_its_length_direction_and_neighbours() {
    close(
        run("fn go() -> f32 { return Vec2(3.0, 4.0).length; }"),
        &[5.0],
    );
    close(
        run("fn go() -> Vec2 { return Vec2(3.0, 4.0).normalized; }"),
        &[0.6, 0.8],
    );
    close(
        run("fn go() -> f32 { return Vec2(1.0, 2.0).dot(Vec2(3.0, 4.0)); }"),
        &[11.0],
    );
    close(
        run("fn go() -> f32 { return Vec2(1.0, 1.0).distance(Vec2(4.0, 5.0)); }"),
        &[5.0],
    );
    close(
        run("fn go() -> Vec3 { return Vec3(0.0, 0.0, 0.0).lerp(Vec3(10.0, 20.0, 30.0), 0.25); }"),
        &[2.5, 5.0, 7.5],
    );
    close(
        run("fn go() -> f32 { return (Vec2(1.0, 0.0) + Vec2(2.0, 4.0)).length; }"),
        &[5.0],
    );
    close(
        run("fn go() -> f32 { return Vec2(0.0, 7.0).normalized.y; }"),
        &[1.0],
    );
}

/// A zero vector has no direction, and normalizing it gives zero rather than
/// NaN that would spread into every number it touches.
#[test]
fn normalizing_nothing_gives_nothing() {
    close(
        run("fn go() -> Vec2 { return Vec2(0.0, 0.0).normalized; }"),
        &[0.0, 0.0],
    );
}

#[test]
fn a_component_is_assigned_and_compound_assigned() {
    close(
        run("fn go() -> Vec2 { var v = Vec2(1.0, 2.0); v.x = 5.0; v.y += 10.0; return v; }"),
        &[5.0, 12.0],
    );
    close(
        run("fn go() -> f32 { var v = Vec2(1.0, 2.0); return v.x = 9.0; }"),
        &[9.0],
    );
    close(
        run("fn go() -> Vec2 { var v = Vec2(1.0, 1.0); v += Vec2(2.0, 3.0); v *= 2.0; return v; }"),
        &[6.0, 8.0],
    );
}

/// A vector is a value: copying one and changing the copy leaves the original.
#[test]
fn a_copy_is_its_own_vector() {
    close(
        run("fn go() -> f32 { let a = Vec2(1.0, 1.0); var b = a; b.x = 100.0; return a.x; }"),
        &[1.0],
    );
}

#[test]
fn a_vector_field_starts_at_zero_and_is_kept() {
    let source = "script Mover {
        var velocity: Vec2;
        fn push() { this.velocity.x += 2.0; velocity.y -= 1.0; }
        fn go() -> Vec2 { push(); push(); return velocity; }
    }";
    close(
        run_on(&mut TransformHost::default(), source, "go"),
        &[4.0, -2.0],
    );
}

#[test]
fn vectors_compare_equal_by_value() {
    assert_eq!(
        run(
            "fn go() -> bool { return Vec2(1.0, 2.0) == Vec2(1.0, 2.0) && Vec2(1.0, 2.0) != Vec2(2.0, 1.0); }"
        ),
        Ok(Value::Bool(true))
    );
}

/// `this.transform.position.x` is still one path the host answers, exactly as
/// before vectors existed, while the whole position crosses as one value.
#[test]
fn a_host_vector_is_reached_whole_or_by_its_path() {
    let mut host = TransformHost {
        position: [1.0, 2.0, 3.0],
        ..TransformHost::default()
    };
    let source = "script Mover {
        fn go() -> f32 {
            this.transform.position.x += 1.0;
            let here = this.transform.position;
            this.transform.position = here + Vec3(0.0, 10.0, 0.0);
            this.transform.position += Vec3(0.0, 0.0, 100.0);
            return here.x;
        }
    }";
    close(run_on(&mut host, source, "go"), &[2.0]);
    close(
        Ok(Value::vector(&host.position).expect("three")),
        &[2.0, 12.0, 103.0],
    );
    assert_eq!(
        host.stored,
        [
            "this.transform.position.x",
            "this.transform.position",
            "this.transform.position"
        ]
    );
}

#[test]
fn mismatched_vector_arithmetic_does_not_compile() {
    let said = |body: &str| refuse(body).join("\n");
    assert!(
        said("fn go() { let a = Vec2(1.0, 1.0) + Vec3(1.0, 1.0, 1.0); }").contains("same size")
    );
    assert!(said("fn go() { let a = Vec2(1.0, 1.0) + 1.0; }").contains("same size"));
    assert!(said("fn go() { let a = Vec2(1.0, 1.0) * Vec2(1.0, 1.0); }").contains("dot"));
    assert!(said("fn go() { let a = 2.0 / Vec2(1.0, 1.0); }").contains("divided by a number"));
    assert!(said("fn go() { let a = Vec2(1.0, 1.0) % 2.0; }").contains("numbers, not vectors"));
    assert!(
        said("fn go() { var speed = 1.0; speed += Vec2(1.0, 1.0); }").contains("cannot assign")
    );
    assert!(
        said("fn go() { let a = Vec2(1.0, 1.0) < Vec2(2.0, 2.0); }").contains("expected `f32`")
    );
}

#[test]
fn vector_members_are_checked() {
    let said = |body: &str| refuse(body).join("\n");
    assert!(said("fn go() { let a = Vec2(1.0, 1.0).z; }").contains("`Vec2` has no member `z`"));
    assert!(said("fn go() { let a = Vec2(1.0, 1.0).length(); }").contains("property"));
    assert!(said("fn go() { let a = Vec2(1.0, 1.0).dot; }").contains("call it"));
    assert!(
        said("fn go() { let a = Vec2(1.0, 1.0).dot(Vec3(1.0, 1.0, 1.0)); }")
            .contains("cannot assign")
    );
    assert!(said("fn go() { let a = Vec2(1.0); }").contains("takes 2 numbers"));
    assert!(said("fn go() { let a = Vec3(1.0, \"two\", 3.0); }").contains("expected `f32`"));
    assert!(said("fn go() { let a = Vec2(1.0, 1.0); a.x = 2.0; }").contains("immutable `a`"));
    assert!(said("fn go() { let a: Vec2 = null; }").contains("cannot assign"));
    assert!(refuse("fn go() -> Vec3 { return Vec3(1.0, 2.0, 3.0).normalized; }").is_empty());
}
