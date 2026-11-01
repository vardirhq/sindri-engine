//! The bridge the Android app calls: `dev.sindri.mobile.NativeBridge`.
//!
//! The app owns the activity, its Compose UI and the `SurfaceView` the scene is
//! drawn into; this owns the GPU, the scene and its edits. Java calls in on the
//! UI thread and only there, so one thread-local [`Host`] holds everything, and
//! no call waits on another thread.
//!
//! Every export returns `null` when it succeeded and a message when it did not,
//! which the app shows rather than this crate deciding how to fail. A panic is
//! caught at the boundary and returned the same way, since unwinding into the
//! JVM would abort the process.

mod exports;
mod playing;
mod scripts;
mod touch;
mod window;

use std::path::PathBuf;

use sindri_gpu::{GpuContext, GpuRequestOptions, WindowSurface};

use sindri_runtime::ProjectAssets;

use crate::{Editing, OpenedScene, Playing, SceneRenderer};
use window::AndroidWindow;

/// Everything the app has opened and is drawing into.
#[derive(Default)]
struct Host {
    /// The project directory and scene file name last opened, kept so the
    /// renderer can be rebuilt if a new surface wants a different format.
    source: Option<(PathBuf, String)>,
    opened: Option<OpenedScene>,
    /// The selection, history and view over the opened scene.
    editing: Option<Editing>,
    /// Whether a renderer has already taken the opened scene's textures.
    installed: bool,
    instance: Option<wgpu::Instance>,
    /// Kept across surfaces: a phone's surface is destroyed whenever the app
    /// goes to the background, and the device outlives it.
    gpu: Option<GpuContext>,
    surface: Option<WindowSurface>,
    renderer: Option<SceneRenderer>,
    /// Screen pixels per dp, which the app sends: what the design's sizes
    /// (the outline's width, the stick's reach) are measured in.
    density: f32,
    /// The game running from the editor, while Play is on.
    playing: Option<Playing>,
    /// Where the finger that drives the stick landed, in surface pixels, and
    /// where it is now.
    stick: Option<([f32; 2], [f32; 2])>,
    /// When the last frame was drawn, which a run is advanced from.
    last_frame: Option<std::time::Instant>,
    /// Why the last run stopped by itself, until the app has seen it.
    play_error: Option<String>,
    /// The script the Behaviour sheet has open, as Decay blocks.
    script: Option<scripts::OpenScript>,
}

impl Host {
    fn open(&mut self, project: PathBuf, scene: String) -> Result<(), String> {
        let opened = OpenedScene::open(&project, &scene)?;
        self.editing = Some(Editing::new(&opened.world, &opened.extractor)?);
        self.opened = Some(opened);
        self.installed = false;
        self.source = Some((project, scene));
        // The new project's textures are not in the old renderer.
        self.renderer = None;
        self.prepare_renderer()
    }

