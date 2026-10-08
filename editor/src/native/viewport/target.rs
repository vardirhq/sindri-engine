//! The GPU side of a view: the pipelines every view draws with, the target
//! one draws into, and drawing a prepared frame there.

use std::time::Instant;

use eframe::{egui, wgpu};
use sindri_render::{
    Bloom, FrameRenderers, FrameTarget, GlyphRenderer, Lighting, ShapeRenderer,
    SpriteBatchRenderer, TextRenderer, TexturedCubeRenderer, Viewport, ViewportTarget,
    encode_lit_frame, encode_prepared_frame,
};
use sindri_scene::{CameraView, EnvironmentComponent, SceneRuntime, UiCanvas, UiTextSizes};

use super::super::scene_io::SceneSource;
use super::super::{
    INITIAL_VIEWPORT_HEIGHT, INITIAL_VIEWPORT_WIDTH, WorkspaceTab, scene_board_view,
};
use crate::profiler::{Phase, Profiler};

/// The GPU pipelines every viewport draws with.
///
/// Held once rather than per viewport: a pipeline does not depend on which
/// camera is looking, and two viewports that each built their own would pay
/// twice for the same thing. The textures used to live here too, handed over by
/// the cube example; they belong to the open scene, which is where they are now.
pub(in crate::native) struct SceneRenderers {
    pub(in crate::native) sprites: SpriteBatchRenderer,
    pub(in crate::native) text: TextRenderer,
    pub(in crate::native) glyphs: GlyphRenderer,
    pub(in crate::native) shapes: ShapeRenderer,
}

impl SceneRenderers {
    pub(in crate::native) fn new(render_state: &eframe::egui_wgpu::RenderState) -> Self {
        Self {
            sprites: SpriteBatchRenderer::new(&render_state.device, ViewportTarget::FORMAT),
            text: TextRenderer::new(),
            glyphs: GlyphRenderer::new(&render_state.device, ViewportTarget::FORMAT),
            shapes: ShapeRenderer::new(&render_state.device, ViewportTarget::FORMAT),
        }
    }
}

pub(in crate::native) struct RuntimeViewport {
    render_state: eframe::egui_wgpu::RenderState,
    target: ViewportTarget,
    pub(super) texture_id: egui::TextureId,
    bloom: Bloom,
    /// Each view's own: the cube renderer keeps a view's light, shadows and
    /// meshes between frames, and two views drawing through one rebuilt
    /// each other's every frame — a voxel world's Game view recorded in
    /// 8.7 ms what it records alone in 3.5.
    cube: TexturedCubeRenderer,
}

impl RuntimeViewport {
    pub(in crate::native) fn new(
        render_state: eframe::egui_wgpu::RenderState,
        label: &str,
    ) -> Self {
        let target = ViewportTarget::new(
            &render_state.device,
            label,
            INITIAL_VIEWPORT_WIDTH,
            INITIAL_VIEWPORT_HEIGHT,
        );
        let texture_id = render_state.renderer.write().register_native_texture(
            &render_state.device,
            target.sampled(),
            wgpu::FilterMode::Linear,
        );
        let mut bloom = Bloom::new(&render_state.device, ViewportTarget::FORMAT);
        bloom.resize(
            &render_state.device,
            INITIAL_VIEWPORT_WIDTH,
            INITIAL_VIEWPORT_HEIGHT,
        );
        let cube = TexturedCubeRenderer::new(&render_state.device, ViewportTarget::FORMAT);
        Self {
            render_state,
            target,
            texture_id,
            bloom,
            cube,
        }
    }

