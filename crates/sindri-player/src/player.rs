//! The player: install a project, then step and draw it every frame.
//!
//! Where the project comes from is the only thing that differs between a
//! browser and a desktop. In a browser nothing is owned until the fetch
//! source has returned it through `AssetLoader` and the manifest has accepted
//! the bytes; natively the project is read from the directory named on the
//! command line. Both arrive as [`ProjectAssets`], and from there a project
//! is installed, stepped, styled and drawn the same way.

use std::time::Duration;

use sindri_core::{AssetId, EngineState, LoadedScenes, World, sheet_id_for};
use sindri_desktop::{AppContext, DesktopApp, Flow};
#[cfg(target_arch = "wasm32")]
use sindri_platform::BrowserAudioBackend;
use sindri_platform::{AudioBackend, AudioClip, EngineHost, InputEvent};
use sindri_render::{
    DepthTarget, FrameRenderers, FrameTarget, GlyphRenderer, ShapeRenderer, SpriteBatchRenderer,
    TextRenderer, Texture2D, TextureRegistry, TexturedCubeRenderer, Viewport,
    encode_prepared_frame,
};
use sindri_scene::{
    CameraView, SceneExtractor, SceneRuntime, TextureBindings, TileSetBindings, measure_ui_text,
};
use weave::Viewport as WeaveViewport;

#[cfg(target_arch = "wasm32")]
use crate::browser::BrowserProjectLoader;
use crate::error::PlayerError;
use crate::project::ProjectAssets;
use sindri_runtime::{Session, scene_extractor};

/// The sound device a platform plays through.
#[cfg(target_arch = "wasm32")]
type Audio = BrowserAudioBackend;
#[cfg(not(target_arch = "wasm32"))]
type Audio = sindri_platform::MaybeAudio<sindri_platform::NativeAudioBackend>;

/// Where the project being played is coming from.
enum Source {
    /// Still being fetched over the network.
    #[cfg(target_arch = "wasm32")]
    Fetching(BrowserProjectLoader),
    /// Read, waiting for the first draw to install it.
    Ready(Box<ProjectAssets>),
    /// Installed, or never coming.
    Spent,
}

pub(crate) struct Player {
    source: Source,
    audio: Option<Audio>,
    engine: Option<EngineHost<Session, Audio>>,
    scene: SceneExtractor,
    bindings: TextureBindings,
    tile_sets: TileSetBindings,
    textures: TextureRegistry,
    depth: DepthTarget,
    cubes: TexturedCubeRenderer,
    sprites: SpriteBatchRenderer,
    text: TextRenderer,
    glyphs: GlyphRenderer,
    shapes: ShapeRenderer,
    viewport: [u32; 2],
    layout_viewport: [f64; 2],
    page_visible: bool,
    platform_suspended: bool,
    paused_for_page: bool,
}

impl Player {
    #[allow(clippy::cast_possible_truncation)]
    fn weave_viewport(&self) -> WeaveViewport {
        WeaveViewport {
            width: self.layout_viewport[0] as f32,
            height: self.layout_viewport[1] as f32,
        }
    }

