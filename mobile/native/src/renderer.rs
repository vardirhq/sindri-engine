//! Drawing an opened scene into a `wgpu` view.
//!
//! The binding and drawing here follow the native player's install and render
//! (`crates/sindri-player/src/player.rs`) step for step, minus the session:
//! this draws an authored world, not a run. The player's version is tied to its
//! desktop host, which does not build for Android, so it cannot be called from
//! here yet. `mobile/first-slice.md` lists it among the pieces to extract into
//! shared code once the mobile host settles what it needs.

use sindri_assets::builtin_textures;
use sindri_core::{AssetId, World, sheet_id_for};
use sindri_render::{
    Bloom, DepthTarget, FrameRenderers, FrameTarget, GlyphRenderer, Lighting, ShapeRenderer,
    SpriteBatchRenderer, TextRenderer, Texture2D, TextureRegistry, TexturedCubeRenderer, Viewport,
    encode_lit_frame, encode_prepared_frame,
};
use sindri_runtime::ProjectAssets;
use sindri_scene::{
    CameraView, Effects2d, SceneExtractor, SceneRuntime, SpriteAnimations, TextureBindings,
    TileSetBindings, UiTextSizes, measure_ui_text,
};

fn text<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}

/// The GPU side of showing a scene: its textures, fonts and tile sets, and the
/// renderers that draw a frame of it.
pub struct SceneRenderer {
    format: wgpu::TextureFormat,
    size: [u32; 2],
    textures: TextureRegistry,
    bindings: TextureBindings,
    tile_sets: TileSetBindings,
    depth: DepthTarget,
    bloom: Bloom,
    cubes: TexturedCubeRenderer,
    sprites: SpriteBatchRenderer,
    text: TextRenderer,
    glyphs: GlyphRenderer,
    shapes: ShapeRenderer,
    /// Whether frames show the scene's 2D lighting.
    lit: bool,
}

