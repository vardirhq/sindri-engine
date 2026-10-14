//! The light the studio shines on the surface: a `sindri.light2d` and the
//! scene's ambient, held as the very values a scene would hold, so a light
//! that looks right here can be written into a scene unchanged.

use sindri_scene::Light2dComponent;

/// The studio's light and where it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct StudioLight {
    /// Exactly the component a scene's light carries.
    pub light: Light2dComponent,
    /// Where it stands in the plane, in world units from the surface's
    /// center.
    pub position: [f32; 2],
    /// The scene's `lighting_2d.ambient`: what the dark is.
    pub ambient: [f32; 3],
    /// How far round the surface a circling light has gone, in radians.
    pub circle: f32,
}

impl StudioLight {
    /// A light suited to a surface `size` world units across: low and to the
    /// upper left, reaching most of the way across, so relief shows
    /// the moment the studio opens.
    pub fn for_surface(size: [f32; 2]) -> Self {
        let reach = size[0].max(size[1]).max(f32::EPSILON);
        Self {
            light: Light2dComponent {
                color: [1.0, 0.86, 0.66, 1.0],
                intensity: 1.8,
                radius: reach * 1.1,
                falloff: 1.2,
                height: reach * 0.08,
                ..Light2dComponent::default()
            },
            position: [-size[0] * 0.3, size[1] * 0.25],
            ambient: [0.1, 0.1, 0.14],
            circle: 2.3,
        }
    }

    /// Moves a circling light on by `seconds`, keeping its distance from the
    /// center, or a fifth of the radius when it is sitting on it.
    pub fn circle_on(&mut self, seconds: f32) {
        let distance = self.position[0]
            .hypot(self.position[1])
            .max(self.light.radius * 0.2);
        self.circle = (self.circle + seconds * 0.9) % std::f32::consts::TAU;
        self.position = [distance * self.circle.cos(), distance * self.circle.sin()];
    }

    /// Puts the light where the pointer is, and remembers that angle so a
    /// circle started afterwards carries on from there.
    pub fn place(&mut self, position: [f32; 2]) {
        if position.iter().all(|value| value.is_finite()) {
            self.position = position;
            self.circle = position[1].atan2(position[0]);
        }
    }

    /// The light as a scene's JSON payload, to paste into a scene.
    pub fn component_json(&self) -> String {
        serde_json::to_string_pretty(&self.light)
            .expect("a light of numbers and flags always serializes")
    }
}
