use super::{TextRenderer, shadow_color};
use crate::text::{TextAlign, TextInstance};

/// The families every glyph of `text`, shaped in `font`, was taken from.
fn families_shaping(renderer: &mut TextRenderer, font: &str, text: &str) -> Vec<String> {
    let instance = TextInstance::new(
        text,
        font,
        [0.0, 0.0],
        0.06,
        0.07,
        [1.0; 4],
        [TextAlign::Start; 2],
    )
    .expect("a valid instance");
    let face = renderer.fonts[font].clone();
    let buffer = renderer.shape(&instance, &face, 0.06);
    let mut families = Vec::new();
    for run in buffer.layout_runs() {
        for glyph in run.glyphs {
            let face = renderer
                .font_system
                .db()
                .face(glyph.font_id)
                .expect("a shaped glyph names a loaded face");
            families.push(face.families[0].0.clone());
        }
    }
    families
}

#[test]
fn a_shadow_fades_with_the_letters_that_cast_it() {
    let shadow = [0.0, 0.02, 0.05, 0.6];
    let alpha = |face: [f32; 4]| shadow_color(shadow, face)[3];
    assert!(
        (alpha([1.0, 1.0, 1.0, 1.0]) - 0.6).abs() < 1e-6,
        "an opaque letter keeps it"
    );
    assert!(
        alpha([1.0, 0.8, 0.5, 0.0]).abs() < 1e-6,
        "an invisible letter casts none"
    );
    assert!(
        (alpha([1.0, 1.0, 1.0, 0.5]) - 0.3).abs() < 1e-6,
        "a half-faded one half"
    );
    let half = shadow_color(shadow, [1.0, 1.0, 1.0, 0.5]);
    assert!(
        half[..3]
            .iter()
            .zip(&shadow[..3])
            .all(|(a, b)| (a - b).abs() < 1e-6),
        "only its strength changes: {half:?}"
    );
}

/// A project that ships only a bold file means that file. Asked for its
/// family at the regular weight, the shaper went to whatever regular face
/// the system had, and every Flappy title was drawn in a font nobody chose.
#[test]
fn a_bold_only_font_is_drawn_in_its_own_face() {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../games/flappy/assets/fonts/ChakraPetch-Bold.ttf"
    ))
    .expect("the fixture font is in the repository");
    let mut renderer = TextRenderer::new();
    renderer.bind_font("fonts/ChakraPetch-Bold.ttf", "Chakra Petch", bytes);
    let families = families_shaping(&mut renderer, "fonts/ChakraPetch-Bold.ttf", "FLAPPY");
    assert!(!families.is_empty(), "the word was shaped");
    assert!(
        families.iter().all(|family| family == "Chakra Petch"),
        "every glyph is the bound face's, not a system fallback: {families:?}"
    );
}
