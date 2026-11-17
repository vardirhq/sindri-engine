//! The command itself, natively.

use crate::{check, mcp, new, play};

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HELP: &str = r#"sindri: check, play and photograph a Sindri project from a shell.

USAGE
  sindri project new PATH [--name NAME] [--format human|json]
  sindri project check [PATH] [--format human|json]
  sindri play run [PATH] [--scene NAME] [--steps "STEP ..."] [--track ID]...
                  [--shots DIR] [--size WxH] [--format human|json]
  sindri render capture [PATH] --out FILE.png [--scene NAME] [--steps "..."]
                  [--size WxH] [--format human|json]
  sindri capabilities [NAME]
  sindri mcp

  PATH is the project folder, the one holding sindri.toml (default: here).

COMMANDS
  project new     Make a project in PATH, as the editor makes one, with a
                  guide for coding agents beside it (AGENTS.md, CLAUDE.md and
                  a skill), the scene and prefab schemas in .sindri/, and VS
                  Code settings that validate scenes against them.

  project check   Read every scene, prefab, Decay script and Weave stylesheet,
                  and sindri.toml, the way a run reads them, and report every
                  problem with a stable code. Exits 1 when there are errors;
                  warnings do not fail it.

  play run        Play the project headless at 60 steps a second, from its
                  main scene or --scene, styled and laid out as it is drawn,
                  doing --steps in order. Report the shared board, where each
                  --track entity ended up, the pictures taken, and what
                  scripts printed and failed. Exits 1 when a script failed or
                  a step could not be done.

  render capture  The same run, then one picture written to --out.

  capabilities    What this build can be told, as JSON: with no NAME, the
                  list; scene and prefab are JSON Schemas for those files
                  (point an editor's json.schemas at them), components every
                  component's fields and meanings, decay the script API,
                  decay-language the language, weave every style property.

  mcp             Serve project_check, play_run, capture and capabilities
                  as MCP tools on standard input and output, for an agent
                  with no shell.
                  Read-only: the tools answer with the commands' own JSON,
                  and capture with the pictures themselves.

  help            This text.

STEPS
  wait:<seconds>        play on
  key:<Key>             tap a key (key:Space, key:F, key:ArrowLeft)
  hold:<Key>            press a key until its release
  release:<Key>
  set:<name>=<number>   write a shared board value (set:wood=4)
  click:<entity>        press and release over a laid-out element, by ID
  type:<text>           type characters into whatever has focus
  place:<entity>=<x>,<y>  move an entity, by ID, to a place in the world
  near:<entity>=<name>  move an entity just in front of the first thing of
                        that name, once something has made it
  shot                  a picture, to --shots (default .), as shot-N.png
  shot:<name>           the same, as <name>.png
  for example: --steps "wait:0.5 hold:D wait:1 release:D shot:walked"

--format json writes one JSON document with schema_version; it is the
contract. --format human, the default, is rendered from it."#;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Human,
    Json,
}

pub fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("sindri: {message}\n\n{HELP}");
            ExitCode::from(2)
        }
    }
}

/// What the command line asked for: the command's words, and its flags.
struct Options {
    words: Vec<String>,
    format: Format,
    scene: Option<String>,
    steps: Vec<String>,
    track: Vec<String>,
    shots: PathBuf,
    out: Option<PathBuf>,
    size: [u32; 2],
    name: Option<String>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            words: Vec::new(),
            format: Format::Human,
            scene: None,
            steps: Vec::new(),
            track: Vec::new(),
            shots: PathBuf::from("."),
            out: None,
            size: [1280, 720],
            name: None,
        };
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            let mut value = |flag: &str| {
                rest.next()
                    .cloned()
                    .ok_or_else(|| format!("{flag} wants a value"))
            };
            match arg.as_str() {
                "--format" => {
                    options.format = match value("--format")?.as_str() {
                        "json" => Format::Json,
                        "human" => Format::Human,
                        other => return Err(format!("--format takes human or json, not {other}")),
                    };
                }
                "--scene" => options.scene = Some(value("--scene")?),
                "--steps" => options
                    .steps
                    .extend(value("--steps")?.split_whitespace().map(str::to_owned)),
                "--track" => options.track.push(value("--track")?),
                "--shots" => options.shots = PathBuf::from(value("--shots")?),
                "--out" => options.out = Some(PathBuf::from(value("--out")?)),
                "--size" => options.size = parse_size(&value("--size")?)?,
                "--name" => options.name = Some(value("--name")?),
                "-h" | "--help" => options.words.insert(0, "help".to_owned()),
                flag if flag.starts_with('-') => return Err(format!("unknown flag {flag}")),
                word => options.words.push(word.to_owned()),
            }
        }
        Ok(options)
    }

    /// Writes a report in the asked-for format, and answers its exit code.
    fn report(
        &self,
        json_form: &impl serde::Serialize,
        human: &str,
        ok: bool,
    ) -> Result<ExitCode, String> {
        match self.format {
            Format::Json => println!("{}", json(json_form)?),
            Format::Human => print!("{human}"),
        }
        Ok(if ok {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        })
    }

    /// A run of the project, as `play run` and `render capture` ask for it.
    fn play(&self, project: &Path, steps: &[String], shots: &Path) -> Result<ExitCode, String> {
        let report = play::play(&play::Play {
            project,
            scene: self.scene.as_deref(),
            steps,
            track: &self.track,
            shots,
            size: self.size,
        })?;
        self.report(&report, &report.human(), report.ok)
    }
}

