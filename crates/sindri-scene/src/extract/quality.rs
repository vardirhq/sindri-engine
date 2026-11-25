//! How far a frame draws and how detailed its shadows are: a graphics setting.
//!
//! A scene authors how far its cameras see and how far its shadows reach for
//! the device it was made on. A more powerful machine can draw further and
//! sharper; a phone can draw less and keep its frame rate. The quality scales
//! what was authored rather than replacing it, so a scene keeps its own
//! proportions at every level.

use super::SceneExtractor;

/// A graphics quality level, from cheapest to richest. `High` draws a scene
/// exactly as it was authored.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum GraphicsQuality {
    Low,
    Medium,
    #[default]
    High,
    Ultra,
}

/// The shared-board entry a host keeps the level a frame is drawn at in, as
/// the level's place in [`GraphicsQuality::ALL`], so scripts can read it.
pub const QUALITY_BOARD: &str = "sindri.graphics.quality";
/// The shared-board entry a script asks for a level in: the level's place in
/// [`GraphicsQuality::ALL`] plus one, and zero once the host has taken it.
pub const QUALITY_REQUEST_BOARD: &str = "sindri.graphics.request";

impl GraphicsQuality {
    /// The level's place in [`Self::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Every level, cheapest first, as a settings menu offers them.
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Ultra];

    /// The level's name, as scripts and settings files write it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Ultra => "ultra",
        }
    }

    /// The level a name means, ignoring case.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|level| level.name().eq_ignore_ascii_case(name))
    }

    /// How far a perspective camera draws, as a multiple of its authored far
    /// plane. Whatever lies beyond it is neither drawn nor culled in.
    #[must_use]
    pub const fn view_distance(self) -> f32 {
        match self {
            Self::Low => 0.5,
            Self::Medium => 0.75,
            Self::High => 1.0,
            Self::Ultra => 1.5,
        }
    }

    /// How far shadows reach, as a multiple of the authored distance.
    #[must_use]
    pub const fn shadow_distance(self) -> f32 {
        self.view_distance()
    }

    /// The shadow map's resolution, as a multiple of the authored size. The
    /// renderer keeps it within its own bounds.
    #[must_use]
    pub const fn shadow_resolution(self) -> f32 {
        match self {
            Self::Low => 0.5,
            Self::Medium | Self::High => 1.0,
            Self::Ultra => 2.0,
        }
    }

    /// The next cheaper level, if there is one.
    #[must_use]
    pub const fn lower(self) -> Option<Self> {
        match self {
            Self::Low => None,
            Self::Medium => Some(Self::Low),
            Self::High => Some(Self::Medium),
            Self::Ultra => Some(Self::High),
        }
    }
}

impl SceneExtractor {
    /// Chooses how far frames draw and how detailed their shadows are.
    pub const fn set_graphics_quality(&mut self, quality: GraphicsQuality) {
        self.graphics_quality = quality;
    }

    /// The graphics quality frames are drawn at now.
    #[must_use]
    pub const fn graphics_quality(&self) -> GraphicsQuality {
        self.graphics_quality
    }

    /// An environment's shadow settings at this quality.
    pub(super) fn scaled_shadows(
        &self,
        shadows: sindri_render::ShadowSettings,
    ) -> sindri_render::ShadowSettings {
        let quality = self.graphics_quality;
        // Rounded to a power of two, as shadow maps are sized.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let map_size = ((shadows.map_size as f32 * quality.shadow_resolution()).max(1.0) as u32)
            .next_power_of_two();
        sindri_render::ShadowSettings {
            distance: shadows.distance * quality.shadow_distance(),
            map_size,
            ..shadows
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GraphicsQuality;

    #[test]
    fn names_round_trip_and_high_is_as_authored() {
        for level in GraphicsQuality::ALL {
            assert_eq!(GraphicsQuality::named(level.name()), Some(level));
        }
        assert_eq!(
            GraphicsQuality::named("ULTRA"),
            Some(GraphicsQuality::Ultra)
        );
        assert_eq!(GraphicsQuality::named("max"), None);
        let high = GraphicsQuality::High;
        assert!((high.view_distance() - 1.0).abs() < f32::EPSILON);
        assert!((high.shadow_resolution() - 1.0).abs() < f32::EPSILON);
        assert_eq!(GraphicsQuality::Low.lower(), None);
    }

    #[test]
    fn shadows_reach_and_sharpen_with_the_quality() {
        let mut extractor = super::SceneExtractor::new().unwrap();
        let authored = sindri_render::ShadowSettings {
            enabled: true,
            distance: 40.0,
            map_size: 1024,
            bias: 0.002,
        };
        extractor.set_graphics_quality(GraphicsQuality::Low);
        let low = extractor.scaled_shadows(authored);
        assert!((low.distance - 20.0).abs() < 1.0e-4 && low.map_size == 512);
        extractor.set_graphics_quality(GraphicsQuality::Ultra);
        let ultra = extractor.scaled_shadows(authored);
        assert!((ultra.distance - 60.0).abs() < 1.0e-4 && ultra.map_size == 2048);
    }
}
