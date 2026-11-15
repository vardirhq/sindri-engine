//! What the post-process does: grade, glow, the band in focus, and the dark.

/// How much light, from how bright, and how far.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BloomSettings {
    /// Whether the bloom stages run at all.
    pub enabled: bool,
    /// How bright a color has to be before it glows at all, on the linear 0-1
    /// scale the shader works in.
    ///
    /// Below one, so ordinary bright colors glow. Neon on black is the case
    /// this exists for and none of it is over-bright: a mint stroke is about
    /// `0.9` at its brightest channel, so a threshold at or above one would
    /// leave the whole picture matte.
    pub threshold: f32,
    /// The band either side of the threshold over which a color ramps into
    /// glowing, rather than starting to at a stroke.
    ///
    /// A hard cutoff makes the glow switch on and off as something pulses or
    /// fades across the threshold, which reads as a flicker.
    pub knee: f32,
    /// How much of the blurred light is added back.
    pub intensity: f32,
    /// How many times the blur is run. Each pass is a horizontal and a vertical
    /// sweep, and each roughly doubles how far the light reaches.
    ///
    /// Three is a glow around a thing. Six is a haze over the whole screen.
    pub passes: u32,
}

impl Default for BloomSettings {
    fn default() -> Self {
        // Tuned by shooting the shape specimen at a spread of settings and
        // looking at them. Higher intensity than this washes a saturated stroke
        // out towards white — the coral pentagon loses its coral first, which
        // is the tell — because a color already near the top of the 0-1 range
        // has nowhere to go but grey when light is added to it.
        Self {
            enabled: true,
            threshold: 0.65,
            knee: 0.20,
            intensity: 0.45,
            passes: 3,
        }
    }
}

impl BloomSettings {
    /// The settings with every number forced into a range that draws something.
    ///
    /// A negative threshold makes every pixel including black glow, a zero knee
    /// divides by nothing, and zero passes leaves an unblurred bright-pass to be
    /// added back — which is not a glow but a second, brighter copy of the
    /// picture. None of those is worth a caller-facing error, so they are
    /// clamped and drawn.
    #[must_use]
    pub(super) fn sane(self) -> Self {
        Self {
            enabled: self.enabled,
            threshold: self.threshold.max(0.0),
            knee: self.knee.clamp(1.0e-4, 1.0),
            intensity: self.intensity.max(0.0),
            passes: self.passes.clamp(1, MAX_PASSES),
        }
    }
}

/// A band of the screen in focus, and the picture blurring away above and
/// below it: the tilt-shift look that makes a world seen from above read as
/// small, near and handmade.
///
/// It works on the screen rather than on depth, because a tilted camera looking
/// down at a flat world already puts the far ground at the top of the screen and
/// the near at the bottom, and because the sprites that stand in such a world
/// write no depth to read. The band is where the camera's subject stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthOfFieldSettings {
    /// Whether the picture blurs at all.
    pub enabled: bool,
    /// Where the band in focus is centred, from the top of the screen (0) to
    /// the bottom (1).
    pub focus: f32,
    /// How far the sharp band reaches either side of `focus`, as a share of the
    /// screen's height.
    pub range: f32,
    /// How far past `range` the blur takes to reach its full width, as a share
    /// of the screen's height. A wide falloff is a lens; a narrow one is a line
    /// across the picture.
    pub falloff: f32,
    /// How wide the blur is at its widest, as a share of the screen's height,
    /// so the picture blurs alike on a phone and on a monitor.
    pub blur: f32,
}

impl Default for DepthOfFieldSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            focus: 0.55,
            range: 0.12,
            falloff: 0.3,
            blur: 0.004,
        }
    }
}

impl DepthOfFieldSettings {
    /// Past this the blur stops reading as a lens and starts reading as a
    /// smear: the taps spread too far apart to agree.
    pub const MAX_BLUR: f32 = 0.02;

    #[must_use]
    pub(super) fn sane(self) -> Self {
        Self {
            enabled: self.enabled,
            focus: self.focus.clamp(0.0, 1.0),
            range: self.range.clamp(0.0, 1.0),
            falloff: self.falloff.clamp(1.0e-3, 1.0),
            blur: self.blur.clamp(0.0, Self::MAX_BLUR),
        }
    }