    fn install(
        &mut self,
        context: &AppContext<'_>,
        project: ProjectAssets,
    ) -> Result<(), PlayerError> {
        let mut loaded_textures: Vec<AssetId> = Vec::new();
        for (id, asset) in project.textures {
            let texture = Texture2D::from_rgba8(
                context.device(),
                context.queue(),
                id.as_str(),
                asset.width(),
                asset.height(),
                asset.rgba8(),
            )?;
            self.bindings
                .bind(id.as_str(), self.textures.insert(texture));
            loaded_textures.push(id);
        }

        for texture_id in loaded_textures {
            let Some(sheet_id) = sheet_id_for(&texture_id) else {
                continue;
            };
            if let Some(sheet) = project.sheets.get(sheet_id.as_str()) {
                self.bindings.bind_sheet(texture_id.as_str(), sheet)?;
            }
        }

        for (id, tile_set) in project.tile_sets {
            self.tile_sets.bind(id.as_str(), tile_set)?;
        }

        for (id, asset) in project.fonts {
            self.text
                .bind_font(id.as_str(), asset.family(), asset.bytes().to_vec());
        }

        let mut audio = self
            .audio
            .take()
            .ok_or_else(|| PlayerError::Project("the audio backend was already moved".into()))?;
        for (id, asset) in project.audio {
            audio.register(AudioClip::new(
                id.as_str(),
                asset.bytes().to_vec(),
                asset.format().mime_type(),
            ))?;
        }

        // Browser exports used to fetch every declared scene and then discard
        // all but the entry document. Enter the entry through LoadedScenes, just
        // like native does, and give the session the complete scene set so a
        // Decay `Scene.go` request has somewhere real to go.
        let (entry_name, entry_document) =
            project.scenes.first().ok_or(PlayerError::MissingScene)?;
        let saves = save_backend(entry_name);
        let mut world = World::default();
        let mut loaded_scenes = LoadedScenes::new();
        loaded_scenes.enter_keeping_identities_with(
            &mut world,
            entry_name,
            entry_document,
            &project.prefabs,
        )?;

        let mut session = Session::with_sources(self.scene.components().clone(), project.scripts)
            .with_prefabs(project.prefabs)
            .with_profiles(project.profiles)
            .with_scenes(project.scenes, loaded_scenes)
            // Cloned rather than moved: the host keeps its own copy for
            // extraction, and a floor the renderer can draw and the scripts
            // cannot reach would be the worst of both.
            .with_tile_sets(self.tile_sets.clone())
            .with_styles(project.stylesheets);
        session.keep_saves_in(saves);

        session.settle_styles(&mut world, self.weave_viewport())?;
        let mut engine =
            EngineHost::new_with_audio(session, sindri_core::FixedStepConfig::default(), audio)?;
        *engine.world_mut() = world;
        engine.start()?;
        engine.set_viewport(self.viewport[0], self.viewport[1]);
        self.engine = Some(engine);
        self.sync_page_lifecycle()?;
        log::info!("Loaded {} project assets", project.asset_count);
        Ok(())
    }

    fn sync_page_lifecycle(&mut self) -> Result<(), PlayerError> {
        let should_pause = !self.page_visible || self.platform_suspended;
        let Some(engine) = &mut self.engine else {
            return Ok(());
        };

        if should_pause && !self.paused_for_page && engine.state() == EngineState::Running {
            engine.pause()?;
            self.paused_for_page = true;
        } else if !should_pause && self.paused_for_page && engine.state() == EngineState::Paused {
            engine.resume()?;
            self.paused_for_page = false;
        }
        Ok(())
    }

    fn clear_loading(context: &AppContext<'_>, view: &wgpu::TextureView) {
        let mut encoder =
            context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Sindri player loading encoder"),
                });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sindri player loading pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.012,
                            g: 0.018,
                            b: 0.03,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        context.queue().submit([encoder.finish()]);
    }
}

impl DesktopApp for Player {
    type Error = PlayerError;

    fn create(context: &AppContext<'_>) -> Result<Self, Self::Error> {
        // The engine's own assets first: the export leaves them out of the
        // manifest because they come with the engine, not over the network.
        let mut textures = TextureRegistry::new(context.device(), context.queue());
        let mut bindings = TextureBindings::new();
        crate::bind_builtin_textures(
            context.device(),
            context.queue(),
            &mut textures,
            &mut bindings,
        )?;
        let mut tile_sets = TileSetBindings::new();
        sindri_runtime::bind_builtin_tile_sets(&mut tile_sets)?;
        Ok(Self {
            source: Source::open()?,
            audio: Some(audio_backend()?),
            engine: None,
            scene: scene_extractor().map_err(PlayerError::Project)?,
            bindings,
            tile_sets,
            textures,
            depth: DepthTarget::new(context.device(), context.width(), context.height()),
            cubes: TexturedCubeRenderer::new(context.device(), context.format()),
            sprites: SpriteBatchRenderer::new(context.device(), context.format()),
            text: TextRenderer::new(),
            glyphs: GlyphRenderer::new(context.device(), context.format()),
            shapes: ShapeRenderer::new(context.device(), context.format()),
            viewport: [context.width(), context.height()],
            layout_viewport: [context.logical_width(), context.logical_height()],
            page_visible: true,
            platform_suspended: false,
            paused_for_page: false,
        })
    }

