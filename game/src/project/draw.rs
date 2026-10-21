//! Drawing an opened project offscreen, as many times as asked.
//!
//! Built once — textures bound, targets made, pipelines compiled — so drawing
//! a frame is only what a host does every frame: style, measure, extract,
//! encode and submit. That is what `project-benchmark` times, and the one
//! picture `project-capture` takes goes through the same draw.

use std::collections::BTreeMap;
use std::error::Error;
use std::time::{Duration, Instant};

use sindri_assets::{AssetBytes, AssetDecoder, TextureAssetDecoder};
use sindri_core::{AssetId, SpriteSheetDocument, sheet_id_for};
use sindri_gpu::GpuContext;
use sindri_render::{
    Bloom, DepthTarget, FrameRenderers, FrameTarget, GlyphRenderer, Lighting, OffscreenTarget,
    ShapeRenderer, SpriteBatchRenderer, Texture2D, TextureRegistry, TexturedCubeRenderer, Viewport,
    encode_lit_frame, encode_prepared_frame,
};
use sindri_scene::{CameraView, SceneRuntime, TextureBindings, measure_ui_text};

use super::ProjectPlayer;

/// One draw's CPU timings and scoped renderer counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrawTimes {
    /// Laying the stylesheets' per-draw states over the world, and taking
    /// them off again.
    pub presentation: Duration,
    /// Measuring text, extracting the frame from the world and recording
    /// where its screen elements were drawn.
    pub extraction: Duration,
    /// Recording the frame's commands and submitting them.
    pub encoding: Duration,
    /// Physical textured 3D/model work; excludes the frame clear, 2D and post.
    pub meshes: sindri_render::MeshDrawStats,
    /// Physical sprite/shape/glyph batches; excludes atlas/texture initialization.
    pub batches: sindri_render::BatchDrawStats,
    /// New imported assets uploaded this draw, not primitive/buffer counts.
    pub model_uploads: u64,
    /// Persistent mesh vertex/index buffer pairs uploaded this draw.
    pub cached_geometry_uploads: u64,
}

/// The GPU side of playing a project offscreen.
pub struct ProjectRenderer {
    textures: TextureRegistry,
    bindings: TextureBindings,
    target: OffscreenTarget,
    depth: DepthTarget,
    bloom: Bloom,
    cubes: TexturedCubeRenderer,
    sprites: SpriteBatchRenderer,
    glyphs: GlyphRenderer,
    shapes: ShapeRenderer,
    size: (u32, u32),
    timing: Option<sindri_gpu::GpuTimer>,
}

impl ProjectRenderer {
    /// Binds the project's images and sheets and makes a target of `size`.
    /// An image that will not decode is left unbound, as a host leaves it.
    pub fn new(
        gpu: &GpuContext,
        images: &BTreeMap<String, Vec<u8>>,
        sheets: &BTreeMap<String, Vec<u8>>,
        size: (u32, u32),
    ) -> Result<Self, Box<dyn Error>> {
        let mut textures = TextureRegistry::new(&gpu.device, &gpu.queue);
        let mut bindings = TextureBindings::new();
        crate::bind_builtin_textures(&gpu.device, &gpu.queue, &mut textures, &mut bindings)?;
        for (id, bytes) in images {
            let Ok(asset) =
                TextureAssetDecoder.decode(AssetBytes::new(id.parse::<AssetId>()?, bytes.clone()))
            else {
                continue;
            };
            let texture = Texture2D::from_rgba8(
                &gpu.device,
                &gpu.queue,
                id,
                asset.width(),
                asset.height(),
                asset.rgba8(),
            )?;
            bindings.bind(id, textures.insert(texture));
            let sheet = id
                .parse::<AssetId>()
                .ok()
                .and_then(|id| sheet_id_for(&id))
                .and_then(|sheet| sheets.get(sheet.as_str()));
            if let Some(json) = sheet {
                bindings.bind_sheet(
                    id,
                    &SpriteSheetDocument::from_json(std::str::from_utf8(json)?)?,
                )?;
            }
        }
        let (width, height) = size;
        Ok(Self {
            textures,
            bindings,
            target: OffscreenTarget::new(&gpu.device, width, height)?,
            depth: DepthTarget::new(&gpu.device, width, height),
            bloom: {
                let mut bloom = Bloom::new(&gpu.device, OffscreenTarget::FORMAT);
                bloom.resize(&gpu.device, width, height);
                bloom
            },
            cubes: TexturedCubeRenderer::new(&gpu.device, OffscreenTarget::FORMAT),
            sprites: SpriteBatchRenderer::new(&gpu.device, OffscreenTarget::FORMAT),
            glyphs: GlyphRenderer::new(&gpu.device, OffscreenTarget::FORMAT),
            shapes: ShapeRenderer::new(&gpu.device, OffscreenTarget::FORMAT),
            size,
            timing: None,
        })
    }

