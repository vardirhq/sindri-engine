//! Any project played offscreen the way the browser plays it: the same
//! session, the same Weave styling for the viewport, the same draw, with
//! clicks, keys and wheel delivered as a host delivers them.
//!
//! What `sindri play` photographs, and what Causeway's `project-capture` and
//! `project-benchmark` drive (Causeway re-exports it as `project`). It reads
//! a project from its directory rather than from what the native game embeds,
//! so any project in the repository can be opened, not only Causeway.

// Offscreen, with a GPU and a file system: a native tool, as it was inside
// Causeway. In the browser it is empty rather than a build failure.
#![cfg(not(target_arch = "wasm32"))]

use std::{collections::BTreeMap, error::Error, fs, io::BufWriter, path::Path, time::Duration};

use sindri_assets::{AssetBytes, AssetDecoder, FontAssetDecoder};
use sindri_core::{AssetId, SceneEntityId, World};
use sindri_platform::{InputEvent, InputState, Key, MouseButton};
use sindri_render::TextRenderer;
use sindri_runtime::ProjectRun;
use sindri_runtime::project::{assets_root, files_under};
use sindri_scene::{SceneExtractor, TileSetBindings, measure_ui_text};

use sindri_runtime::Session;

mod draw;
mod models;

pub use draw::{DrawTimes, ProjectRenderer};

use sindri_gpu::{GpuContext, GpuRequestOptions};

pub use sindri_runtime::STEP;

/// A pixel over the middle of `name`, as the last step laid it out.
fn pixel_of(session: &Session, world: &World, name: &str, size: [f32; 2]) -> Option<[f32; 2]> {
    let entity = world.entity_for_source_id(&SceneEntityId::new(name).ok()?)?;
    let rect = session.screen_ui().rect(entity)?;
    let half = size[1] / 2.0;
    Some([
        size[0] / 2.0 + rect.center[0] * half,
        half - rect.center[1] * half,
    ])
}

pub struct ProjectPlayer {
    pub session: Session,
    pub world: World,
    pub input: InputState,
    pub scene: SceneExtractor,
    pub text: TextRenderer,
    /// The screen it plays on, in pixels.
    pub size: [f32; 2],
    /// The project's block and tile sets, which the browser host binds
    /// from the export manifest and the scripts and world read.
    pub tile_sets: TileSetBindings,
    /// How long it has played, which sprite shaders read as their clock.
    pub seconds: f32,
    /// Each step's report, by frame, while a caller is collecting them
    /// ([`Self::collecting`]); otherwise they are logged.
    collected: Option<Vec<(u32, sindri_runtime::StepReport)>>,
}

impl ProjectPlayer {
    pub const fn viewport(&self) -> weave::Viewport {
        weave::Viewport {
            width: self.size[0],
            height: self.size[1],
        }
    }

    /// One fixed step and one draw's worth of styling, as a host runs.
    pub fn step(&mut self) -> Result<(), Box<dyn Error>> {
        let frame = self.frame();
        let report = self.step_reported()?;
        match &mut self.collected {
            Some(collected) => collected.push((frame, report)),
            None => report.log(),
        }
        Ok(())
    }