    fn editing_text(&self) -> bool {
        self.engine
            .as_ref()
            .is_some_and(|engine| engine.game().editing_text())
    }

    fn take_copied(&mut self) -> Option<String> {
        self.engine.as_mut()?.game_mut().take_copied()
    }

    fn input(&mut self, event: InputEvent) {
        if let Some(engine) = &mut self.engine {
            engine.queue_input(event);
            return;
        }
        if matches!(
            event,
            InputEvent::KeyPressed(_)
                | InputEvent::ButtonPressed(_)
                | InputEvent::TouchStarted { .. }
        ) && let Some(audio) = &mut self.audio
        {
            let _ = audio.unlock();
        }
    }

    fn update(&mut self, delta: Duration) -> Result<Flow, Self::Error> {
        if self.engine.is_none() {
            self.source.poll()?;
            return Ok(Flow::Continue);
        }

        // Escape is the game's: a back, a pause, closing a list. It never
        // ends a page, and treating it as Exit stopped the whole engine
        // whenever a slow frame fell between its going down and coming up --
        // a pause on Orbital, or Back in a menu, froze the game on a phone.
        let engine = self.engine.as_mut().expect("checked above");
        engine.advance(delta)?;
        Ok(Flow::Continue)
    }

    fn resize(&mut self, context: &AppContext<'_>) -> Result<(), Self::Error> {
        self.depth
            .resize(context.device(), context.width(), context.height());
        self.viewport = [context.width(), context.height()];
        self.layout_viewport = [context.logical_width(), context.logical_height()];
        let viewport = self.weave_viewport();
        if let Some(engine) = self.engine.as_mut() {
            engine.set_viewport(context.width(), context.height());
            // Restyled for the new shape, as a browser re-evaluates its media
            // queries; taken out of the host for the while.
            let mut world = std::mem::take(engine.world_mut());
            let settled = engine.game_mut().settle_styles(&mut world, viewport);
            *engine.world_mut() = world;
            settled?;
        }
        Ok(())
    }

    fn suspend(&mut self) -> Result<(), Self::Error> {
        self.platform_suspended = true;
        self.sync_page_lifecycle()
    }

    fn resume(&mut self) -> Result<(), Self::Error> {
        self.platform_suspended = false;
        self.sync_page_lifecycle()
    }

    /// Ready once the project is installed: until then `render` draws only
    /// the loading clear, and the page's loading screen should stay up.
    fn ready(&self) -> bool {
        self.engine.is_some()
    }

    fn visibility_changed(&mut self, visible: bool) -> Result<(), Self::Error> {
        self.page_visible = visible;
        self.sync_page_lifecycle()
    }