    /// Whether this blurs anything.
    #[must_use]
    pub fn blurs(self) -> bool {
        let sane = self.sane();
        sane.enabled && sane.blur > f32::EPSILON
    }
}

/// How the eye sees in the dark: what little light there is shows no colour and
/// no detail, so dark places go grey and soft while what is lit stays sharp.
///
/// Judged by how bright each place is on the screen, as the eye judges it, so it
/// needs nothing from the lights: a lantern's pool is crisp and coloured, and the
/// land past its edge fades into a grey blur.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NightVisionSettings {
    /// Whether the dark is seen this way at all.
    pub enabled: bool,
    /// How bright a place has to be, on the linear 0-1 scale, before it is seen
    /// plainly. Below it, it greys and blurs the darker it is.
    pub threshold: f32,
    /// How wide the blur is in the darkest places, as a share of the screen's
    /// height.
    pub blur: f32,
    /// How much colour the darkest places lose, from 0 (none) to 1 (all).
    pub grey: f32,
}

impl Default for NightVisionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 0.12,
            blur: 0.006,
            grey: 0.85,
        }
    }
}

impl NightVisionSettings {
    #[must_use]
    pub(super) fn sane(self) -> Self {
        Self {
            enabled: self.enabled,
            threshold: self.threshold.clamp(1.0e-3, 1.0),
            blur: self.blur.clamp(0.0, DepthOfFieldSettings::MAX_BLUR),
            grey: self.grey.clamp(0.0, 1.0),
        }
    }

    /// Whether this changes anything.
    #[must_use]
    pub fn sees(self) -> bool {
        let sane = self.sane();
        sane.enabled && (sane.blur > f32::EPSILON || sane.grey > f32::EPSILON)
    }
}

/// Tone curve applied to the world before bloom and vignette.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ToneMapping {
    /// Preserve the scene's linear color without a tone curve.
    #[default]
    None,
    /// A cheap, gentle shoulder suitable for ordinary scenes.
    Reinhard,
    /// A filmic approximation with a stronger highlight shoulder.
    Aces,
}

impl ToneMapping {
    pub(super) const fn shader_value(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Reinhard => 1.0,
            Self::Aces => 2.0,
        }
    }
}

/// Ordered world post-processing applied before overlay/UI rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PostProcessSettings {
    /// Exposure in photographic stops. One stop doubles linear brightness.
    pub exposure: f32,
    /// Mid-grey contrast where 1.0 preserves the source.
    pub contrast: f32,
    /// Color saturation where 0.0 is greyscale and 1.0 preserves the source.
    pub saturation: f32,
    pub tone_mapping: ToneMapping,
    /// Edge darkening strength from 0.0 to 1.0.
    pub vignette: f32,
    pub bloom: BloomSettings,
    /// The band in focus, applied to the scene before it is graded.
    pub depth_of_field: DepthOfFieldSettings,
    /// The dark seen grey and soft, applied with the depth of field.
    pub night_vision: NightVisionSettings,
}

impl Default for PostProcessSettings {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            tone_mapping: ToneMapping::None,
            vignette: 0.0,
            bloom: BloomSettings {
                enabled: false,
                ..BloomSettings::default()
            },
            depth_of_field: DepthOfFieldSettings::default(),
            night_vision: NightVisionSettings::default(),
        }
    }
}

impl PostProcessSettings {
    #[must_use]
    pub(super) fn sane(self) -> Self {
        Self {
            exposure: self.exposure.clamp(-8.0, 8.0),
            contrast: self.contrast.clamp(0.0, 4.0),
            saturation: self.saturation.clamp(0.0, 4.0),
            tone_mapping: self.tone_mapping,
            vignette: self.vignette.clamp(0.0, 1.0),
            bloom: self.bloom.sane(),
            depth_of_field: self.depth_of_field.sane(),
            night_vision: self.night_vision.sane(),
        }
    }

    #[must_use]
    pub fn is_active(self) -> bool {
        let sane = self.sane();
        sane.exposure.abs() > f32::EPSILON
            || (sane.contrast - 1.0).abs() > f32::EPSILON
            || (sane.saturation - 1.0).abs() > f32::EPSILON
            || sane.tone_mapping != ToneMapping::None
            || sane.vignette > f32::EPSILON
            || sane.bloom.enabled
            || sane.depth_of_field.blurs()
            || sane.night_vision.sees()
    }
}