    /// How many fixed steps it has played.
    #[must_use]
    pub fn frame(&self) -> u32 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frame = (self.seconds / STEP).round() as u32;
        frame
    }

    /// Keeps each step's report from now on, for [`Self::take_reports`],
    /// instead of logging it.
    pub fn collecting(&mut self) {
        self.collected.get_or_insert_with(Vec::new);
    }

    /// The reports kept since the last call, each with its frame.
    pub fn take_reports(&mut self) -> Vec<(u32, sindri_runtime::StepReport)> {
        self.collected
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default()
    }

    /// [`Self::step`], answering what the scripts printed and what failed
    /// rather than logging it: what a tool reporting a run reads.
    pub fn step_reported(&mut self) -> Result<sindri_runtime::StepReport, Box<dyn Error>> {
        let viewport = (self.size[0], self.size[1]);
        let report = self
            .session
            .step(&mut self.world, &self.input, viewport, STEP)?;
        self.input.begin_frame(Duration::from_secs_f32(STEP));
        self.seconds += STEP;
        let view = self.viewport();
        let undo = self.session.style(&mut self.world, view)?;
        let sizes = measure_ui_text(&self.world, self.scene.components(), &mut self.text)?;
        self.session.record_drawn(&self.world, view, sizes)?;
        if let Some(undo) = undo {
            undo.undo(&mut self.world);
        }
        Ok(report)
    }

    /// One fixed step of gameplay and nothing drawn: what a host's update
    /// does, with the draw left to whoever draws.
    pub fn advance(&mut self) -> Result<(), Box<dyn Error>> {
        let viewport = (self.size[0], self.size[1]);
        self.session
            .step(&mut self.world, &self.input, viewport, STEP)?
            .log();
        self.input.begin_frame(Duration::from_secs_f32(STEP));
        self.seconds += STEP;
        Ok(())
    }

    pub fn play(&mut self, seconds: f32) -> Result<(), Box<dyn Error>> {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = (seconds / STEP).ceil().max(1.0) as usize;
        for _ in 0..steps {
            self.step()?;
        }
        Ok(())
    }

    /// Moves the entity with source ID `name` to `at` in the world.
    fn put(&mut self, name: &str, at: [f32; 2], z: Option<f32>) -> Result<(), Box<dyn Error>> {
        let entity = self
            .world
            .entity_for_source_id(&SceneEntityId::new(name)?)
            .ok_or_else(|| format!("no entity {name}"))?;
        let place = self
            .world
            .get_mut(entity)
            .and_then(|data| data.transform_3d.as_mut())
            .ok_or_else(|| format!("{name} has no place"))?;
        place.position[0] = at[0];
        place.position[1] = at[1];
        if let Some(z) = z {
            place.position[2] = z;
        }
        Ok(())
    }

    fn point_at(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
        let [x, y] = pixel_of(&self.session, &self.world, name, self.size)
            .ok_or_else(|| format!("{name} is not laid out on screen"))?;
        eprintln!("{name} at ({x:.0}, {y:.0})");
        self.input.apply(InputEvent::PointerMoved { x, y });
        Ok(())
    }

    pub fn perform(&mut self, action: &str) -> Result<(), Box<dyn Error>> {
        let (verb, rest) = action.split_once(':').unwrap_or((action, ""));
        match verb {
            "click" => {
                self.point_at(rest)?;
                self.input
                    .apply(InputEvent::ButtonPressed(MouseButton::Left));
                self.step()?;
                self.input
                    .apply(InputEvent::ButtonReleased(MouseButton::Left));
                self.step()?;
            }
            "wheel" => {
                let (name, pixels) = rest.rsplit_once(':').ok_or("wheel:<entity>:<px>")?;
                self.point_at(name)?;
                let pixels: f32 = pixels.parse()?;
                self.input
                    .apply(InputEvent::Scrolled { x: 0.0, y: -pixels });
                self.step()?;
            }
            "key" => {
                let key = Key::from_name(rest).ok_or_else(|| format!("no key {rest}"))?;
                self.input.apply(InputEvent::KeyPressed(key));
                self.step()?;
                self.input.apply(InputEvent::KeyReleased(key));
                self.step()?;
            }
            // Held across the steps that follow, as a run is held.
            "hold" | "release" => {
                let key = Key::from_name(rest).ok_or_else(|| format!("no key {rest}"))?;
                self.input.apply(if verb == "hold" {
                    InputEvent::KeyPressed(key)
                } else {
                    InputEvent::KeyReleased(key)
                });
                self.step()?;
            }
            "type" => {
                for c in rest.chars() {
                    self.input.apply(InputEvent::TextInput(c));
                }
                self.step()?;
            }
            "wait" => self.play(rest.parse()?)?,
            // A pointer at a pixel, and the left button pressed or let
            // go there: enough to drag something across the world.
            "move" => {
                let (x, y) = rest.split_once(',').ok_or("move:<x>,<y>")?;
                let (x, y) = (x.parse()?, y.parse()?);
                self.input.apply(InputEvent::PointerMoved { x, y });
                self.step()?;
            }
            "down" => {
                self.input
                    .apply(InputEvent::ButtonPressed(MouseButton::Left));
                self.step()?;
            }
            "up" => {
                self.input
                    .apply(InputEvent::ButtonReleased(MouseButton::Left));
                self.step()?;
            }
            "set" => {
                let (name, value) = rest.split_once('=').ok_or("set:<name>=<value>")?;
                self.session.set_board(name, value.parse()?);
                self.step()?;
            }
            // An entity put at a place in the world: somewhere a walk would
            // take too long to reach.
            "place" => {
                let (name, at) = rest.split_once('=').ok_or("place:<entity>=<x>,<y>[,<z>]")?;
                let parts: Vec<f32> = at.split(',').map(str::parse).collect::<Result<_, _>>()?;
                let [x, y, rest @ ..] = parts.as_slice() else {
                    return Err("place:<entity>=<x>,<y>[,<z>]".into());
                };
                self.put(name, [*x, *y], rest.first().copied())?;
                self.step()?;
            }
            // An entity put just in front of the first thing of a name that
            // has been made: a hamlet's lamp, say, once the scatter grew it.
            "near" => {
                let (name, target) = rest.split_once('=').ok_or("near:<entity>=<name>")?;
                let found = self
                    .world
                    .entities()
                    .find(|(_, data)| data.name.as_deref() == Some(target))
                    .map(|(entity, _)| entity)
                    .ok_or_else(|| format!("nothing named {target} has been made"))?;
                let at = self
                    .world
                    .world_transform(found)
                    .ok_or("it has no place")?
                    .position;
                eprintln!("{target} at ({:.1}, {:.1})", at[0], at[1]);
                self.put(name, [at[0], at[1] - 0.8], None)?;
                self.step()?;
            }
            _ => return Err(format!("unknown step {action}").into()),
        }
        Ok(())
    }
}

