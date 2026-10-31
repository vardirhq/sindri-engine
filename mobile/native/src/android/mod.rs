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
mod touch;
mod window;

use std::path::PathBuf;

use sindri_gpu::{GpuContext, GpuRequestOptions, WindowSurface};

use sindri_runtime::ProjectAssets;

use crate::{Editing, OpenedScene, SceneRenderer};
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
        let (Some(gpu), Some(surface), Some(renderer), Some(opened), Some(editing)) = (
            &self.gpu,
            &mut self.surface,
            &mut self.renderer,
            &self.opened,
            &self.editing,
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
        renderer.draw(
            &gpu.device,
            &gpu.queue,
            &view,
            &opened.extractor,
            &opened.world,
            editing.canvas.view(),
        )?;
        gpu.queue.present(frame);
        Ok(())
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
