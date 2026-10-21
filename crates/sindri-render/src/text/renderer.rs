//! Shaping a string, filling the atlas, and emitting the quads.
//!
//! The one part of text that needs a font and a GPU. Everything it decides about
//! *where* the words go comes from `layout.rs`, so that arithmetic stays
//! checkable without either.

use std::collections::BTreeMap;

use glam::{Mat4, Vec3};
use glyphon::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Style, SwashCache, Weight, Wrap,
};

use crate::{FIELD_PER_RASTER_PIXEL, GlyphAtlas, GlyphInstance, RASTER_EM, Texture2D};

use super::TextError;
use super::instance::TextInstance;
use super::layout::{LaidOut, laid_out, layout_origin, line_alignment, outer_corner};
use super::options::TextWrap;
use super::rich::rich_spans;

/// One glyph batch, ready to draw through the pass's own camera.
pub struct GlyphQuads<'a> {
    /// The atlas every quad in `instances` samples.
    pub atlas: &'a Texture2D,
    /// Which build of that atlas this is, for a renderer caching bind groups.
    pub generation: u64,
    pub instances: Vec<GlyphInstance>,
}

/// Glyph shaping, rasterisation and the atlas they land in, shared by every
/// viewport.
pub struct TextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: GlyphAtlas,
    /// Logical asset reference to the face its bytes hold.
    fonts: BTreeMap<String, BoundFace>,
}

