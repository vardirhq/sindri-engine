//! Colors: every way to build one, and every mistake with one, checked.

use crate::{Environment, analyze_with_environment};

fn messages(source: &str) -> Vec<String> {
    analyze_with_environment(source, &Environment::new())
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn a_color_is_accepted_wherever_a_value_goes() {
    let found = messages(
        r##"struct Theme { back: Color, fore: Color = Color("#ffffff") }
         script S {
             @export var tint: Color = Color(1.0, 0.5, 0.0);
             var palette: List<Color> = [Color("#000000"), Color(0.0, 0.0, 1.0, 0.5)];
             fn update(dt: f32) {
                 tint.a -= dt;
                 let mixed: Color = tint.lerp(palette[0], 0.5).with_alpha(tint.a);
                 let red: f32 = mixed.r + mixed.g + mixed.b;
                 if tint == mixed { }
                 let theme = Theme(back: mixed);
             }
         }"##,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_mistake_about_a_color_is_refused() {
    let found = messages(
        r#"script S {
             fn f() {
                 let fixed = Color(1.0, 1.0, 1.0);
                 let two = Color(1.0, 0.5);
                 let word = Color("orange");
                 let number = Color(1.0).r;
                 fixed.r = 0.0;
                 let x = fixed.x;
                 let l = fixed.lerp;
                 let c = fixed.r();
                 let w: f32 = fixed.with_alpha(0.5);
             }
         }"#,
    );
    let all = found.join("\n");
    for expected in [
        "`Color` takes `r, g, b`, `r, g, b, a` or a hex text like `\"#ff8800\"`, found 2 arguments",
        "`orange` is not a color: write `#rrggbb` or `#rrggbbaa`",
        "expected `String`, found `f32`",
        "cannot assign to a component of immutable `fixed`",
        "`Color` has no member `x`; it has `r`, `g`, `b`, `a`, `lerp`, `with_alpha`",
        "`lerp` on a color needs arguments -- call it: `.lerp(...)`",
        "`r` is a number, not a function -- write `.r`",
        "cannot assign `Color` to `f32`",
    ] {
        assert!(all.contains(expected), "missing {expected:?} in:\n{all}");
    }
}