    /// Builds the GPU on the first surface and a presentable surface on every
    /// one after it.
    fn attach(&mut self, window: AndroidWindow, width: u32, height: u32) -> Result<(), String> {
        let window = std::sync::Arc::new(window);
        let source = {
            let window = std::sync::Arc::clone(&window);
            move |instance: &wgpu::Instance| instance.create_surface(std::sync::Arc::clone(&window))
        };
        let instance = self
            .instance
            .get_or_insert_with(|| {
                wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(
                    Box::new(window::AndroidDisplay),
                ))
            })
            .clone();
        let surface = source(&instance).map_err(|error| error.to_string())?;
        if self.gpu.is_none() {
            let gpu = pollster::block_on(GpuContext::request(
                &instance,
                Some(&surface),
                &GpuRequestOptions::default(),
            ))
            .map_err(|error| error.to_string())?;
            self.gpu = Some(gpu);
        }
        let gpu = self.gpu.as_ref().ok_or("no GPU")?;
        let surface = WindowSurface::new(instance, surface, source, gpu, width, height)
            .map_err(|error| error.to_string())?;
        if self
            .renderer
            .as_ref()
            .is_some_and(|renderer| renderer.format() != surface.format())
        {
            self.renderer = None;
        }
        // A renderer kept from the last surface follows this one's size, which
        // changes if the phone was turned while the app was away.
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(&gpu.device, surface.width(), surface.height());
        }
        self.surface = Some(surface);
        self.prepare_renderer()
    }

    fn resize(&mut self, width: u32, height: u32) {
        let (Some(gpu), Some(surface)) = (&self.gpu, &mut self.surface) else {
            return;
        };
        surface.resize(&gpu.device, width, height);
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(&gpu.device, width, height);
        }
    }

    /// The surface is about to be destroyed: let go of it, keep everything else.
    fn detach(&mut self) {
        self.surface = None;
    }

    /// Makes a renderer for the current surface and scene, once both exist.
    ///
    /// The project's assets are read again when the renderer is rebuilt,
    /// because installing takes the decoded textures out of them. The world is
    /// kept, so a rebuild never loses an edit.
    fn prepare_renderer(&mut self) -> Result<(), String> {
        if self.renderer.is_some() {
            return Ok(());
        }
        let (Some(gpu), Some(surface)) = (&self.gpu, &self.surface) else {
            return Ok(());
        };
        let Some(opened) = &mut self.opened else {
            return Ok(());
        };
        if self.installed
            && let Some((project, _)) = &self.source
        {
            opened.assets = ProjectAssets::read(project)?;
            self.installed = false;
        }
        let mut renderer = SceneRenderer::new(
            &gpu.device,
            &gpu.queue,
            surface.format(),
            surface.width(),
            surface.height(),
        )?;
        renderer.install(
            &gpu.device,
            &gpu.queue,
            &mut opened.extractor,
            &mut opened.assets,
        )?;
        self.installed = true;
        self.renderer = Some(renderer);
        Ok(())
    }

    /// Draws one frame, if there is a surface and a scene to draw.
    fn draw(&mut self) -> Result<(), String> {
        let now = std::time::Instant::now();
        let elapsed = self
            .last_frame
            .replace(now)
            .map_or(0.0, |last| now.duration_since(last).as_secs_f32());
        let outline = self.outline_width();
        let (Some(gpu), Some(surface), Some(renderer), Some(opened), Some(editing)) = (
            &self.gpu,
            &mut self.surface,
            &mut self.renderer,
            &self.opened,
            &mut self.editing,
        ) else {
            return Ok(());
        };
        let Some(frame) = surface
            .acquire(&gpu.device)
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(surface.format()),
            ..Default::default()
        });
        if let Some(playing) = &mut self.playing {
            // A run that fails stops, and says why; the editor carries on.
            let ran = playing
                .advance(elapsed)
                .and_then(|()| playing.draw(renderer, &gpu.device, &gpu.queue, &view));
            gpu.queue.present(frame);
            if let Err(error) = ran {
                self.playing = None;
                self.stick = None;
                self.play_error = Some(format!("The game stopped: {error}"));
            }
            return Ok(());
        }
        #[allow(clippy::cast_precision_loss)] // A screen is far below 2^24 pixels.
        let size = [surface.width() as f32, surface.height() as f32];
        let view_camera = editing.canvas.view();
        renderer.set_lit(editing.lighting_preview);
        let world = editing.drawn(&opened.world, size, outline);
        renderer.draw(
            &gpu.device,
            &gpu.queue,
            &view,
            &opened.extractor,
            world,
            view_camera,
        )?;
        gpu.queue.present(frame);
        Ok(())
    }

    /// The selection outline's width in surface pixels: the design's one and
    /// a half points.
    fn outline_width(&self) -> f32 {
        1.5 * self.density
    }

    /// One line for the app's status bar.
    fn status(&self) -> String {
        let scene = self
            .opened
            .as_ref()
            .map_or_else(|| "no scene".to_owned(), OpenedScene::summary);
        let gpu = self.gpu.as_ref().map_or_else(
            || "no GPU yet".to_owned(),
            |gpu| {
                format!(
                    "{} ({:?})",
                    gpu.capabilities.adapter_name, gpu.capabilities.backend
                )
            },
        );
        format!("{scene} · {gpu}")
    }
}
