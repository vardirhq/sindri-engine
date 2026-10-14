//! `sindri.particles`: an emitter that keeps throwing flecks while it plays.
//!
//! A burst is one throw a script makes. An emitter is a stream the scene
//! authors: sparkle round a coin, dust at a runner's feet, smoke from a
//! chimney. Its particles are flecks in the same pool, so a thousand of them
//! cost what a thousand flecks cost, and they come from the pool's own random
//! stream, so a seeded run stays the same run however many there are.

use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;

/// Where in the emitter's shape a particle starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmissionShape {
    /// The emitter's own position.
    #[default]
    Point,
    /// Anywhere inside a circle `shape_size[0]` across.
    Circle,
    /// Anywhere inside a `shape_size` rectangle.
    Box,
    /// On the edge of a circle `shape_size[0]` across.
    Ring,
}

impl EmissionShape {
    pub const NAMES: [&'static str; 4] = ["point", "circle", "box", "ring"];
}

/// Whether particles follow their emitter once they are out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticleSpace {
    /// They stay where they were thrown: a trail.
    #[default]
    World,
    /// They move with the emitter: a halo.
    Local,
}

impl ParticleSpace {
    pub const NAMES: [&'static str; 2] = ["world", "local"];
}

/// A continuous particle emitter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ParticlesComponent {
    /// Whether it is emitting. A script plays and stops it.
    pub playing: bool,
    /// The image every particle draws: one batch for the whole emitter.
    pub texture: String,
    /// Particles a second while playing.
    pub rate: f32,
    /// The most of this emitter's particles alive at once.
    pub max: u32,
    /// Seconds a particle lives, varied by `lifetime_spread` of itself.
    pub lifetime: f32,
    pub lifetime_spread: f32,
    /// World units a second, varied by `speed_spread` of itself.
    pub speed: f32,
    pub speed_spread: f32,
    /// The direction they leave in, in degrees: 0 is right, 90 is up.
    pub direction: f32,
    /// How wide a cone round `direction` they leave in, in degrees; 360 is
    /// every way.
    pub spread: f32,
    pub shape: EmissionShape,
    /// Where the shape sits from the entity, in world units: a runner's feet
    /// rather than its middle.
    pub offset: [f32; 2],
    /// The shape's width and height; a circle or ring reads the width.
    pub shape_size: [f32; 2],
    /// Added to every particle's velocity each second.
    pub gravity: [f32; 2],
    /// How much of its speed a particle loses each second.
    pub drag: f32,
    /// Its size when thrown and when it dies, in world units.
    pub size: f32,
    pub size_end: f32,
    /// Colors evenly spaced over a particle's life, blended between: two is a
    /// fade, three a flare that cools.
    pub colors: Vec<[f32; 4]>,
    /// Whether each starts turned a random way.
    pub random_rotation: bool,
    /// Degrees a second it turns.
    pub spin: f32,
    pub space: ParticleSpace,
    pub layer: i32,
}

impl Default for ParticlesComponent {
    fn default() -> Self {
        Self {
            playing: true,
            texture: String::new(),
            rate: 10.0,
            max: 200,
            lifetime: 1.0,
            lifetime_spread: 0.2,
            speed: 2.0,
            speed_spread: 0.3,
            direction: 90.0,
            spread: 30.0,
            shape: EmissionShape::Point,
            offset: [0.0, 0.0],
            shape_size: [1.0, 1.0],
            gravity: [0.0, 0.0],
            drag: 0.0,
            size: 0.1,
            size_end: 0.1,
            colors: vec![[1.0, 1.0, 1.0, 1.0], [1.0, 1.0, 1.0, 0.0]],
            random_rotation: false,
            spin: 0.0,
            space: ParticleSpace::World,
            layer: 0,
        }
    }
}

impl SceneComponent for ParticlesComponent {
    const TYPE_NAME: &'static str = "sindri.particles";
}

/// Why an emitter was refused, naming the field.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("`{field}` {expected}")]
pub struct ParticlesError {
    pub field: &'static str,
    pub expected: &'static str,
}

impl ParticlesComponent {
    /// The emitter, or the first field that cannot be used.
    pub fn validate(&self) -> Result<(), ParticlesError> {
        let fail = |field, expected| Err(ParticlesError { field, expected });
        let at_least_zero = [
            ("rate", self.rate),
            ("lifetime_spread", self.lifetime_spread),
            ("speed", self.speed),
            ("speed_spread", self.speed_spread),
            ("drag", self.drag),
            ("size", self.size),
            ("size_end", self.size_end),
            ("shape_size", self.shape_size[0]),
            ("shape_size", self.shape_size[1]),
        ];
        for (field, value) in at_least_zero {
            if !value.is_finite() || value < 0.0 {
                return fail(field, "must be a finite number of at least 0");
            }
        }
        if !self.lifetime.is_finite() || self.lifetime <= 0.0 {
            return fail("lifetime", "must be a finite number above 0");
        }
        let finite = [self.direction, self.spread, self.spin];
        let vectors = self.gravity.iter().chain(&self.offset);
        if !finite.iter().chain(vectors).all(|value| value.is_finite()) {
            return fail(
                "direction",
                "spread, spin, gravity and offset must be finite",
            );
        }
        if self.colors.iter().flatten().any(|value| !value.is_finite()) {
            return fail("colors", "must be finite numbers");
        }
        Ok(())
    }

    /// The color `through` of the way through a particle's life, 0 to 1.
    #[must_use]
    pub fn color_at(colors: &[[f32; 4]], through: f32) -> [f32; 4] {
        match colors {
            [] => [1.0; 4],
            [only] => *only,
            _ => {
                #[allow(clippy::cast_precision_loss)]
                let scaled = through.clamp(0.0, 1.0) * (colors.len() - 1) as f32;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let index = (scaled.floor() as usize).min(colors.len() - 2);
                #[allow(clippy::cast_precision_loss)]
                let t = scaled - index as f32;
                let [from, to] = [colors[index], colors[index + 1]];
                std::array::from_fn(|channel| {
                    (to[channel] - from[channel]).mul_add(t, from[channel])
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ParticlesComponent;

    #[test]
    fn a_ramp_blends_between_its_stops() {
        let ramp = [[1.0, 0.0, 0.0, 1.0], [0.0, 0.0, 1.0, 0.0]];
        let middle = ParticlesComponent::color_at(&ramp, 0.5);
        assert!((middle[0] - 0.5).abs() < 1.0e-6 && (middle[3] - 0.5).abs() < 1.0e-6);
        let end = ParticlesComponent::color_at(&ramp, 1.0);
        assert!((end[2] - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn a_three_stop_ramp_passes_through_its_middle() {
        let ramp = [[0.0; 4], [1.0; 4], [0.0; 4]];
        let middle = ParticlesComponent::color_at(&ramp, 0.5);
        assert!((middle[0] - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn a_nonsense_emitter_names_its_field() {
        let broken = ParticlesComponent {
            lifetime: 0.0,
            ..ParticlesComponent::default()
        };
        assert_eq!(broken.validate().unwrap_err().field, "lifetime");
        assert!(ParticlesComponent::default().validate().is_ok());
    }
}