/// The face a font file holds: its family, and the weight and slant it was
/// drawn at.
///
/// A scene names a file, so it means that file's face. Asking only for the
/// family meant asking for its *regular* weight, and a project that shipped
/// just `Something-Bold.ttf` was drawn in whatever the system had instead --
/// the shaper would rather find a regular face anywhere than a bold one in the
/// family it was told to use.
#[derive(Clone, Debug, PartialEq, Eq)]
struct BoundFace {
    family: String,
    weight: Weight,
    style: Style,
}
impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            atlas: GlyphAtlas::new(),
            fonts: BTreeMap::new(),
        }
    }

    /// Makes validated font bytes available under the scene reference.
    pub fn bind_font(
        &mut self,
        reference: impl Into<String>,
        family: impl Into<String>,
        bytes: Vec<u8>,
    ) -> Option<String> {
        let ids = self
            .font_system
            .db_mut()
            .load_font_source(glyphon::fontdb::Source::Binary(std::sync::Arc::new(bytes)));
        let (weight, style) = ids
            .first()
            .and_then(|id| self.font_system.db().face(*id))
            .map_or((Weight::NORMAL, Style::Normal), |face| {
                (face.weight, face.style)
            });
        self.fonts
            .insert(
                reference.into(),
                BoundFace {
                    family: family.into(),
                    weight,
                    style,
                },
            )
            .map(|face| face.family)
    }

    pub fn unbind_font(&mut self, reference: &str) -> Option<String> {
        self.fonts.remove(reference).map(|face| face.family)
    }

    pub fn clear_bindings(&mut self) {
        self.fonts.clear();
    }

    pub fn has_font(&self, reference: &str) -> bool {
        self.fonts.contains_key(reference)
    }

    /// How much of its own units one string covers, across and down.
    ///
    /// `None` for an unbound font, which is what [`Self::quads`] does with one
    /// too: a string whose face never arrived is not drawn, so it covers nothing
    /// and there is nothing to hit-test against.
    ///
    /// The width is the widest laid-out line and the height is the lines it
    /// actually used — the block, not the box it was allowed. Independent of any
    /// viewport, because the answer is a size in the scene now rather than a
    /// rectangle on a screen, which is what lets an editor's pick box be built
    /// once and stay right through a zoom.
    ///
    /// This exists because an editor cannot otherwise say where a string is.
    /// Every other drawn thing has a size in the scene; a string's is decided by
    /// glyph layout inside this module, and a guessed box picks the wrong thing
    /// near its edges. So the answer comes from the same shaping the frame is
    /// drawn from.
    pub fn measure(&mut self, instance: &TextInstance) -> Option<[f32; 2]> {
        let face = self.fonts.get(instance.font()).cloned()?;
        let laid = self.lay_out(instance, &face);
        Some(laid.block)
    }

    /// Where a string sits and how much room it takes, in its own units.
    ///
    /// The words' own extent when the string has no box, and the box itself when
    /// it has one: a label given a rect occupies that rect whether or not its
    /// words fill it, which is what an editor draws a handle around and what a
    /// click has to land in.
    ///
    /// Answered as a centre and a size, because that is the shape every other
    /// element in the scene is described by.
    pub fn rect(&mut self, instance: &TextInstance) -> Option<([f32; 2], [f32; 2])> {
        let face = self.fonts.get(instance.font()).cloned()?;
        let laid = self.lay_out(instance, &face);
        let (corner, outer) = outer_corner(instance, laid.block);
        Some((
            [corner[0] + outer[0] * 0.5, corner[1] - outer[1] * 0.5],
            outer,
        ))
    }

    /// One instance laid out, the only place that is decided.
    ///
    /// Shared by drawing and measuring rather than written twice, because two
    /// copies is exactly how a pick box ends up disagreeing with the picture it
    /// is meant to be over.
    fn lay_out(&mut self, instance: &TextInstance, face: &BoundFace) -> LaidOut {
        let size = self.fitted_size(instance, face);
        let buffer = self.shape(instance, face, size);
        let scale = TextInstance::units_per_raster_pixel(size);
        let [width, height] = laid_out(&buffer, instance.line_height / scale);
        LaidOut {
            buffer,
            wrapped: instance.wraps(),
            scale,
            block: [width * scale, height * scale],
            faces: if instance.rich {
                rich_spans(&instance.text)
                    .into_iter()
                    .map(|span| span.color)
                    .collect()
            } else {
                Vec::new()
            },
        }
    }

    /// The font size this string is actually drawn at.
    ///
    /// The authored one unless it was told to fit a box, in which case the
    /// largest size in range whose block fits. A bisection rather than a walk
    /// down from the maximum: a dozen shapes settles a range to within a
    /// thousandth of it, and a walk is unbounded in how many it takes.
    fn fitted_size(&mut self, instance: &TextInstance, face: &BoundFace) -> f32 {
        let Some(fit) = instance.fit else {
            return instance.font_size;
        };
        // Nothing to be too big for. A fit with no box would shrink text to its
        // minimum for no reason anyone could see.
        if instance.bounds[0] <= 0.0 && instance.bounds[1] <= 0.0 {
            return instance.font_size;
        }
        let fits = |renderer: &mut Self, size: f32| {
            let buffer = renderer.shape(instance, face, size);
            let scale = TextInstance::units_per_raster_pixel(size);
            // The line height is a share of the authored size, so it has to
            // travel with the trial size or a shrunk block keeps its old leading.
            let leading = instance.line_height / instance.font_size * size;
            let [width, height] = laid_out(&buffer, leading / scale);
            (instance.bounds[0] <= 0.0 || width * scale <= instance.bounds[0])
                && (instance.bounds[1] <= 0.0 || height * scale <= instance.bounds[1])
        };
        if fits(self, fit.max) {
            return fit.max;
        }
        let (mut small, mut large) = (fit.min, fit.max);
        for _ in 0..12 {
            let middle = f32::midpoint(small, large);
            if fits(self, middle) {
                small = middle;
            } else {
                large = middle;
            }
        }
        small
    }

    /// The buffer for one string at one size.
    ///
    /// Always shaped at [`RASTER_EM`], whatever size the instance asked for: the
    /// atlas holds one baked copy of each glyph, and a string shaped at its own
    /// size would key glyphs that are not in it. The requested size becomes the
    /// scale everything else is measured against, so it reaches the layout as a
    /// wrap width and a line height rather than as an em.
    fn shape(&mut self, instance: &TextInstance, face: &BoundFace, size: f32) -> Buffer {
        let scale = TextInstance::units_per_raster_pixel(size);
        // The authored leading is a share of the authored size; at a fitted size
        // it keeps the same share.
        let leading = instance.line_height / instance.font_size * size;
        let mut buffer = Buffer::new(
            &mut self.font_system,
            Metrics::new(RASTER_EM, leading / scale),
        );
        // A width to wrap in, in the raster pixels the buffer is laid out in.
        // Without one nothing wraps, whatever mode was chosen — which is right:
        // a HUD reading has no box and breaking it would be worse than
        // overflowing.
        let wrap_width = instance.wraps().then(|| instance.bounds[0] / scale);
        buffer.set_size(wrap_width, None);
        buffer.set_wrap(match instance.wrap {
            TextWrap::None => Wrap::None,
            TextWrap::Word => Wrap::WordOrGlyph,
            TextWrap::Glyph => Wrap::Glyph,
        });

        let mut attrs = Attrs::new()
            .family(Family::Name(&face.family))
            .weight(face.weight)
            .style(face.style);
        if instance.style.bold {
            attrs = attrs.weight(Weight::BOLD.max(face.weight));
        }
        if instance.style.italic {
            attrs = attrs.style(Style::Italic);
        }
        if instance.letter_spacing != 0.0 {
            // Cosmic-text adds this to a glyph's advance *in ems*, not in the
            // pixels the buffer is otherwise laid out in — advances are kept
            // normalised and multiplied by the font size later. Handing it a
            // pixel count moves each letter that many ems along, which puts the
            // whole string off the side of the screen with one glyph left
            // visible. As a share of the em it is just the authored distance
            // over the authored size.
            attrs = attrs.letter_spacing(instance.letter_spacing / size);
        }
        if instance.rich {
            let spans: Vec<_> = rich_spans(&instance.text)
                .into_iter()
                .map(|span| (instance.case.applied(&span.text).into_owned(), span))
                .collect();
            // Each span's index rides on its glyphs as metadata, one past it so
            // that cosmic-text's own zero still means "the string's color".
            buffer.set_rich_text(
                spans.iter().enumerate().map(|(index, (text, span))| {
                    let mut own = attrs.clone().metadata(index + 1);
                    if span.bold {
                        own = own.weight(Weight::BOLD.max(face.weight));
                    }
                    if span.italic {
                        own = own.style(Style::Italic);
                    }
                    (text.as_str(), own)
                }),
                &attrs,
                Shaping::Advanced,
                None,
            );
        } else {
            buffer.set_text(
                &instance.case.applied(&instance.text),
                &attrs,
                Shaping::Advanced,
                None,
            );
        }

        // Line alignment is per line rather than per buffer in cosmic-text, and
        // it only means anything with a width to align within.
        if let Some(align) = line_alignment(instance)
            && wrap_width.is_some()
        {
            for line in &mut buffer.lines {
                line.set_align(Some(align));
            }
        }
        buffer.shape_until_scroll(&mut self.font_system, false);
        buffer
    }

    /// Shapes one ordered text pass into glyph quads, and the atlas they sample.
    ///
    /// An unbound font skips its string. Asset diagnostics name it separately;
    /// silently choosing a machine font here would make a browser and a desktop
    /// disagree while both appeared to work.
    ///
    /// `None` when nothing is drawn — every string had an unbound font, or none
    /// of them put a mark anywhere.
    pub fn quads(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[TextInstance],
    ) -> Result<Option<GlyphQuads<'_>>, TextError> {
        let resolved: Vec<(&TextInstance, BoundFace)> = instances
            .iter()
            .filter_map(|instance| {
                self.fonts
                    .get(instance.font())
                    .cloned()
                    .map(|face| (instance, face))
            })
            .collect();
        if resolved.is_empty() {
            return Ok(None);
        }
        let laid: Vec<LaidOut> = resolved
            .iter()
            .map(|(instance, face)| self.lay_out(instance, face))
            .collect();

        // Fill the atlas before reading it, and let it settle before reading it:
        // growing repacks the atlas, so a rect handed out before a grow stops
        // being where its glyph is. `fill` owns that invariant, which is why the
        // quads below are built in a second pass over the same layout.
        let keys: Vec<_> = laid
            .iter()
            .flat_map(|laid| laid.buffer.layout_runs())
            .flat_map(|run| {
                run.glyphs
                    .iter()
                    .map(|glyph| glyph.physical((0.0, 0.0), 1.0).cache_key)
                    .collect::<Vec<_>>()
            })
            .collect();
        self.atlas
            .fill(&mut self.font_system, &mut self.swash_cache, &keys);

        let mut quads = Vec::new();
        for (laid, (instance, _)) in laid.iter().zip(resolved.iter()) {
            self.push_string(&mut quads, instance, laid);
        }
        if quads.is_empty() {
            return Ok(None);
        }
        let (atlas, generation) = self.atlas.texture(device, queue)?;
        Ok(Some(GlyphQuads {
            atlas,
            generation,
            instances: quads,
        }))
    }

    /// One string's quads: its shadow first, then its letters over it.
    fn push_string(
        &mut self,
        quads: &mut Vec<GlyphInstance>,
        instance: &TextInstance,
        laid: &LaidOut,
    ) {
        let origin = layout_origin(instance, laid);
        // An outline and a shadow's softness are authored on the text and read
        // by the shader against the stored field, so this is where they cross
        // over: a raster pixel is worth a fixed slice of the field, and the
        // scale says how many raster pixels a unit of the scene is.
        let field = |units: f32| units / laid.scale * FIELD_PER_RASTER_PIXEL;
        // The authored width is how far the stroke reaches *out* from the edge,
        // which is what the shader's threshold measures, so it crosses over
        // whole rather than halved.
        let outline_width = field(instance.outline.width);

        if instance.shadow.is_drawn() {
            self.push_glyphs(
                quads,
                instance,
                laid,
                |glyph| {
                    glyph
                        .with_softness(field(instance.shadow.softness))
                        // The shadow is the letter's shape in one flat color, so
                        // its outline is drawn in the same color rather than left
                        // as a rim of the face's.
                        .with_outline(outline_width, instance.shadow.color)
                },
                [
                    origin[0] + instance.shadow.offset[0],
                    origin[1] + instance.shadow.offset[1],
                ],
                |_| instance.shadow.color,
            );
        }

        let outline = instance.outline;
        self.push_glyphs(
            quads,
            instance,
            laid,
            move |glyph| {
                if outline.is_drawn() {
                    glyph.with_outline(outline_width, outline.color)
                } else {
                    glyph
                }
            },
            origin,
            |span| face_color(laid, instance.color, span),
        );
    }

    /// Every drawn glyph of one string, placed from `origin` and colored by
    /// `face` from the span it is in, with `adjust` applied to each.
    fn push_glyphs(
        &mut self,
        quads: &mut Vec<GlyphInstance>,
        instance: &TextInstance,
        laid: &LaidOut,
        adjust: impl Fn(GlyphInstance) -> GlyphInstance,
        origin: [f32; 2],
        face: impl Fn(usize) -> [f32; 4],
    ) {
        let scale = laid.scale;
        // The whole string, shadow included, turns about the point it was
        // placed at, so the anchor stays where the entity is.
        let pivot = Vec3::new(instance.position[0], instance.position[1], 0.0);
        let turn = Mat4::from_translation(pivot)
            * Mat4::from_rotation_z(instance.rotation)
            * Mat4::from_translation(-pivot);
        let mut drawn = 0_usize;
        for run in laid.buffer.layout_runs() {
            for glyph in run.glyphs {
                if instance.visible.is_some_and(|limit| drawn >= limit) {
                    return;
                }
                drawn += 1;
                let physical = glyph.physical((0.0, 0.0), 1.0);
                // Pen offsets within a shaped line, so a few hundred at most and
                // exact in an f32 many times over.
                #[allow(clippy::cast_precision_loss)]
                let (pen_x, pen_y) = (physical.x as f32, physical.y as f32);
                // Read, never bake. Baking here could grow the atlas, and
                // growing repacks it — every quad already pushed would then be
                // drawing whatever landed at its coordinates afterwards. The
                // atlas was filled before this pass began; a glyph missing now
                // is one it had no room for, and is skipped.
                let Some(slot) = self.atlas.placed(physical.cache_key) else {
                    continue;
                };
                // Raster pixels from the string's top-left corner, down and to
                // the right, which is the space cosmic-text lays out in.
                let left = pen_x + slot.offset[0];
                let top = run.line_y.round() + pen_y - slot.offset[1];
                let size = [slot.size[0] * scale, slot.size[1] * scale];
                let centre = [
                    origin[0] + (left + slot.size[0] * 0.5) * scale,
                    origin[1] - (top + slot.size[1] * 0.5) * scale,
                ];
                let model = turn
                    * Mat4::from_translation(Vec3::new(centre[0], centre[1], 0.0))
                    * Mat4::from_scale(Vec3::new(size[0], size[1], 1.0));
                let quad = GlyphInstance::new(model, slot.uv, face(glyph.metadata));
                let quad = if slot.colored {
                    quad.colored()
                } else {
                    adjust(quad)
                };
                quads.push(quad);
            }
        }
    }

    /// How many distinct glyphs are baked into the atlas.
    ///
    /// A host's only window onto how much text has cost it.
    #[must_use]
    pub fn glyph_count(&self) -> usize {
        self.atlas.glyph_count()
    }
}

/// The color a glyph is drawn in: its span's own, if it has one, faded by the
/// string's alpha so a whole rich line still fades as one.
fn face_color(laid: &LaidOut, string: [f32; 4], span: usize) -> [f32; 4] {
    match span
        .checked_sub(1)
        .and_then(|index| laid.faces.get(index).copied().flatten())
    {
        Some([r, g, b, a]) => [r, g, b, a * string[3]],
        None => string,
    }
}

#[cfg(test)]
mod tests {
    use super::TextRenderer;
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
}