    /// Checks and binds the project's sprite shaders, refusing the first one
    /// that does not compile: a capture of a broken shader is a capture of
    /// the built-in look, which proves nothing about the shader.
    pub fn bind_shaders(
        &mut self,
        shaders: &BTreeMap<String, Vec<u8>>,
    ) -> Result<(), Box<dyn Error>> {
        for (id, bytes) in shaders {
            self.bindings
                .shaders_mut()
                .bind(id, std::str::from_utf8(bytes)?)?;
        }
        Ok(())
    }

    pub const fn target(&self) -> &OffscreenTarget {
        &self.target
    }

    /// Enables optional native GPU timestamps for synchronous benchmark draws.
    pub fn enable_gpu_timing(&mut self, gpu: &GpuContext) -> bool {
        self.timing = sindri_gpu::GpuTimer::new(&gpu.device, &gpu.queue);
        self.timing.is_some()
    }

    /// Read after submission, before drawing again with timing enabled.
    pub fn gpu_execution(
        &self,
        gpu: &GpuContext,
    ) -> Result<Option<Duration>, sindri_gpu::GpuTimingError> {
        self.timing
            .as_ref()
            .map(|timer| timer.read(&gpu.device))
            .transpose()
    }

    fn batch_stats(&self) -> sindri_render::BatchDrawStats {
        self.sprites
            .draw_stats()
            .combined(self.shapes.draw_stats())
            .combined(self.glyphs.draw_stats())
    }

    /// Draws the player's world once and submits it.
    pub fn draw(
        &mut self,
        player: &mut ProjectPlayer,
        gpu: &GpuContext,
    ) -> Result<DrawTimes, Box<dyn Error>> {
        let began = Instant::now();
        let view = player.viewport();
        let undo = player.session.style(&mut player.world, view)?;
        let styled = Instant::now();
        let mut post_process = sindri_render::PostProcessSettings::default();
        let prepared = measure_ui_text(&player.world, player.scene.components(), &mut player.text)
            .map_err(Box::<dyn Error>::from)
            .and_then(|sizes| {
                post_process = self
                    .cubes
                    .configure(&gpu.device, player.scene.world_settings(&player.world)?);
                let prepared = player.scene.extract_animated(
                    &player.world,
                    Viewport::new(self.size.0, self.size.1),
                    CameraView::default(),
                    &self.bindings,
                    SceneRuntime::default()
                        .with_animations(player.session.animations())
                        .with_effects(player.session.effects())
                        .with_text_sizes(&sizes)
                        .with_tile_sets(&player.tile_sets)
                        .with_seconds(player.seconds),
                )?;
                // As a host does: what was drawn is what the next step's
                // pointer is hit-tested against.
                player.session.record_drawn(&player.world, view, sizes)?;
                Ok(prepared)
            });
        let extracted = Instant::now();
        if let Some(undo) = undo {
            undo.undo(&mut player.world);
        }
        let prepared = prepared?;
        let unstyled = Instant::now();
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("project draw"),
            });
        if let Some(timer) = &self.timing {
            timer.begin(&mut encoder);
        }
        let before = self.cubes.draw_stats();
        let batches = self.batch_stats();
        let model_uploads = self.cubes.model_cache_stats().uploads;
        let cached_uploads = self.cubes.cached_mesh_stats().buffer_uploads;
        let renderers = FrameRenderers {
            cube: &mut self.cubes,
            sprites: &mut self.sprites,
            text: &mut player.text,
            glyphs: &mut self.glyphs,
            shapes: &mut self.shapes,
            textures: &self.textures,
        };
        let target = FrameTarget {
            color: self.target.view(),
            depth: &self.depth,
        };
        if post_process.is_active() {
            encode_lit_frame(
                renderers,
                &gpu.device,
                &gpu.queue,
                &mut encoder,
                target,
                &prepared,
                Lighting {
                    bloom: &mut self.bloom,
                    settings: post_process,
                },
            )?;
        } else {
            encode_prepared_frame(
                renderers,
                &gpu.device,
                &gpu.queue,
                &mut encoder,
                target,
                &prepared,
            )?;
        }
        if let Some(timer) = &self.timing {
            timer.finish(&mut encoder);
        }
        gpu.queue.submit([encoder.finish()]);
        Ok(DrawTimes {
            presentation: styled.duration_since(began) + unstyled.duration_since(extracted),
            extraction: extracted.duration_since(styled),
            encoding: unstyled.elapsed(),
            meshes: self.cubes.draw_stats().since(before),
            batches: self.batch_stats().since(batches),
            model_uploads: self
                .cubes
                .model_cache_stats()
                .uploads
                .saturating_sub(model_uploads),
            cached_geometry_uploads: self
                .cubes
                .cached_mesh_stats()
                .buffer_uploads
                .saturating_sub(cached_uploads),
        })
    }
}
