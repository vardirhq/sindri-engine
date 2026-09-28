//! Numbers written as text: a fixed count of decimals, or a whole number led
//! with zeros.

use crate::{EmptyHost, Runtime, RuntimeError, Value};

const SOURCE: &str = r#"script Hud {
    fn fixed(n: f32, digits: f32) -> String { return n.fixed(digits); }
    fn padded(n: f32, width: f32) -> String { return n.padded(width); }
    fn label(time: f32, wave: f32) -> String {
        return "Wave " + wave.padded(2) + " · " + time.fixed(1) + "s";
    }
    fn chained(n: f32) -> f32 { return (n * 2.0).fixed(0).length; }
}"#;

fn call(function: &str, args: &[f64]) -> Result<Value, RuntimeError> {
    let program = decay_ir::lower(SOURCE).program.expect("valid program");
    let mut runtime = Runtime::new(&program, EmptyHost);
    runtime.call(
        "Hud",
        function,
        args.iter().copied().map(Value::Number).collect(),
    )
}

fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

#[test]
fn fixed_writes_exactly_that_many_decimals() {
    assert_eq!(call("fixed", &[12.5, 2.0]), Ok(text("12.50")));
    assert_eq!(call("fixed", &[3.0, 1.0]), Ok(text("3.0")));
    assert_eq!(call("fixed", &[2.25, 1.0]), Ok(text("2.3")));
    assert_eq!(call("fixed", &[-2.25, 1.0]), Ok(text("-2.3")));
    assert_eq!(call("fixed", &[7.6, 0.0]), Ok(text("8")));
    // A number that rounds to nothing is not negative.
    assert_eq!(call("fixed", &[-0.001, 2.0]), Ok(text("0.00")));
    // What an f32 source holds for 0.1 still reads as it was written.
    assert_eq!(call("fixed", &[f64::from(0.1f32), 3.0]), Ok(text("0.100")));
}

#[test]
fn padded_leads_a_whole_number_with_zeros() {
    assert_eq!(call("padded", &[7.0, 3.0]), Ok(text("007")));
    assert_eq!(call("padded", &[1234.0, 2.0]), Ok(text("1234")));
    assert_eq!(call("padded", &[6.6, 2.0]), Ok(text("07")));
    assert_eq!(call("padded", &[-7.0, 3.0]), Ok(text("-007")));
    assert_eq!(call("padded", &[-0.2, 2.0]), Ok(text("00")));
    assert_eq!(call("padded", &[5.0, 0.0]), Ok(text("5")));
}

#[test]
fn formatted_numbers_join_and_chain_like_any_text() {
    assert_eq!(call("label", &[42.25, 3.0]), Ok(text("Wave 03 · 42.3s")));
    assert_eq!(call("chained", &[61.0]), Ok(Value::Number(3.0)));
}

#[test]
fn a_count_of_digits_must_be_whole_and_bounded() {
    for digits in [0.5, -1.0, 10.0, f64::NAN] {
        assert!(
            matches!(
                call("fixed", &[1.0, digits]),
                Err(RuntimeError::DigitsOutOfRange { most: 9, .. })
            ),
            "{digits}"
        );
    }
    assert!(matches!(
        call("padded", &[1.0, 21.0]),
        Err(RuntimeError::DigitsOutOfRange { most: 20, .. })
    ));
}

#[test]
fn a_number_that_is_not_finite_is_spelled_as_joining_would() {
    assert_eq!(call("fixed", &[f64::INFINITY, 2.0]), Ok(text("inf")));
}
