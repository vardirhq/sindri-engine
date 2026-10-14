//! What a normal map is made with: the settings, and the presets that are
//! only starting values for them.

use serde::{Deserialize, Serialize};

/// Where the height a normal map is the slope of comes from.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HeightSource {
    /// The picture's own brightness: bright reads as raised.
    ///
    /// A guess. Often a good one for stone and metal, and a poor one for a
    /// painted highlight or a dark rivet, which is why it can be inverted and
    /// why the result is always looked at lit.
    #[default]
    Luminance,
    /// A grayscale image laid out exactly like the picture, painted to say
    /// what is raised.
    HeightMap,
}

/// What a slope reads past the edge of the picture, or of its cell.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeMode {
    /// The edge texel repeats, so the border reads as flat outward.
    #[default]
    Clamp,
    /// The far side continues, for a texture that tiles.
    Wrap,
}

/// Which way green points.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GreenAxis {
    /// Green is up the world: the "OpenGL" convention, and what Sindri reads.
    #[default]
    Up,
    /// Green is down the picture: the "DirectX" convention, for a map that
    /// will be used somewhere that expects it.
    Down,
}

/// Everything that decides the map made from a picture.
///
/// Missing fields read as their defaults, so a settings file written before a
/// field existed still opens and makes the map it made.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct NormalMapSettings {
    pub method: HeightSource,
    /// How steep a given change of height is. 0 is flat.
    pub strength: f32,
    /// Radius in texels of the smoothing over the height. 0 keeps every
    /// texel's own height, which is what pixel art wants.
    pub blur: f32,
    /// How much of the unsmoothed relief is laid back over the smoothed one,
    /// 0 to 1. Only matters with a blur.
    pub detail: f32,
    /// Dark reads as raised instead.
    pub invert: bool,
    /// Transparent texels count as the lowest height, so a sprite's outline
    /// bevels toward its edge.
    pub alpha_height: bool,
    pub edges: EdgeMode,
    /// Whether a sliced sheet's cells are separate pictures, so no slope
    /// reads its neighbor in the sheet.
    pub cells: bool,
    pub green: GreenAxis,
}

impl Default for NormalMapSettings {
    fn default() -> Self {
        Self {
            method: HeightSource::Luminance,
            strength: 2.0,
            blur: 0.0,
            detail: 1.0,
            invert: false,
            alpha_height: true,
            edges: EdgeMode::Clamp,
            cells: true,
            green: GreenAxis::Up,
        }
    }
}

impl NormalMapSettings {
    /// The settings with every number inside the range it means anything in,
    /// and anything not finite put back to its default.
    #[must_use]
    pub fn sanitized(&self) -> Self {
        let defaults = Self::default();
        let finite = |value: f32, default: f32| if value.is_finite() { value } else { default };
        Self {
            strength: finite(self.strength, defaults.strength).clamp(0.0, 32.0),
            blur: finite(self.blur, defaults.blur).clamp(0.0, 16.0),
            detail: finite(self.detail, defaults.detail).clamp(0.0, 1.0),
            ..self.clone()
        }
    }
}

/// A starting point for a kind of material.
///
/// Only values: nothing looks at a picture and decides it is metal, and every
/// value stays editable after a preset is chosen. Each keeps the height
/// source and green axis already chosen, since those are about the files
/// rather than the material.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NormalMapPreset {
    /// Even, crisp relief: the defaults.
    General,
    /// Rivets, panel seams and dents: steep and sharp.
    Metal,
    /// Grain and plank edges: moderate, slightly softened.
    Wood,
    /// Cracks and irregular faces: strong, broad shapes with grain kept.
    Stone,
    /// Subtle folds: gentle and smooth.
    Fabric,
    /// Restrained relief for ground that tiles.
    Terrain,
}

impl NormalMapPreset {
    pub const ALL: [Self; 6] = [
        Self::General,
        Self::Metal,
        Self::Wood,
        Self::Stone,
        Self::Fabric,
        Self::Terrain,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Metal => "Metal",
            Self::Wood => "Wood",
            Self::Stone => "Stone",
            Self::Fabric => "Fabric",
            Self::Terrain => "Terrain",
        }
    }

    /// One line on what the preset is for.
    pub const fn description(self) -> &'static str {
        match self {
            Self::General => "Even, crisp relief. A good first look at anything.",
            Self::Metal => "Steep and sharp, for rivets, seams and dents.",
            Self::Wood => "Moderate and slightly soft, for grain and plank edges.",
            Self::Stone => "Strong broad shapes with the grain kept, for cracks and rock.",
            Self::Fabric => "Gentle and smooth, for folds.",
            Self::Terrain => "Restrained relief that tiles, for ground.",
        }
    }

    /// `current` with this preset's values laid over it.
    #[must_use]
    pub fn apply(self, current: &NormalMapSettings) -> NormalMapSettings {
        let (strength, blur, detail, edges) = match self {
            Self::General => (2.0, 0.0, 1.0, EdgeMode::Clamp),
            Self::Metal => (3.5, 0.0, 1.0, EdgeMode::Clamp),
            Self::Wood => (1.6, 1.0, 0.6, EdgeMode::Clamp),
            Self::Stone => (3.0, 1.5, 0.5, EdgeMode::Clamp),
            Self::Fabric => (1.2, 2.5, 0.25, EdgeMode::Clamp),
            Self::Terrain => (1.2, 1.0, 0.5, EdgeMode::Wrap),
        };
        NormalMapSettings {
            strength,
            blur,
            detail,
            edges,
            invert: false,
            ..current.clone()
        }
    }
}