/// The project folder a command's target names, which must hold `sindri.toml`.
fn project(target: &[&str]) -> Result<PathBuf, String> {
    let project = match target {
        [] => PathBuf::from("."),
        [path] => PathBuf::from(path),
        _ => return Err("give one project folder".to_owned()),
    };
    if project.join("sindri.toml").is_file() {
        Ok(project)
    } else {
        Err(format!(
            "{} holds no sindri.toml; give the project folder",
            project.display()
        ))
    }
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let options = Options::parse(args)?;
    let words: Vec<&str> = options.words.iter().map(String::as_str).collect();
    match words.as_slice() {
        [] | ["help", ..] => {
            println!("{HELP}");
            Ok(ExitCode::SUCCESS)
        }
        ["project", "new", rest @ ..] => {
            if rest.len() > 1 {
                return Err("project new takes one folder".to_owned());
            }
            let (root, name) = new::target(rest.first().copied(), options.name.as_deref())?;
            let report = new::new(&root, &name)?;
            options.report(&report, &report.human(), true)
        }
        ["project", "check", target @ ..] => {
            let report = check::check(&project(target)?);
            options.report(&report, &report.human(), report.ok)
        }
        ["capabilities"] => {
            let names: Vec<_> = sindri_capabilities::REFERENCES
                .iter()
                .map(|(name, about)| serde_json::json!({ "name": name, "about": about }))
                .collect();
            let human = sindri_capabilities::REFERENCES.iter().fold(
                String::new(),
                |mut human, (name, about)| {
                    let _ = writeln!(human, "{name:<16}{about}");
                    human
                },
            );
            let listed = serde_json::json!({ "schema_version": 1, "capabilities": names });
            options.report(&listed, &human, true)
        }
        ["capabilities", name] => {
            let reference = sindri_capabilities::reference(name)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| {
                    format!("no capability document {name}; `sindri capabilities` lists them")
                })?;
            // The document is JSON whatever the format: a schema rendered
            // for a person is no longer a schema.
            // Written rather than printed, so a reader that stops early,
            // such as `head`, ends the command instead of panicking it.
            let text = json(&reference)? + "\n";
            let _ = std::io::Write::write_all(&mut std::io::stdout(), text.as_bytes());
            Ok(ExitCode::SUCCESS)
        }
        ["mcp"] => {
            mcp::serve();
            Ok(ExitCode::SUCCESS)
        }
        ["play", "run", target @ ..] => {
            options.play(&project(target)?, &options.steps, &options.shots)
        }
        ["render", "capture", target @ ..] => {
            let project = project(target)?;
            let out = options
                .out
                .as_ref()
                .ok_or("render capture needs --out FILE.png")?;
            let name = out
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or("--out names a file")?;
            let mut steps = options.steps.clone();
            steps.push(format!("shot:{name}"));
            let folder = out
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .map_or_else(|| PathBuf::from("."), PathBuf::from);
            options.play(&project, &steps, &folder)
        }
        other => Err(format!("unknown command {}", other.join(" "))),
    }
}

/// `1280x720` as pixels.
fn parse_size(text: &str) -> Result<[u32; 2], String> {
    let (width, height) = text
        .split_once('x')
        .ok_or_else(|| format!("--size takes WIDTHxHEIGHT, not {text}"))?;
    let pixels = |side: &str| {
        side.parse::<u32>()
            .ok()
            .filter(|pixels| (16..=4096).contains(pixels))
            .ok_or_else(|| format!("--size: {side} is not 16 to 4096 pixels"))
    };
    Ok([pixels(width)?, pixels(height)?])
}

fn json(value: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|error| error.to_string())
}