/// Everything opened: the player, and the images and sheets a renderer
/// binds, by asset ID.
pub struct OpenedProject {
    pub player: ProjectPlayer,
    pub images: BTreeMap<String, Vec<u8>>,
    pub sheets: BTreeMap<String, Vec<u8>>,
    /// The project's sprite shaders, as source.
    pub shaders: BTreeMap<String, Vec<u8>>,
}

/// Opens the project at `project` to play on a screen of `size` pixels:
/// `sindri_runtime::ProjectRun`'s run, with the fonts and images a drawing
/// host adds.
pub fn open(project: &Path, size: [f32; 2]) -> Result<OpenedProject, Box<dyn Error>> {
    open_on(project, None, size)
}

/// The same, started on another of the project's scenes, named as `Scene.go`
/// names it: how a capture or a test reaches a level that sits behind a title
/// screen without playing its way there.
pub fn open_scene(
    project: &Path,
    scene: &str,
    size: [f32; 2],
) -> Result<OpenedProject, Box<dyn Error>> {
    open_on(project, Some(scene), size)
}

fn open_on(
    project: &Path,
    entry: Option<&str>,
    size: [f32; 2],
) -> Result<OpenedProject, Box<dyn Error>> {
    let mut run = match entry {
        Some(scene) => ProjectRun::open_scene(project, scene, size)?,
        None => ProjectRun::open(project, size)?,
    };
    let assets = assets_root(project);
    // Every model the project holds, as the player binds them: a scene
    // reached later, or a prefab spawned later, draws without a load.
    models::bind(
        &assets,
        files_under(&assets, ".glb").into_keys().collect(),
        &mut run.scene,
    )?;
    let mut text = TextRenderer::new();
    for (id, bytes) in files_under(&assets, ".ttf") {
        let asset = FontAssetDecoder.decode(AssetBytes::new(id.parse::<AssetId>()?, bytes))?;
        text.bind_font(&id, asset.family(), asset.bytes().to_vec());
    }
    let tile_sets = run.session.tile_sets().clone();
    let player = ProjectPlayer {
        session: run.session,
        world: run.world,
        input: run.input,
        scene: run.scene,
        text,
        size,
        tile_sets,
        seconds: 0.0,
        collected: None,
    };
    Ok(OpenedProject {
        player,
        images: files_under(&assets, ".png"),
        sheets: files_under(&assets, ".sheet"),
        shaders: files_under(&assets, ".wgsl"),
    })
}

/// Draws the player's world once, offscreen, and writes it to `path`.
pub async fn photograph(
    opened: &mut OpenedProject,
    size: (u32, u32),
    path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut photographer = Photographer::new(opened, size).await?;
    photographer.shoot(&mut opened.player, path)
}

/// A GPU and a renderer kept for a run's photographs, so a run that takes
/// several pays for the device once.
pub struct Photographer {
    gpu: GpuContext,
    renderer: ProjectRenderer,
    size: (u32, u32),
}

impl Photographer {
    /// A renderer for `opened`'s pictures, `size` pixels.
    ///
    /// # Errors
    /// No GPU adapter, or the project's pictures or shaders do not load.
    pub async fn new(opened: &OpenedProject, size: (u32, u32)) -> Result<Self, Box<dyn Error>> {
        let instance = wgpu::Instance::default();
        let gpu = GpuContext::request(&instance, None, &GpuRequestOptions::default()).await?;
        let mut renderer = ProjectRenderer::new(&gpu, &opened.images, &opened.sheets, size)?;
        renderer.bind_shaders(&opened.shaders)?;
        Ok(Self {
            gpu,
            renderer,
            size,
        })
    }

    /// The player's world as it is now, as RGBA rows.
    ///
    /// # Errors
    /// The draw or the readback fails.
    pub fn pixels(&mut self, player: &mut ProjectPlayer) -> Result<Vec<u8>, Box<dyn Error>> {
        self.renderer.draw(player, &self.gpu)?;
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("project capture readback"),
            });
        let readback = self
            .renderer
            .target()
            .copy_to_buffer(&self.gpu.device, &mut encoder)?;
        self.gpu.queue.submit([encoder.finish()]);
        Ok(readback.read_rgba8(&self.gpu.device)?)
    }

    /// The player's world as it is now, written to `path` as a PNG.
    ///
    /// # Errors
    /// The draw, the readback or the write fails.
    pub fn shoot(&mut self, player: &mut ProjectPlayer, path: &Path) -> Result<(), Box<dyn Error>> {
        let pixels = self.pixels(player)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut png = png::Encoder::new(
            BufWriter::new(fs::File::create(path)?),
            self.size.0,
            self.size.1,
        );
        png.set_color(png::ColorType::Rgba);
        png.set_depth(png::BitDepth::Eight);
        png.write_header()?.write_image_data(&pixels)?;
        Ok(())
    }
}
