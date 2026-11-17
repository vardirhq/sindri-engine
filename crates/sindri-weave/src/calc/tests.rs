use super::{Calc, Mix, parse};

fn linear(text: &str) -> Mix {
    match parse(text) {
        Some(Calc::Linear(mix)) => mix,
        other => panic!("{text} is not linear: {other:?}"),
    }
}

/// Resolves with a pixel worth one unit and a viewport width worth ten per
/// `vw`, so a result reads as a sum of the two.
fn pixels_and_width(calc: &Calc) -> Option<f32> {
    calc.resolve(&|mix: Mix| Some(mix.pixels + mix.view_width * 10.0 + mix.overlay))
}

#[test]
fn lengths_add_and_scale_as_css_does() {
    let mix = linear("calc(100% - 2 * 20px + 5vw)");
    assert_eq!(
        mix,
        Mix {
            percent: 100.0,
            pixels: -40.0,
            view_width: 5.0,
            relative: true,
            ..Mix::default()
        }
    );
    let halved = linear("calc((50vh + 10px) / 2)");
    assert_eq!(halved.view_height, 25.0);
    assert_eq!(halved.pixels, 5.0);
    assert!(!halved.relative);
    assert_eq!(linear("calc(-10px + 30px)").pixels, 20.0);
    assert_eq!(linear("calc(calc(10px) * 3)").pixels, 30.0);
}

#[test]
fn what_css_refuses_is_refused() {
    for text in [
        "calc(10px * 10px)",
        "calc(10px / 0)",
        "calc(10px / 2px)",
        "calc(10px + 2)",
        "calc(3)",
        "calc(10em)",
        "calc(10px",
        "min(10px, 2)",
        "clamp(10px, 20px)",
        "min()",
        "lerp(1px, 2px)",
        "(10px)",
        "10px",
    ] {
        assert_eq!(parse(text), None, "{text}");
    }
    assert_eq!(parse("calc(0)"), Some(Calc::Linear(Mix::default())));
}

#[test]
fn min_and_max_choose_when_resolved() {
    let least = parse("min(300px, 40vw)").expect("a length");
    // 40vw is 400 here, so 300px is the smaller.
    assert_eq!(pixels_and_width(&least), Some(300.0));
    let most = parse("max(300px, 40vw)").expect("a length");
    assert_eq!(pixels_and_width(&most), Some(400.0));
    // A choice joins arithmetic like any length.
    let padded = parse("calc(min(10px, 2vw) * 2 + 4px)").expect("a length");
    assert_eq!(pixels_and_width(&padded), Some(24.0));
}

#[test]
fn clamp_keeps_a_length_between_two() {
    let clamped = parse("clamp(240px, 30vw, 360px)").expect("a length");
    assert_eq!(pixels_and_width(&clamped), Some(300.0));
    let low = parse("clamp(240px, 10vw, 360px)").expect("a length");
    assert_eq!(pixels_and_width(&low), Some(240.0));
    let high = parse("clamp(240px, 90vw, 360px)").expect("a length");
    assert_eq!(pixels_and_width(&high), Some(360.0));
}

#[test]
fn numbers_inside_are_chosen_at_once() {
    let scaled = linear("calc(10px * max(1, 3))");
    assert_eq!(scaled.pixels, 30.0);
    assert_eq!(linear("calc(10px * clamp(0, 5, 2))").pixels, 20.0);
}

#[test]
fn an_unresolvable_part_leaves_the_choice_unresolved() {
    let choice = parse("min(50%, 100px)").expect("a length");
    let none = choice.resolve(&|mix: Mix| (!mix.relative).then_some(mix.pixels));
    assert_eq!(none, None);
}