    fn render(
        &mut self,
        context: &AppContext<'_>,
        view: &wgpu::TextureView,
    ) -> Result<(), Self::Error> {
        if self.engine.is_none()
            && let Some(project) = self.source.take()
        {
            self.install(context, *project)?;
        }

        let viewport = self.weave_viewport();
        let Some(engine) = &mut self.engine else {
            Self::clear_loading(context, view);
            return Ok(());
        };

        // Styled where it stands for this draw and put back straight after;
        // taken out of the host for the while so the session can style it.
        let mut world = std::mem::take(engine.world_mut());
        let styled = engine
            .game_mut()
            .style(&mut world, viewport)
            .map_err(PlayerError::from);
        let prepared = styled.and_then(|undo| {
            // Measured as styled, since a stylesheet sets the font size.
            let prepared = measure_ui_text(&world, self.scene.components(), &mut self.text)
                .map_err(PlayerError::from)
                .and_then(|sizes| {
                    let prepared = self.scene.extract_animated(
                        &world,
                        Viewport::new(context.width(), context.height()),
                        CameraView::default(),
                        &self.bindings,
                        SceneRuntime::default()
                            .with_animations(engine.game().animations())
                            .with_effects(engine.game().effects())
                            .with_tile_sets(&self.tile_sets)
                            .with_text_sizes(&sizes),
                    )?;
                    engine.game_mut().record_drawn(&world, viewport, sizes)?;
                    Ok(prepared)
                });
            if let Some(undo) = undo {
                undo.undo(&mut world);
            }
            prepared
        });
        *engine.world_mut() = world;
        let prepared = prepared?;
        let mut encoder =
            context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Sindri player encoder"),
                });
        encode_prepared_frame(
            FrameRenderers {
                cube: &mut self.cubes,
                sprites: &mut self.sprites,
                text: &mut self.text,
                glyphs: &mut self.glyphs,
                shapes: &mut self.shapes,
                textures: &self.textures,
            },
            context.device(),
            context.queue(),
            &mut encoder,
            FrameTarget {
                color: view,
                depth: &self.depth,
            },
            &prepared,
        )?;
        context.queue().submit([encoder.finish()]);
        Ok(())
    }
}

impl Source {
    /// Starts fetching the project served beside the page.
    #[cfg(target_arch = "wasm32")]
    fn open() -> Result<Self, PlayerError> {
        Ok(Self::Fetching(BrowserProjectLoader::new()?))
    }

    /// Reads the project named on the command line.
    #[cfg(not(target_arch = "wasm32"))]
    fn open() -> Result<Self, PlayerError> {
        let directory = std::env::args_os().nth(1).ok_or_else(|| {
            PlayerError::Project("usage: sindri-player <project directory>".to_owned())
        })?;
        let project = crate::directory::read(std::path::Path::new(&directory))?;
        Ok(Self::Ready(Box::new(project)))
    }

    /// Moves a fetch on, keeping what arrived. Nothing to do natively, where
    /// the project was read before the window opened.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        allow(clippy::unused_self, clippy::unnecessary_wraps)
    )]
    fn poll(&mut self) -> Result<(), PlayerError> {
        #[cfg(target_arch = "wasm32")]
        if let Self::Fetching(loader) = self
            && let Some(project) = loader.poll()?
        {
            *self = Self::Ready(Box::new(project));
        }
        Ok(())
    }

    /// The project, once, if it has arrived.
    // Which other variants there are depends on the target, so the catch-all
    // is the one arm that reads the same on both.
    #[allow(clippy::match_wildcard_for_single_variants)]
    fn take(&mut self) -> Option<Box<ProjectAssets>> {
        match std::mem::replace(self, Self::Spent) {
            Self::Ready(project) => Some(project),
            other => {
                *self = other;
                None
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[allow(clippy::unnecessary_wraps)]
fn audio_backend() -> Result<Audio, PlayerError> {
    Ok(BrowserAudioBackend::new())
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::unnecessary_wraps)]
fn audio_backend() -> Result<Audio, PlayerError> {
    Ok(sindri_platform::MaybeAudio::open_or_silent(
        sindri_platform::NativeAudioBackend::new,
    ))
}

/// Where a project's save is kept: the page's storage in a browser, a file
/// beside where the player was run natively. Named by the project's entry
/// scene, because every game on one site shares one origin's storage, and a
/// save keyed by anything less is a save another game overwrites.
fn save_backend(entry_scene: &str) -> Box<dyn sindri_platform::SaveBackend> {
    let stem = entry_scene
        .rsplit('/')
        .next()
        .unwrap_or(entry_scene)
        .trim_end_matches(".scene");
    #[cfg(target_arch = "wasm32")]
    {
        Box::new(sindri_platform::BrowserSaves::under(&format!(
            "sindri.{stem}.save"
        )))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Box::new(sindri_platform::FileSaves::at(std::path::Path::new(
            &format!("{stem}-save.json"),
        )))
    }
}
