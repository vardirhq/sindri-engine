//! `sindri play run`: a project played headless, at a fixed step, with
//! scripted input, and what happened reported as JSON.
//!
//! What it gives an agent is the one thing files cannot: whether the change
//! did what was meant. Press D for a second and read where the player went;
//! press F and read whether the board says a fire was built. The fixed step
//! makes a run reproducible, so it is a check rather than an anecdote
//! (`docs/agent-native-direction.md`, section 3).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use sindri_platform::{InputEvent, Key};
use sindri_runtime::project::ProjectRun;

use crate::report::SCHEMA_VERSION;

/// One fixed step: sixty to the second, as every game's test plays.
const STEP: f32 = 1.0 / 60.0;
/// A screen to lay a styled game out on, as a phone held sideways.
const SCREEN: [f32; 2] = [1280.0, 720.0];

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
    /// What scripts printed, what failed in them and what else went wrong,
    /// each with the frame it happened on.
    pub printed: Vec<Event>,
    pub failures: Vec<Event>,
    pub problems: Vec<Event>,
    /// Why the run stopped early, if it did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Event {
    pub frame: u32,
    pub message: String,
}

/// What to play: where, for how long, pressing what, watching whom.
pub struct Play<'a> {
    pub project: &'a Path,
    pub scene: Option<&'a str>,
    pub steps: &'a [String],
    pub track: &'a [String],
}

/// Plays the project.
///
/// # Errors
/// A project that will not open, or a step that does not read.
pub fn play(request: &Play<'_>) -> Result<PlayReport, String> {
    let steps = request
        .steps
        .iter()
        .map(|step| parse_step(step))
        .collect::<Result<Vec<_>, _>>()?;
    let run = match request.scene {
        Some(scene) => ProjectRun::open_scene(request.project, scene, SCREEN),
        None => ProjectRun::open(request.project, SCREEN),
    }?;
    let mut player = Player {
        scene: run.session.scene().map(str::to_owned),
        run,
        frame: 0,
        printed: Vec::new(),
        failures: Vec::new(),
        problems: Vec::new(),
    };
    let mut stopped = None;
    for step in &steps {
        if let Err(error) = player.perform(step) {
            stopped = Some(error);
            break;
        }
    }
    let Player {
        run,
        frame,
        printed,
        failures,
        problems,
        scene,
    } = player;
    let board = run
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
            (
                id.clone(),
                run.entity(id).map(|entity| run.position(entity)),
            )
        })
        .collect();
    Ok(PlayReport {
        schema_version: SCHEMA_VERSION,
        project: request.project.display().to_string(),
        ok: failures.is_empty() && stopped.is_none(),
        scene,
        ended_in: run.session.scene().map(str::to_owned),
        frames: frame,
        #[allow(clippy::cast_precision_loss)]
        seconds: frame as f32 * STEP,
        board,
        tracked,
        printed,
        failures,
        problems,
        stopped,
    })
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

/// One thing to do: `wait:<seconds>`, `key:<Key>` (tapped), `hold:<Key>` and
/// `release:<Key>` (held across what follows), `set:<name>=<value>` (a board
/// value, as a test asks a game for a state).
#[derive(Debug, PartialEq)]
enum Step {
    Wait(u32),
    Tap(Key),
    Hold(Key),
    Release(Key),
    Set(String, f32),
}

fn parse_step(text: &str) -> Result<Step, String> {
    let (verb, rest) = text.split_once(':').unwrap_or((text, ""));
    let key = || Key::from_name(rest).ok_or_else(|| format!("{text}: no key named {rest}"));
    Ok(match verb {
        "wait" => {
            let seconds: f32 = rest
                .parse()
                .map_err(|_| format!("{text}: wait takes seconds"))?;
            if !seconds.is_finite() || seconds < 0.0 {
                return Err(format!("{text}: wait takes a positive number of seconds"));
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Step::Wait((seconds / STEP).round() as u32)
        }
        "key" => Step::Tap(key()?),
        "hold" => Step::Hold(key()?),
        "release" => Step::Release(key()?),
        "set" => {
            let (name, value) = rest
                .split_once('=')
                .ok_or_else(|| format!("{text}: set takes <name>=<value>"))?;
            let value = value
                .parse()
                .map_err(|_| format!("{text}: {value} is not a number"))?;
            Step::Set(name.to_owned(), value)
        }
        _ => {
            return Err(format!(
                "{text}: a step is wait:<seconds>, key:<Key>, hold:<Key>, release:<Key> or set:<name>=<value>"
            ));
        }
    })
}

struct Player {
    run: ProjectRun,
    scene: Option<String>,
    frame: u32,
    printed: Vec<Event>,
    failures: Vec<Event>,
    problems: Vec<Event>,
}

impl Player {
    fn perform(&mut self, step: &Step) -> Result<(), String> {
        match step {
            Step::Wait(frames) => {
                for _ in 0..*frames {
                    self.tick()?;
                }
            }
            Step::Tap(key) => {
                self.run.input.apply(InputEvent::KeyPressed(*key));
                self.tick()?;
                self.run.input.apply(InputEvent::KeyReleased(*key));
                self.tick()?;
            }
            Step::Hold(key) => self.run.input.apply(InputEvent::KeyPressed(*key)),
            Step::Release(key) => self.run.input.apply(InputEvent::KeyReleased(*key)),
            Step::Set(name, value) => self.run.set_board(name, *value),
        }
        Ok(())
    }

    fn tick(&mut self) -> Result<(), String> {
        let report = self.run.step(STEP)?;
        let frame = self.frame;
        self.frame += 1;
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_read_as_written() {
        assert_eq!(parse_step("wait:1"), Ok(Step::Wait(60)));
        assert_eq!(parse_step("hold:D"), Ok(Step::Hold(Key::D)));
        assert_eq!(parse_step("key:Space"), Ok(Step::Tap(Key::Space)));
        assert_eq!(
            parse_step("set:wood=4"),
            Ok(Step::Set("wood".to_owned(), 4.0))
        );
        assert!(parse_step("wait:-1").is_err());
        assert!(parse_step("hold:Nope").is_err());
        assert!(parse_step("jump").is_err());
    }
}
