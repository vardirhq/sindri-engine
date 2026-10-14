//! The studio's GPU side: the picture, map and channel uploaded as a game
//! uploads textures, and the studio's world drawn by the same
//! `RuntimeViewport::render` the Scene and Game views draw with.

use std::path::PathBuf;

use eframe::egui;
use sindri_scene::{CameraView, UiCanvas, ViewCamera};

use super::super::scene_io::SceneSource;
use super::super::viewport::{RuntimeViewport, SceneRenderers};
use super::scene::{CHANNEL, Look, NORMAL, SOURCE, StudioScene};
use crate::normal_studio::{Channel, NormalStudio, StudioMode};
use crate::profiler::Profiler;
use crate::textures::SceneTextures;

/// One of the two targets: the canvas, and the other side of a split.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Pane {
    Main,
    Other,
}

pub(super) struct StudioStage {
    scene: StudioScene,
    textures: SceneTextures,
    /// The picture and map revision last put on the GPU.
    uploaded: Option<(PathBuf, u64)>,
    /// The channel picture last put on the GPU, for that revision.
    channel: Option<(Channel, u64)>,
    main: RuntimeViewport,
    other: RuntimeViewport,
    /// The studio's draws are not Play's, so they are timed apart from it.
    profiler: Profiler,
    still: (sindri_scene::SpriteAnimations, sindri_scene::Effects2d),
}

impl StudioStage {
    pub(super) fn new(render_state: &eframe::egui_wgpu::RenderState) -> Self {
        Self {
            scene: StudioScene::new(),
            textures: SceneTextures::for_scene(&render_state.device, &render_state.queue, None),
            uploaded: None,
            channel: None,
            main: RuntimeViewport::new(render_state.clone(), "Normal Map Studio"),
            other: RuntimeViewport::new(render_state.clone(), "Normal Map Studio split"),
            profiler: Profiler::default(),
            still: Default::default(),
        }
    }

    /// Puts the studio's picture, map and channel on the GPU when they
    /// changed, and its light and surface into the world.
    pub(super) fn sync(
        &mut self,
        render_state: &eframe::egui_wgpu::RenderState,
        studio: &NormalStudio,
    ) -> Result<(), String> {
        let Some(picture) = studio.picture() else {
            return Ok(());
        };
        let size = (picture.width, picture.height);
        let (device, queue) = (&render_state.device, &render_state.queue);
        let current = (studio.source().to_path_buf(), studio.revision());
        if self.uploaded.as_ref() != Some(&current) {
            self.textures
                .provide(device, queue, SOURCE, size, &picture.rgba)
                .map_err(|error| error.to_string())?;
            self.textures
                .provide(device, queue, NORMAL, size, studio.map())
                .map_err(|error| error.to_string())?;
            self.uploaded = Some(current);
            self.channel = None;
        }
        let channel = (studio.view.channel, studio.revision());
        if studio.view.mode == StudioMode::Normal && self.channel != Some(channel) {
            let pixels = studio.channel_picture(channel.0);
            self.textures
                .provide(device, queue, CHANNEL, size, &pixels)
                .map_err(|error| error.to_string())?;
            self.channel = Some(channel);
        }
        self.scene.follow(studio);
        Ok(())
    }

    /// Draws the surface as `look` into `pane`, `size` physical pixels, and
    /// answers the picture egui draws of it.
    pub(super) fn draw(
        &mut self,
        renderers: &mut SceneRenderers,
        pane: Pane,
        look: Look,
        size: (u32, u32),
        camera: CameraView,
    ) -> Result<egui::TextureId, String> {
        self.scene.wear(look);
        let viewport = match pane {
            Pane::Main => &mut self.main,
            Pane::Other => &mut self.other,
        };
        viewport.render(
            renderers,
            SceneSource {
                scene: &self.scene.extractor,
                world: &self.scene.world,
                animations: &self.still.0,
                effects: &self.still.1,
                textures: &self.textures,
                studio_lighting: false,
            },
            size,
            camera,
            UiCanvas::OnViewport,
            &mut self.profiler,
        )?;
        Ok(viewport.texture_id())
    }

    pub(super) fn view_camera(&self, aspect: f32, camera: CameraView) -> Option<ViewCamera> {
        self.scene.view_camera(aspect, camera)
    }
}
