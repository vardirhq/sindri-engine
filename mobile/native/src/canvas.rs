//! Where the phone looks: a flat camera over the scene that touch pans and
//! pinches, independent of the scene's own camera.

use sindri_core::World;
use sindri_scene::{CameraView, SceneExtractor, ViewCamera, WorldProjection};

/// Pinching never zooms past these, in world units of half the screen height.
const CLOSEST: f32 = 0.5;
const FARTHEST: f32 = 500.0;

/// A 2D view of the scene: the world point at the middle of the screen, and
/// half the world height the screen shows.
///
/// Drawn through the engine's own flat viewer camera (`WorldProjection::Flat`),
/// so the matrix a frame is drawn with and the one a touch is picked with are
/// one matrix, asked of the extractor, never rebuilt here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Canvas {
    pub center: [f32; 2],
    pub half_height: f32,
    /// What the engine's flat camera frames at a distance scale of one; how a
    /// half-height is turned into the scale the camera is asked for.
    unit_half_height: f32,
}

impl Canvas {
    /// The canvas the scene opens on: where its own camera looks, as much as it
    /// shows, or the origin at a modest size for a scene with no camera.
    ///
    /// # Errors
    /// The engine's flat camera could not be resolved, which only a broken
    /// build does.
    pub fn framing(world: &World, extractor: &SceneExtractor) -> Result<Self, String> {
        let unit = extractor
            .world_camera(
                world,
                CameraView {
                    projection: WorldProjection::Flat,
                    ..CameraView::default()
                },
            )
            .map_err(|error| error.to_string())?
            .ok_or("the flat viewer camera did not resolve")?
            .framed_half_height;
        let authored = extractor
            .world_camera(world, CameraView::default())
            .ok()
            .flatten();
        let (center, half_height) = authored.map_or(([0.0, 0.0], 8.0), |camera| {
            let eye = camera.view.inverse().w_axis;
            ([eye.x, eye.y], camera.framed_half_height)
        });
        Ok(Self {
            center,
            half_height: half_height.clamp(CLOSEST, FARTHEST),
            unit_half_height: unit,
        })
    }

    /// The camera adjustment a frame and a pick are made through.
    pub fn view(&self) -> CameraView {
        CameraView {
            projection: WorldProjection::Flat,
            distance_scale: self.half_height / self.unit_half_height,
            // The engine's pan shifts the picture, so the camera looks at the
            // opposite of it (`panned_shift` in `sindri-scene`).
            pan: -glam::Vec2::new(self.center[0], self.center[1]) / self.half_height,
            ..CameraView::default()
        }
    }

    /// The resolved camera for a viewport `size` pixels across.
    ///
    /// # Errors
    /// As [`SceneExtractor::world_camera_for_viewport`].
    pub fn camera(
        &self,
        world: &World,
        extractor: &SceneExtractor,
        size: [f32; 2],
    ) -> Result<ViewCamera, String> {
        extractor
            .world_camera_for_viewport(world, size[0] / size[1].max(1.0), self.view())
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "the canvas camera did not resolve".to_owned())
    }

    /// World units one screen pixel covers, for a viewport `height` pixels tall.
    pub fn per_pixel(&self, height: f32) -> f32 {
        self.half_height * 2.0 / height.max(1.0)
    }

    /// The world point under a pixel: right is +x, and down the screen is -y.
    pub fn world_point(&self, pixel: [f32; 2], size: [f32; 2]) -> [f32; 2] {
        let per_pixel = self.per_pixel(size[1]);
        [
            self.center[0] + (pixel[0] - size[0] * 0.5) * per_pixel,
            self.center[1] - (pixel[1] - size[1] * 0.5) * per_pixel,
        ]
    }

    /// Follows a finger that moved `delta` pixels: the world moves with it.
    pub fn pan(&mut self, delta: [f32; 2], size: [f32; 2]) {
        let per_pixel = self.per_pixel(size[1]);
        self.center[0] -= delta[0] * per_pixel;
        self.center[1] += delta[1] * per_pixel;
    }

    /// Pinches by `factor` (above one zooms in) about the pixel `focus`, which
    /// stays over the same world point.
    pub fn zoom(&mut self, factor: f32, focus: [f32; 2], size: [f32; 2]) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let before = self.world_point(focus, size);
        self.half_height = (self.half_height / factor).clamp(CLOSEST, FARTHEST);
        let after = self.world_point(focus, size);
        self.center[0] += before[0] - after[0];
        self.center[1] += before[1] - after[1];
    }
}