impl SceneRenderer {
    /// Renderers for views of `format`, `width` by `height` pixels, with the
    /// engine's own textures and tile sets already bound.
    ///
    /// # Errors
    /// One of the engine's own assets will not decode or upload.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let mut textures = TextureRegistry::new(device, queue);
        let mut bindings = TextureBindings::new();
        for texture in builtin_textures() {
            let asset = texture.decode().map_err(text)?;
            let uploaded = Texture2D::from_rgba8(
                device,
                queue,
                texture.reference,
                asset.width(),
                asset.height(),
                asset.rgba8(),
            )
            .map_err(text)?;
            bindings.bind(texture.reference, textures.insert(uploaded));
            if let Some(sheet) = texture.sheet() {
                bindings
                    .bind_sheet(texture.reference, &sheet.map_err(text)?)
                    .map_err(text)?;
            }
        }
        let mut tile_sets = TileSetBindings::new();
        sindri_runtime::bind_builtin_tile_sets(&mut tile_sets).map_err(text)?;
        let mut bloom = Bloom::new(device, format);
        bloom.resize(device, width, height);
        Ok(Self {
            format,
            size: [width, height],
            textures,
            bindings,
            tile_sets,
            depth: DepthTarget::new(device, width, height),
            bloom,
            cubes: TexturedCubeRenderer::new(device, format),
            sprites: SpriteBatchRenderer::new(device, format),
            text: TextRenderer::new(),
            glyphs: GlyphRenderer::new(device, format),
            shapes: ShapeRenderer::new(device, format),
            lit: true,
        })
    }

    /// Whether frames show the scene's 2D lights and the darkness they light,
    /// as the game does (the default), or every sprite in its own colors, as
    /// an editor's view with its lighting preview off does.
    pub const fn set_lit(&mut self, lit: bool) {
        self.lit = lit;
    }

    /// The view format these renderers were built for.
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Uploads and binds what the project holds for drawing: textures and their
    /// sheets, tile sets, sprite shaders, fonts and render materials.
    ///
    /// Takes them out of `assets`, which keeps its scenes, scripts and prefabs.
    /// Models are not bound yet, so a 3D scene draws without them.
    ///
    /// # Errors
    /// A texture that will not upload, or a sheet, tile set or material that
    /// will not bind.
    pub fn install(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        extractor: &mut SceneExtractor,
        assets: &mut ProjectAssets,
    ) -> Result<(), String> {
        extractor.set_materials(
            sindri_scene::RenderMaterialSources::from_profiles(
                assets
                    .profiles
                    .ids()
                    .filter_map(|id| assets.profiles.get(id).map(|profile| (id, profile))),
            )
            .map_err(text)?,
        );
        let mut loaded: Vec<AssetId> = Vec::new();
        for (id, asset) in std::mem::take(&mut assets.textures) {
            let texture = Texture2D::from_rgba8(
                device,
                queue,
                id.as_str(),
                asset.width(),
                asset.height(),
                asset.rgba8(),
            )
            .map_err(text)?;
            self.bindings
                .bind(id.as_str(), self.textures.insert(texture));
            loaded.push(id);
        }
        for texture_id in loaded {
            let Some(sheet_id) = sheet_id_for(&texture_id) else {
                continue;
            };
            if let Some(sheet) = assets.sheets.get(sheet_id.as_str()) {
                self.bindings
                    .bind_sheet(texture_id.as_str(), sheet)
                    .map_err(text)?;
            }
        }
        for (id, sheet) in &assets.sheets {
            self.tile_sets
                .bind_terrains(id.as_str(), sheet.terrains.clone());
        }
        for (id, tile_set) in std::mem::take(&mut assets.tile_sets) {
            self.tile_sets.bind(id.as_str(), tile_set).map_err(text)?;
        }
        // A broken shader leaves its sprites drawn the built-in way, as in the
        // player; the app is told through the status line rather than failing.
        for (id, source) in std::mem::take(&mut assets.shaders) {
            let _ = self.bindings.shaders_mut().bind(&id, &source);
        }
        for (id, asset) in std::mem::take(&mut assets.fonts) {
            self.text
                .bind_font(id.as_str(), asset.family(), asset.bytes().to_vec());
        }
        Ok(())
    }

    /// Follows the view to a new size. Zero is clamped to one pixel.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let (width, height) = (width.max(1), height.max(1));
        if self.size == [width, height] {
            return;
        }
        self.size = [width, height];
        self.depth = DepthTarget::new(device, width, height);
        self.bloom.resize(device, width, height);
    }

    /// Draws `world` into `view`, through `camera`: the scene's own with
    /// [`CameraView::default`], or a [`crate::Canvas`] view, lit or not as
    /// [`Self::set_lit`] last said. Nothing is animated: the editor's view.
    ///
    /// # Errors
    /// A component that will not extract, or a frame that will not encode.
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        extractor: &SceneExtractor,
        world: &World,
        camera: CameraView,
    ) -> Result<(), String> {
        self.draw_shot(
            device,
            queue,
            view,
            Shot {
                extractor,
                world,
                camera,
                animations: None,
                effects: None,
                seconds: 0.0,
            },
        )
        .map(|_| ())
    }

    /// Draws a [`Shot`], and returns the screen text as it was measured, which
    /// a running session lays its screen out with.
    ///
    /// # Errors
    /// A component that will not extract, or a frame that will not encode.
    pub fn draw_shot(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        shot: Shot<'_>,
    ) -> Result<UiTextSizes, String> {
        let Shot {
            extractor,
            world,
            camera,
            animations,
            effects,
            seconds,
        } = shot;
        let post_process = self
            .cubes
            .configure(device, extractor.world_settings(world).map_err(text)?);
        let sizes = measure_ui_text(world, extractor.components(), &mut self.text).map_err(text)?;
        let mut runtime = SceneRuntime::default()
            .with_tile_sets(&self.tile_sets)
            .with_text_sizes(&sizes)
            .with_seconds(seconds)
            .with_unlit(!self.lit);
        if let Some(animations) = animations {
            runtime = runtime.with_animations(animations);
        }
        if let Some(effects) = effects {
            runtime = runtime.with_effects(effects);
        }
        let prepared = extractor
            .extract_animated(
                world,
                Viewport::new(self.size[0], self.size[1]),
                camera,
                &self.bindings,
                runtime,
            )
            .map_err(text)?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Sindri Mobile scene"),
        });
        let renderers = FrameRenderers {
            cube: &mut self.cubes,
            sprites: &mut self.sprites,
            text: &mut self.text,
            glyphs: &mut self.glyphs,
            shapes: &mut self.shapes,
            textures: &self.textures,
        };
        let target = FrameTarget {
            color: view,
            depth: &self.depth,
        };
        if post_process.is_active() {
            encode_lit_frame(
                renderers,
                device,
                queue,
                &mut encoder,
                target,
                &prepared,
                Lighting {
                    bloom: &mut self.bloom,
                    settings: post_process,
                },
            )
            .map_err(text)?;
        } else {
            encode_prepared_frame(renderers, device, queue, &mut encoder, target, &prepared)
                .map_err(text)?;
        }
        queue.submit([encoder.finish()]);
        Ok(sizes)
    }
}

/// What one frame draws: a world, through a camera, as far into its
/// animations and flecks as a running session has got.
#[derive(Clone, Copy)]
pub struct Shot<'a> {
    /// The schemas the world is read with.
    pub extractor: &'a SceneExtractor,
    pub world: &'a World,
    pub camera: CameraView,
    /// Where each animated sprite has got to; `None` shows every sprite at
    /// its authored rect.
    pub animations: Option<&'a SpriteAnimations>,
    /// The live flecks, while a game runs.
    pub effects: Option<&'a Effects2d>,
    /// How long the scene has been running.
    pub seconds: f32,
}
