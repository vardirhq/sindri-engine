//! Colors: built from channels or hex, read and written by channel, and
//! blended.

use crate::{EmptyHost, Runtime, RuntimeError, Value};

const SOURCE: &str = r##"script Paint {
    var tint = Color(1.0, 0.5, 0.0);
    fn opaque() -> f32 { return tint.a; }
    fn hex() -> Color { return Color("#ff8000"); }
    fn hex_alpha() -> f32 { return Color("#00000080").a; }
    fn channel() -> Color {
        var c = Color(0.0, 0.0, 0.0, 1.0);
        c.r = 1.0;
        c.g += 0.25;
        return c;
    }
    fn field_channel() -> f32 { tint.b = 0.75; return tint.b + tint.r; }
    fn blend() -> Color { return Color(0.0, 0.0, 0.0, 0.0).lerp(Color(1.0, 0.5, 0.25, 1.0), 0.5); }
    fn fade() -> Color { return tint.with_alpha(0.25); }
    fn same() -> bool { return Color("#ff8000") == Color(1.0, 128.0 / 255.0, 0.0) && tint != fade(); }
    fn bad(text: String) -> Color { return Color(text); }
}"##;

fn call(function: &str, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let lowered = decay_ir::lower(SOURCE);
    let program = lowered
        .program
        .unwrap_or_else(|| panic!("{:?}", lowered.analysis.diagnostics));
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut paint = runtime.instantiate("Paint").expect("instance");
    runtime.call_instance(&mut paint, function, args)
}

#[test]
fn a_color_is_built_from_channels_or_hex() {
    assert_eq!(call("opaque", vec![]), Ok(Value::Number(1.0)));
    assert_eq!(
        call("hex", vec![]),
        Ok(Value::Color([1.0, 128.0 / 255.0, 0.0, 1.0]))
    );
    assert_eq!(call("hex_alpha", vec![]), Ok(Value::Number(128.0 / 255.0)));
    assert_eq!(call("same", vec![]), Ok(Value::Bool(true)));
}

#[test]
fn a_channel_is_read_and_written_where_the_color_is_held() {
    assert_eq!(
        call("channel", vec![]),
        Ok(Value::Color([1.0, 0.25, 0.0, 1.0]))
    );
    assert_eq!(call("field_channel", vec![]), Ok(Value::Number(1.75)));
}

#[test]
fn a_color_blends_and_fades() {
    assert_eq!(
        call("blend", vec![]),
        Ok(Value::Color([0.5, 0.25, 0.125, 0.5]))
    );
    assert_eq!(
        call("fade", vec![]),
        Ok(Value::Color([1.0, 0.5, 0.0, 0.25]))
    );
}

#[test]
fn text_that_is_not_a_color_is_named_when_it_runs() {
    assert_eq!(
        call("bad", vec![Value::String("orange".to_owned())]),
        Err(RuntimeError::InvalidColor("orange".to_owned()))
    );
}

#[test]
fn a_palette_is_named_once_as_constants() {
    let source = r##"const ORANGE: Color = Color("#ff8000");
const DIM: Color = Color(0.5, 0.5, 0.5, 0.5);
script Palette {
    fn warm() -> Color { return ORANGE.lerp(DIM, 1.0); }
    fn hot() -> Color { return ORANGE; }
}"##;
    let lowered = decay_ir::lower(source);
    let program = lowered
        .program
        .unwrap_or_else(|| panic!("{:?}", lowered.analysis.diagnostics));
    let mut runtime = Runtime::new(&program, EmptyHost);
    let mut palette = runtime.instantiate("Palette").expect("instance");
    assert_eq!(
        runtime.call_instance(&mut palette, "hot", vec![]),
        Ok(Value::Color([1.0, 128.0 / 255.0, 0.0, 1.0]))
    );
    assert_eq!(
        runtime.call_instance(&mut palette, "warm", vec![]),
        Ok(Value::Color([0.5; 4]))
    );
}
