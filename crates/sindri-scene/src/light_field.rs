//! What a script can ask about the 2D lighting: can this see that, how bright is
//! it here.
//!
//! The renderer draws light from shadow maps built for the lights nearest the
//! camera. A game's rules cannot depend on the camera, so this is built from
//! every light and every occluder in the world and answers by geometry: a
//! point is lit by a light when no occluder edge crosses the line between
//! them, and by how much when the light's radius and falloff say so. It is the
//! light's centre that is asked, so a soft penumbra is a hard edge here.

use sindri_core::{ComponentSchemaRegistry, World};
use sindri_render::Light2d;

use crate::extract::SceneExtractError;
use crate::extract::lighting_2d::{gather_edges, gather_lights};

/// Every light and occluder in a scene with 2D lighting on.
#[derive(Clone, Debug, Default)]
pub struct LightField2d {
    ambient: [f32; 3],
    lights: Vec<Light2d>,
    edges: Vec<[[f32; 2]; 2]>,
}

impl LightField2d {
    /// The scene's light field, or `None` when its environment has 2D
    /// lighting off. A light or occluder that cannot be used is left out, as a
    /// tolerant frame leaves it out of the picture.
    ///
    /// # Errors
    /// A component that cannot be read from the world.
    pub fn gather(
        world: &World,
        components: &ComponentSchemaRegistry,
    ) -> Result<Option<Self>, SceneExtractError> {
        let Ok(Some(environment)) = crate::environment_of(world) else {
            return Ok(None);
        };
        if !environment.lighting_2d.enabled {
            return Ok(None);
        }
        let (lights, _) = gather_lights(components, world)?;
        let edges = if lights.iter().any(|light| light.shadow.is_some()) {
            gather_edges(components, world)?.0
        } else {
            Vec::new()
        };
        Ok(Some(Self {
            ambient: environment.lighting_2d.ambient,
            lights,
            edges,
        }))
    }

    /// Whether the straight line from `from` to `to` crosses no occluder edge.
    #[must_use]
    pub fn visible(&self, from: [f32; 2], to: [f32; 2]) -> bool {
        !self.edges.iter().any(|edge| crosses(from, to, *edge))
    }

    /// How bright it is at `point`, the luminance of the light reaching it:
    /// 0 in the dark, 1 where a white light is at full strength, and more where
    /// lights overlap. The ambient counts, a light that casts shadows is blocked
    /// by occluders, and one that does not shines through them.
    #[must_use]
    pub fn level(&self, point: [f32; 2]) -> f32 {
        let mut total = self.ambient;
        for light in &self.lights {
            let distance = (light.position[0] - point[0]).hypot(light.position[1] - point[1]);
            if distance >= light.radius {
                continue;
            }
            let amount = (1.0 - distance / light.radius).max(0.0).powf(light.falloff);
            if amount <= 0.0 || (light.shadow.is_some() && !self.visible(light.position, point)) {
                continue;
            }
            for (channel, color) in total.iter_mut().zip(light.color) {
                *channel += color * amount;
            }
        }
        0.2126 * total[0] + 0.7152 * total[1] + 0.0722 * total[2]
    }
}

/// Whether the segment `p` to `q` properly crosses `edge`. Touching an end, or
/// lying along it, does not: a thing standing on an edge is not behind it.
fn crosses(p: [f32; 2], q: [f32; 2], [a, b]: [[f32; 2]; 2]) -> bool {
    let side = |from: [f32; 2], to: [f32; 2], point: [f32; 2]| {
        (to[0] - from[0]) * (point[1] - from[1]) - (to[1] - from[1]) * (point[0] - from[0])
    };
    let (d1, d2) = (side(a, b, p), side(a, b, q));
    let (d3, d4) = (side(p, q, a), side(p, q, b));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_render::Light2dShadow;
    use std::sync::Arc;

    fn field(shadows: bool) -> LightField2d {
        LightField2d {
            ambient: [0.0; 3],
            lights: vec![Light2d {
                position: [0.0, 0.0],
                height: 1.0,
                radius: 6.0,
                falloff: 1.0,
                color: [1.0; 3],
                shadow: shadows.then(|| Light2dShadow {
                    reach: 0.0,
                    source_radius: 0.0,
                    map: Arc::from([]),
                }),
            }],
            edges: vec![[[2.0, -1.0], [2.0, 1.0]]],
        }
    }

    #[test]
    fn an_edge_between_two_points_hides_one_from_the_other() {
        let field = field(true);
        assert!(!field.visible([0.0, 0.0], [4.0, 0.0]));
        assert!(field.visible([0.0, 0.0], [1.5, 0.0]));
        assert!(field.visible([0.0, 0.0], [4.0, 3.0]), "past the edge's end");
        assert!(field.visible([2.0, 0.0], [4.0, 0.0]), "from on the edge");
    }

    #[test]
    fn a_shadow_casting_light_is_blocked_and_one_without_is_not() {
        let dark = field(true).level([4.0, 0.0]);
        let through = field(false).level([4.0, 0.0]);
        assert!(dark.abs() < 1.0e-6, "{dark}");
        assert!((through - (1.0 - 4.0 / 6.0)).abs() < 1.0e-5, "{through}");
        assert!(field(true).level([1.0, 0.0]) > 0.8, "before the edge");
        assert!(
            field(true).level([7.0, 0.0]).abs() < 1.0e-6,
            "past the radius"
        );
    }
}
