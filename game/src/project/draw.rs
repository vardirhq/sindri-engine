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
    DepthTarget, FrameRenderers, FrameTarget, GlyphRenderer, OffscreenTarget, ShapeRenderer,
    SpriteBatchRenderer, Texture2D, TextureRegistry, TexturedCubeRenderer, Viewport,
    encode_prepared_frame,
};
use sindri_scene::{CameraView, SceneRuntime, TextureBindings, measure_ui_text};

use super::ProjectPlayer;

/// Where one draw's time went, on the CPU.
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
}

/// The GPU side of playing a project offscreen.
pub struct ProjectRenderer {
    textures: TextureRegistry,
    bindings: TextureBindings,
    target: OffscreenTarget,
    depth: DepthTarget,
    cubes: TexturedCubeRenderer,
    sprites: SpriteBatchRenderer,
    glyphs: GlyphRenderer,
    shapes: ShapeRenderer,
    size: (u32, u32),
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
            cubes: TexturedCubeRenderer::new(&gpu.device, OffscreenTarget::FORMAT),
            sprites: SpriteBatchRenderer::new(&gpu.device, OffscreenTarget::FORMAT),
            glyphs: GlyphRenderer::new(&gpu.device, OffscreenTarget::FORMAT),
            shapes: ShapeRenderer::new(&gpu.device, OffscreenTarget::FORMAT),
            size,
        })
    }

    pub const fn target(&self) -> &OffscreenTarget {
        &self.target
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
        let prepared = measure_ui_text(&player.world, player.scene.components(), &mut player.text)
            .map_err(Box::<dyn Error>::from)
            .and_then(|sizes| {
                let prepared = player.scene.extract_animated(
                    &player.world,
                    Viewport::new(self.size.0, self.size.1),
                    CameraView::default(),
                    &self.bindings,
                    SceneRuntime::default()
                        .with_animations(player.session.animations())
                        .with_effects(player.session.effects())
                        .with_text_sizes(&sizes)
                        .with_tile_sets(&player.tile_sets),
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
        encode_prepared_frame(
            FrameRenderers {
                cube: &mut self.cubes,
                sprites: &mut self.sprites,
                text: &mut player.text,
                glyphs: &mut self.glyphs,
                shapes: &mut self.shapes,
                textures: &self.textures,
            },
            &gpu.device,
            &gpu.queue,
            &mut encoder,
            FrameTarget {
                color: self.target.view(),
                depth: &self.depth,
            },
            &prepared,
        )?;
        gpu.queue.submit([encoder.finish()]);
        Ok(DrawTimes {
            presentation: styled.duration_since(began) + unstyled.duration_since(extracted),
            extraction: extracted.duration_since(styled),
            encoding: unstyled.elapsed(),
        })
    }
}
