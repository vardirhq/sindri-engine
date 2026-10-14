//! How the studio is being looked at: which picture, which channel, what it is
//! compared with, and where the 3D camera stands. Editor state; none of it is
//! saved with the map.

/// The picture in the middle of the studio.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StudioMode {
    /// The sprite as it is, unlit.
    Texture,
    /// The map, or one channel of it.
    Normal,
    /// The sprite and the map together, under a light that follows a drag.
    #[default]
    Lit,
    /// The same lit surface seen from a camera that orbits it.
    Inspect,
}

impl StudioMode {
    pub const ALL: [Self; 4] = [Self::Texture, Self::Normal, Self::Lit, Self::Inspect];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Texture => "Texture",
            Self::Normal => "Normal map",
            Self::Lit => "Lit",
            Self::Inspect => "3D",
        }
    }

    /// Whether the picture is drawn by the game's renderer under a light.
    pub const fn is_lit(self) -> bool {
        matches!(self, Self::Lit | Self::Inspect)
    }
}

/// Which part of the map the Normal mode shows.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Channel {
    #[default]
    All,
    /// Red: how far the surface faces right.
    Red,
    /// Green: how far it faces up the world.
    Green,
    /// Blue: how squarely it faces out of the picture.
    Blue,
    /// The height the map is the slope of, before it became directions.
    Height,
}

impl Channel {
    pub const ALL: [Self; 5] = [Self::All, Self::Red, Self::Green, Self::Blue, Self::Height];

    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "RGB",
            Self::Red => "R",
            Self::Green => "G",
            Self::Blue => "B",
            Self::Height => "Height",
        }
    }
}

/// What the other side of the split shows, when the view is split.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Compare {
    /// No split: the whole canvas is the normal-mapped picture.
    #[default]
    Off,
    /// The sprite lit by distance alone, as a sprite with no normal map is:
    /// what the map adds, and nothing else.
    Flat,
    /// The sprite with no lighting at all: what it looked like before.
    Unlit,
}

impl Compare {
    pub const ALL: [Self; 3] = [Self::Off, Self::Flat, Self::Unlit];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "No split",
            Self::Flat => "Split: without map",
            Self::Unlit => "Split: unlit",
        }
    }
}

/// Where the 3D camera stands, relative to the surface it circles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    /// Yaw and pitch in radians, from the viewer camera's own three-quarter
    /// view of the surface.
    pub angles: [f32; 2],
    /// How much nearer than the starting distance, as the Scene view zooms.
    pub zoom: f32,
}

impl Default for Orbit {
    fn default() -> Self {
        Self {
            angles: [0.0, 0.0],
            zoom: 1.0,
        }
    }
}

/// The furthest the 3D camera tilts, short of edge-on: a surface seen exactly
/// edge-on is a line, which shows nothing about its lighting.
pub const PITCH_LIMIT: f32 = 1.4;

impl Orbit {
    /// Turns the camera by a drag of `delta` points.
    pub fn turn(&mut self, delta: [f32; 2]) {
        self.angles[0] -= delta[0] * 0.008;
        self.angles[1] = (self.angles[1] + delta[1] * 0.008).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    /// Zooms by a scroll of `lines`, within the Scene view's range.
    pub fn zoom_by(&mut self, lines: f32) {
        self.zoom = (self.zoom * 1.1_f32.powf(lines)).clamp(0.25, 8.0);
    }
}

impl StudioView {
    /// Zooms the flat modes by a scroll of `lines`.
    pub fn zoom_by(&mut self, lines: f32) {
        self.zoom = (self.zoom * 1.1_f32.powf(lines)).clamp(0.1, 64.0);
    }

    /// Back to the surface fitted and the camera where it started.
    pub fn reset_camera(&mut self) {
        self.zoom = 1.0;
        self.pan = [0.0, 0.0];
        self.orbit = Orbit::default();
    }
}

/// Everything about how the studio is being looked at.
#[derive(Clone, Debug, PartialEq)]
pub struct StudioView {
    pub mode: StudioMode,
    pub channel: Channel,
    pub compare: Compare,
    /// Where the split falls across the canvas, 0 to 1.
    pub split: f32,
    pub orbit: Orbit,
    /// How far the flat modes are zoomed in, 1 being the surface fitted.
    pub zoom: f32,
    /// How far the flat modes are panned, in fractions of the framed
    /// half-height, as a Scene view pans.
    pub pan: [f32; 2],
    /// Whether the lit modes read the map at all; off draws what a sprite
    /// with no map looks like.
    pub normals_on: bool,
    /// Whether the light circles the surface by itself.
    pub circling: bool,
}

impl Default for StudioView {
    fn default() -> Self {
        Self {
            mode: StudioMode::Lit,
            channel: Channel::All,
            compare: Compare::Off,
            split: 0.5,
            orbit: Orbit::default(),
            zoom: 1.0,
            pan: [0.0, 0.0],
            normals_on: true,
            circling: false,
        }
    }
}