    /// The shape of what this viewport draws into.
    ///
    /// Read from the target rather than from whatever rect was last laid out,
    /// so it answers the same thing whether or not this view was drawn in the
    /// current layout — a Scene view alone in the window still knows what the
    /// Game view frames.
    pub(in crate::native) fn aspect(&self) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let (width, height) = (self.target.width() as f32, self.target.height() as f32);
        if height <= 0.0 { 1.0 } else { width / height }
    }

    pub(super) fn render(
        &mut self,
        renderers: &mut SceneRenderers,
        source: SceneSource<'_>,
        size: (u32, u32),
        camera: CameraView,
        canvas: UiCanvas,
        profiler: &mut Profiler,
    ) -> Result<UiTextSizes, String> {
        let began = Instant::now();
        self.resize(size.0, size.1);
        // Text that fits its words is measured by the same renderer that
        // draws it, so the view shows the size the game will.
        let text_sizes = sindri_scene::measure_ui_text(
            source.world,
            source.scene.components(),
            &mut renderers.text,
        )
        .map_err(|error| error.to_string())?;
        let prepared = source
            .scene
            .extract_animated(
                source.world,
                Viewport::new(self.target.width(), self.target.height()),
                camera,
                source.textures.bindings(),
                SceneRuntime::default()
                    .with_animations(source.animations)
                    .with_effects(source.effects)
                    .with_tile_sets(source.textures.tile_sets())
                    .with_seconds(super::super::animated::seconds())
                    .with_canvas(canvas)
                    .with_text_sizes(&text_sizes),
            )
            .map_err(|error| error.to_string())?;
        profiler.add(Phase::Extraction, began.elapsed());
        // The editor's own frame — the panels egui painted last time — may
        // still be on the GPU, and a submit behind it blocks until it is done.
        // Waited for first when measuring, so that wait is timed as the GPU's
        // rather than as this view's encoding.
        if profiler.waits_for_gpu() {
            let waiting = Instant::now();
            self.render_state
                .device
                .poll(wgpu::PollType::wait_indefinitely())
                .map_err(|error| error.to_string())?;
            profiler.add(Phase::Gpu, waiting.elapsed());
        }
        let extracted = Instant::now();
        let mut encoder =
            self.render_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Sindri editor runtime viewport encoder"),
                });
        let environment = self.light(source, camera)?;
        let frame_renderers = FrameRenderers {
            cube: &mut self.cube,
            sprites: &mut renderers.sprites,
            text: &mut renderers.text,
            glyphs: &mut renderers.glyphs,
            shapes: &mut renderers.shapes,
            textures: source.textures.registry(),
        };
        let target = FrameTarget {
            color: self.target.attachment(),
            depth: self.target.depth(),
        };
        let post_process = environment
            .map(EnvironmentComponent::post_process_settings)
            .unwrap_or_default();
        if post_process.is_active() {
            encode_lit_frame(
                frame_renderers,
                &self.render_state.device,
                &self.render_state.queue,
                &mut encoder,
                target,
                &prepared,
                Lighting {
                    bloom: &mut self.bloom,
                    settings: post_process,
                },
            )
        } else {
            encode_prepared_frame(
                frame_renderers,
                &self.render_state.device,
                &self.render_state.queue,
                &mut encoder,
                target,
                &prepared,
            )
        }
        .map_err(|error| error.to_string())?;
        self.render_state.queue.submit([encoder.finish()]);
        profiler.add(Phase::Encoding, extracted.elapsed());
        if profiler.waits_for_gpu() {
            let waiting = Instant::now();
            self.render_state
                .device
                .poll(wgpu::PollType::wait_indefinitely())
                .map_err(|error| error.to_string())?;
            profiler.add(Phase::Gpu, waiting.elapsed());
        }
        Ok(text_sizes)
    }

    /// Sets the cube renderer's light, shadows, fog and ambient occlusion for
    /// this view, and answers the environment they came from.
    fn light(
        &mut self,
        source: SceneSource<'_>,
        camera: CameraView,
    ) -> Result<Option<EnvironmentComponent>, String> {
        // The extractor's environment rather than the world's: tolerantly, an
        // invalid one is the last valid one, so lighting holds still while a
        // value is dragged out of range instead of the frame failing.
        let environment = source
            .scene
            .environment(source.world)
            .map_err(|error| error.to_string())?;
        // The scene's sun and ambient, or the editor's own light when the
        // Scene view has the scene's lighting switched off.
        let (lighting, shadows) =
            super::super::scene_lighting::lighting_for(source, camera, self.aspect(), environment)?;
        self.cube.set_lighting(lighting);
        self.cube.set_shadows(&self.render_state.device, shadows);
        self.cube.set_fog(
            environment
                .map(EnvironmentComponent::fog_settings)
                .unwrap_or_default(),
        );
        self.cube
            .set_ambient_occlusion(environment.map_or(0.0, |environment| {
                if environment.ambient_occlusion.enabled {
                    environment.ambient_occlusion.strength
                } else {
                    0.0
                }
            }));
        Ok(environment)
    }

    /// Resizes the target and, when it actually changed, points egui at the
    /// new texture. The target answers whether that happened.
    fn resize(&mut self, width: u32, height: u32) {
        if !self.target.resize(&self.render_state.device, width, height) {
            return;
        }
        self.bloom.resize(&self.render_state.device, width, height);
        self.render_state
            .renderer
            .write()
            .update_egui_texture_from_wgpu_texture(
                &self.render_state.device,
                self.target.sampled(),
                wgpu::FilterMode::Linear,
                self.texture_id,
            );
    }
}

/// Gives the Scenes panel the frame a view just drew of `scene`, as often as
/// the board wants one; nothing for a scene with no file yet. The copy is GPU
/// work recorded on the CPU, so it is timed as encoding.
pub(super) fn picture(
    board: &mut scene_board_view::SceneBoardState,
    profiler: &mut Profiler,
    viewport: &RuntimeViewport,
    scene: Option<&std::path::Path>,
    tab: WorkspaceTab,
) {
    let Some(scene) = scene else {
        return;
    };
    let copying = Instant::now();
    let game = tab == WorkspaceTab::Game;
    board
        .pictures_mut()
        .take(&viewport.render_state, &viewport.target, scene, game);
    profiler.add(Phase::Encoding, copying.elapsed());
}
