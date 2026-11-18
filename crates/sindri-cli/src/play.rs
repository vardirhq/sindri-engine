//! `sindri play run`: a project played headless, at a fixed step, with
//! scripted input, reported as JSON, with pictures when asked.
//!
//! What it gives an agent is the one thing files cannot: whether the change
//! did what was meant. Press D for a second and read where the player went;
//! press F and read whether the board says a fire was built; take a shot and
//! look. The fixed step makes a run reproducible, so it is a check rather
//! than an anecdote (`docs/agent-native-direction.md`, section 3).
//!
//! It plays through `sindri-capture`'s player, the one the browser's draw is
//! mirrored by: a styled game is styled and laid out every step, so a click
//! lands where a person's would and a picture is what they would see.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sindri_capture::{OpenedProject, Photographer, open, open_scene};
use sindri_core::SceneEntityId;

use crate::debug_view::{self, Layers, Probe};
use crate::expect::{self, Checked};
use crate::film::{self, Film};
use crate::report::SCHEMA_VERSION;

/// What `sindri play run` answers.
#[derive(Debug, Serialize)]
pub struct PlayReport {
    pub schema_version: u32,
    pub project: String,
    /// No script failed and nothing stopped the run.
    pub ok: bool,
    /// The scene it began in and the one it ended in.
    pub scene: Option<String>,
    pub ended_in: Option<String>,
    pub frames: u32,
    pub seconds: f32,
    /// Everything the scripts left on the shared board (`Game.score` is
    /// `score`), at the end.
    pub board: BTreeMap<String, f64>,
    /// Where each tracked entity was at the end, by stable ID.
    pub tracked: BTreeMap<String, Option<[f32; 2]>>,
    /// The pictures taken, in order.
    pub shots: Vec<Shot>,
    /// What scripts printed, what failed in them and what else went wrong,
    /// each with the frame it happened on.
    pub printed: Vec<Event>,
    pub failures: Vec<Event>,
    pub problems: Vec<Event>,
    /// Each `expect:` step, where it was reached and what was there.
    pub expectations: Vec<Checked>,
    /// Each `probe:` step: what was at that pixel.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub probes: Vec<Probe>,
    /// Why the run stopped early, if it did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Event {
    pub frame: u32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct Shot {
    pub frame: u32,
    pub path: String,
    /// For a `film:` sheet, the frame each picture on it was taken at, in
    /// order; empty for a single shot.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub frames: Vec<u32>,
}

/// What to play: where, for how long, pressing what, watching whom, and
/// where pictures go.
pub struct Play<'a> {
    pub project: &'a Path,
    pub scene: Option<&'a str>,
    pub steps: &'a [String],
    pub track: &'a [String],
    /// The folder `shot` steps write to.
    pub shots: &'a Path,
    /// The screen it is played and photographed at, in pixels.
    pub size: [u32; 2],
}

/// The verbs a step may start with; checked before anything is opened, so a
/// typo is a usage error rather than a run that stops halfway.
const VERBS: [&str; 13] = [
    "wait", "key", "hold", "release", "set", "click", "wheel", "type", "move", "down", "up",
    "place", "near",
];