/// Past this the blur has stopped being a glow and started being a fog, and the
/// cost is real: every pass is two more full-screen draws.
pub(super) const MAX_PASSES: u32 = 8;

#[cfg(test)]
mod tests {
    use super::super::Bloom;
    use super::{
        BloomSettings, DepthOfFieldSettings, MAX_PASSES, PostProcessSettings, ToneMapping,
    };

    /// The world is drawn into the scene with the same pipelines that draw the
    /// interface onto the output, so a browser's `Bgra8UnormSrgb` canvas gets a
    /// scene in that format rather than one its pipelines are rejected by.
    #[test]
    fn the_scene_is_drawn_in_an_srgb_outputs_own_format() {
        for output in [
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ] {
            assert_eq!(Bloom::scene_format_for(output), output);
        }
        assert_eq!(
            Bloom::scene_format_for(wgpu::TextureFormat::Rgba16Float),
            Bloom::SCENE_FORMAT,
            "a linear output still gets an encoded picture to glow from"
        );
    }

    /// Every setting that would draw something other than a glow is clamped
    /// rather than refused.
    ///
    /// None of these is worth failing a frame over, and each has an obvious
    /// nearest sane value: a negative threshold would make black itself glow, a
    /// zero knee divides by nothing, and zero blur passes would add an
    /// unblurred bright-pass back over the scene — a second brighter copy of
    /// the picture rather than light around it.
    #[test]
    fn settings_that_would_not_draw_a_glow_are_clamped() {
        let absurd = BloomSettings {
            enabled: true,
            threshold: -3.0,
            knee: 0.0,
            intensity: -1.0,
            passes: 0,
        }
        .sane();
        assert!(absurd.threshold >= 0.0);
        assert!(absurd.knee > 0.0);
        assert!(absurd.intensity >= 0.0);
        assert_eq!(absurd.passes, 1);

        let excessive = BloomSettings {
            passes: 500,
            knee: 9.0,
            ..BloomSettings::default()
        }
        .sane();
        assert_eq!(excessive.passes, MAX_PASSES);
        assert!(excessive.knee <= 1.0);
    }

    /// The default glows on the colors this exists for.
    ///
    /// Neon on black is not over-bright — a mint stroke peaks around 0.9 on its
    /// brightest channel — so a threshold at or above one would leave the whole
    /// picture matte, which is the failure that looks like bloom is broken
    /// rather than off.
    #[test]
    fn the_default_threshold_is_below_an_ordinary_bright_color() {
        let settings = BloomSettings::default();
        assert!(settings.threshold < 0.9, "{settings:?}");
        assert!(settings.passes >= 1);
        assert!(settings.intensity > 0.0);
    }

    #[test]
    fn neutral_post_process_is_a_real_passthrough() {
        assert!(!PostProcessSettings::default().is_active());
        assert!(
            PostProcessSettings {
                tone_mapping: ToneMapping::Aces,
                ..PostProcessSettings::default()
            }
            .is_active()
        );
    }

    /// A depth of field is post-processing of its own: a scene that wants
    /// nothing but the band in focus still goes through the composite, and one
    /// that turns it on with no blur does not.
    #[test]
    fn a_depth_of_field_that_blurs_is_post_processing() {
        let blurred = PostProcessSettings {
            depth_of_field: DepthOfFieldSettings {
                enabled: true,
                ..DepthOfFieldSettings::default()
            },
            ..PostProcessSettings::default()
        };
        assert!(blurred.is_active());
        let unblurred = PostProcessSettings {
            depth_of_field: DepthOfFieldSettings {
                enabled: true,
                blur: 0.0,
                ..DepthOfFieldSettings::default()
            },
            ..PostProcessSettings::default()
        };
        assert!(!unblurred.is_active());
        let smeared = DepthOfFieldSettings {
            enabled: true,
            blur: 1.0,
            falloff: 0.0,
            ..DepthOfFieldSettings::default()
        }
        .sane();
        assert!(smeared.blur <= DepthOfFieldSettings::MAX_BLUR);
        assert!(smeared.falloff > 0.0);
    }
}
