//! Any project played offscreen the way the browser plays it: the same
//! session, the same Weave styling for the viewport, the same draw, with
//! clicks, keys and wheel delivered as a host delivers them.
//!
//! What `project-capture` photographs and `project-benchmark` times. It reads
//! a project from its directory rather than from what the native game embeds,
//! so any project in the repository can be opened, not only Causeway.

use std::{collections::BTreeMap, error::Error, path::Path, time::Duration};

use sindri_assets::{AssetBytes, AssetDecoder, FontAssetDecoder};
use sindri_core::{AssetId, SceneEntityId, World};
use sindri_platform::{InputEvent, InputState, Key, MouseButton};
use sindri_render::TextRenderer;
use sindri_runtime::ProjectRun;
use sindri_runtime::project::{assets_root, files_under};
use sindri_scene::{SceneExtractor, TileSetBindings, measure_ui_text};

use crate::Session;

mod draw;
mod models;

pub use draw::{DrawTimes, ProjectRenderer};

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
        self.advance()?;
        let view = self.viewport();
        let undo = self.session.style(&mut self.world, view)?;
        let sizes = measure_ui_text(&self.world, self.scene.components(), &mut self.text)?;
        self.session.record_drawn(&self.world, view, sizes)?;
        if let Some(undo) = undo {
            undo.undo(&mut self.world);
        }
        Ok(())
    }

    /// One fixed step of gameplay and nothing drawn: what a host's update
    /// does, with the draw left to whoever draws.
    pub fn advance(&mut self) -> Result<(), Box<dyn Error>> {
        let viewport = (self.size[0], self.size[1]);
        self.session
            .step(&mut self.world, &self.input, viewport, STEP)?
            .log();
        self.input.begin_frame(Duration::from_secs_f32(STEP));
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

    fn point_at(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
        let [x, y] = pixel_of(&self.session, &self.world, name, self.size)
            .ok_or_else(|| format!("{name} is not laid out on screen"))?;
        println!("{name} at ({x:.0}, {y:.0})");
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
}

/// Opens the project at `project` to play on a screen of `size` pixels:
/// `sindri_runtime::ProjectRun`'s run, with the fonts and images a drawing
/// host adds.
pub fn open(project: &Path, size: [f32; 2]) -> Result<OpenedProject, Box<dyn Error>> {
    let mut run = ProjectRun::open(project, size)?;
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
    };
    Ok(OpenedProject {
        player,
        images: files_under(&assets, ".png"),
        sheets: files_under(&assets, ".sheet"),
    })
}