/// Plays the project.
///
/// # Errors
/// A project that will not open, or a step that does not read.
pub fn play(request: &Play<'_>) -> Result<PlayReport, String> {
    for step in request.steps {
        let (verb, rest) = step.split_once(':').unwrap_or((step.as_str(), ""));
        match verb {
            "shot" => {}
            "expect" => {
                expect::parse(rest)?;
            }
            "film" => {
                Film::parse(rest)?;
            }
            "debug" => {
                Layers::parse(rest)?;
            }
            "probe" => {
                debug_view::parse_pixel(rest)?;
            }
            verb if VERBS.contains(&verb) => {}
            _ => {
                return Err(format!(
                    "{step}: a step is one of shot, film, expect, debug, probe, {}",
                    VERBS.join(", ")
                ));
            }
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let screen = [request.size[0] as f32, request.size[1] as f32];
    let mut opened = match request.scene {
        Some(scene) => open_scene(request.project, scene, screen),
        None => open(request.project, screen),
    }
    .map_err(|error| error.to_string())?;
    opened.player.collecting();
    let scene = opened.player.session.scene().map(str::to_owned);
    let mut run = Run {
        opened,
        photographer: None,
        printed: Vec::new(),
        failures: Vec::new(),
        problems: Vec::new(),
        shots: Vec::new(),
        expectations: Vec::new(),
        debug: Layers::default(),
        probes: Vec::new(),
    };
    let mut stopped = None;
    for (index, step) in request.steps.iter().enumerate() {
        if let Err(error) = run.perform(step, index, request) {
            stopped = Some(format!("{step}: {error}"));
            break;
        }
    }
    run.gather();
    let player = &run.opened.player;
    let board = player
        .session
        .scripts()
        .blackboard()
        .notes()
        .map(|(name, value)| (name.to_owned(), value))
        .collect();
    let tracked = request
        .track
        .iter()
        .map(|id| {
            let at = SceneEntityId::new(id.clone())
                .ok()
                .and_then(|id| player.world.entity_for_source_id(&id))
                .and_then(|entity| player.world.world_transform(entity))
                .map(|place| [place.position[0], place.position[1]]);
            (id.clone(), at)
        })
        .collect();
    Ok(PlayReport {
        schema_version: SCHEMA_VERSION,
        project: request.project.display().to_string(),
        ok: run.failures.is_empty()
            && stopped.is_none()
            && run.expectations.iter().all(|checked| checked.passed),
        scene,
        ended_in: player.session.scene().map(str::to_owned),
        frames: player.frame(),
        seconds: player.seconds,
        board,
        tracked,
        shots: run.shots,
        printed: run.printed,
        failures: run.failures,
        problems: run.problems,
        expectations: run.expectations,
        probes: run.probes,
        stopped,
    })
}

struct Run {
    opened: OpenedProject,
    /// Made at the first `shot`, so a run that takes none needs no GPU.
    photographer: Option<Photographer>,
    printed: Vec<Event>,
    failures: Vec<Event>,
    problems: Vec<Event>,
    shots: Vec<Shot>,
    expectations: Vec<Checked>,
    /// The overlays pictures carry from here on.
    debug: Layers,
    probes: Vec<Probe>,
}

impl Run {
    fn perform(&mut self, step: &str, index: usize, request: &Play<'_>) -> Result<(), String> {
        let (verb, name) = step.split_once(':').unwrap_or((step, ""));
        if verb == "expect" {
            self.gather();
            let (passed, actual) = expect::parse(name)?.check(&*self);
            self.expectations.push(Checked {
                frame: self.opened.player.frame(),
                step: step.to_owned(),
                passed,
                actual,
            });
            return Ok(());
        }
        if verb == "debug" {
            self.debug = Layers::parse(name)?;
            return Ok(());
        }
        if verb == "probe" {
            let pixel = debug_view::parse_pixel(name)?;
            self.probes.push(debug_view::probe(
                &self.opened.player,
                pixel,
                self.opened.player.frame(),
            ));
            return Ok(());
        }
        if verb == "film" {
            return self.film(&Film::parse(name)?, index, request);
        }
        if verb != "shot" {
            let done = self
                .opened
                .player
                .perform(step)
                .map_err(|error| error.to_string());
            self.gather();
            return done;
        }
        self.camera(request)?;
        let file = if name.is_empty() {
            format!("shot-{}.png", index + 1)
        } else {
            format!("{name}.png")
        };
        let path: PathBuf = request.shots.join(file);
        let pixels = self.picture()?;
        let image = image::RgbaImage::from_raw(request.size[0], request.size[1], pixels)
            .ok_or("the picture was not the size of the run")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        image.save(&path).map_err(|error| error.to_string())?;
        self.shots.push(Shot {
            frame: self.opened.player.frame(),
            path: path.display().to_string(),
            frames: Vec::new(),
        });
        Ok(())
    }

    /// The frame as it is now, with the debug overlays asked for drawn on it.
    fn picture(&mut self) -> Result<Vec<u8>, String> {
        let mut pixels = self
            .photographer
            .as_mut()
            .expect("made before any picture")
            .pixels(&mut self.opened.player)
            .map_err(|error| error.to_string())?;
        if self.debug.any() {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let size = [
                self.opened.player.size[0] as u32,
                self.opened.player.size[1] as u32,
            ];
            debug_view::draw(&mut pixels, size, &self.opened.player, self.debug);
        }
        Ok(pixels)
    }

    /// The photographer, made at the first picture, so a run that takes
    /// none needs no GPU.
    fn camera(&mut self, request: &Play<'_>) -> Result<(), String> {
        if self.photographer.is_none() {
            let photographer = pollster::block_on(Photographer::new(
                &self.opened,
                (request.size[0], request.size[1]),
            ))
            .map_err(|error| format!("a picture needs a GPU adapter: {error}"))?;
            self.photographer = Some(photographer);
        }
        Ok(())
    }

    /// A `film:` step: frames taken across its span while the run plays on,
    /// written as one sheet.
    fn film(&mut self, film: &Film, index: usize, request: &Play<'_>) -> Result<(), String> {
        self.camera(request)?;
        let mut pixels = Vec::new();
        let mut frames = Vec::new();
        for taken in 0..film.count {
            if taken > 0 {
                self.opened
                    .player
                    .perform(&format!("wait:{}", film.between()))
                    .map_err(|error| error.to_string())?;
                self.gather();
            }
            frames.push(self.opened.player.frame());
            pixels.push(self.picture()?);
        }
        let sheet = film.name.as_ref().map_or_else(
            || format!("film-{}.png", index + 1),
            |name| format!("{name}.png"),
        );
        let path = request.shots.join(sheet);
        film::write(pixels, request.size, &path)?;
        self.shots.push(Shot {
            frame: frames[0],
            path: path.display().to_string(),
            frames,
        });
        Ok(())
    }

    /// The steps' reports since the last call, sorted into what was printed,
    /// what failed and what else went wrong.
    fn gather(&mut self) {
        for (frame, report) in self.opened.player.take_reports() {
            let event = |message: String| Event { frame, message };
            self.printed.extend(
                report
                    .scripts
                    .printed
                    .into_iter()
                    .map(|printed| event(printed.message)),
            );
            self.failures.extend(
                report
                    .scripts
                    .failures
                    .into_iter()
                    .map(|failure| event(failure.to_string())),
            );
            self.problems.extend(report.problems.into_iter().map(event));
        }
    }
}

/// What an expectation is checked against: the run as it is now.
impl expect::State for Run {
    fn board(&self, name: &str) -> Option<f64> {
        self.opened
            .player
            .session
            .scripts()
            .blackboard()
            .notes()
            .find(|(noted, _)| *noted == name)
            .map(|(_, value)| value)
    }

    fn position(&self, entity: &str) -> Option<[f32; 2]> {
        let world = &self.opened.player.world;
        let id = SceneEntityId::new(entity.to_owned()).ok()?;
        let place = world.world_transform(world.entity_for_source_id(&id)?)?;
        Some([place.position[0], place.position[1]])
    }

    fn scene(&self) -> Option<String> {
        self.opened.player.session.scene().map(str::to_owned)
    }

    fn printed(&self) -> Vec<String> {
        self.printed
            .iter()
            .map(|event| event.message.clone())
            .collect()
    }
}

impl PlayReport {
    /// The report as a person reads it.
    #[must_use]
    pub fn human(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "played {} frames ({:.2} s) from {} to {}",
            self.frames,
            self.seconds,
            self.scene.as_deref().unwrap_or("?"),
            self.ended_in.as_deref().unwrap_or("?")
        );
        for (kind, events) in [
            ("printed", &self.printed),
            ("failed", &self.failures),
            ("problem", &self.problems),
        ] {
            for event in events {
                let _ = writeln!(out, "  frame {}: {kind}: {}", event.frame, event.message);
            }
        }
        for shot in &self.shots {
            if shot.frames.is_empty() {
                let _ = writeln!(out, "  frame {}: shot {}", shot.frame, shot.path);
            } else {
                let _ = writeln!(out, "  frames {:?}: film {}", shot.frames, shot.path);
            }
        }
        for checked in &self.expectations {
            let mark = if checked.passed { "pass" } else { "FAIL" };
            let _ = writeln!(
                out,
                "  frame {}: {mark} {} (was {})",
                checked.frame, checked.step, checked.actual
            );
        }
        for (id, at) in &self.tracked {
            match at {
                Some([x, y]) => {
                    let _ = writeln!(out, "  {id} at ({x:.2}, {y:.2})");
                }
                None => {
                    let _ = writeln!(out, "  {id} is not in the world");
                }
            }
        }
        if !self.board.is_empty() {
            let board = self
                .board
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join(" ");
            let _ = writeln!(out, "  board: {board}");
        }
        if let Some(why) = &self.stopped {
            let _ = writeln!(out, "  stopped: {why}");
        }
        let _ = writeln!(out, "{}", if self.ok { "ok" } else { "failed" });
        out
    }
}
