//! The command itself, natively.

use crate::{check, install, mcp, new, play, query};

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HELP: &str = r#"sindri: check, play and photograph a Sindri project from a shell.

USAGE
  sindri project new PATH [--name NAME] [--format human|json]
  sindri project check [PATH] [--format human|json]
  sindri project overview [PATH]
  sindri scene outline [PATH] [--scene NAME]
  sindri entity find [PATH] [--component TYPE] [--name TEXT] [--script NAME]
                  [--prefab FILE] [--scene NAME]
  sindri entity show [PATH] --id ID [--scene NAME]
  sindri asset uses [PATH] --asset FILE
  sindri play run [PATH] [--scene NAME] [--steps "STEP ..."] [--track ID]...
                  [--shots DIR] [--size WxH] [--format human|json]
  sindri render capture [PATH] --out FILE.png [--scene NAME] [--steps "..."]
                  [--size WxH] [--format human|json]
  sindri capabilities [NAME]
  sindri mcp
  sindri mcp install [PATH] [--client NAME] [--scope user|project] [--dry-run]

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

  project overview  What the project holds: its scenes and prefabs with
                  their sizes, its scripts and what each declares, its
                  stylesheets, how often each component is used, its files.

  scene outline   A scene or prefab (the main scene unless --scene) as a
                  tree, one line an entity: its ID, name and components.

  entity find     Entities in every scene and prefab that match all the
                  filters given: a component type, part of a name or ID, a
                  script by name or file, a prefab they are instances of.

  entity show     One entity as the file has it, with its children.

  asset uses      Everything that names a file: component fields and
                  prefab instances, before the file is renamed or removed.

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
                  decay-language the language, weave every style property;
                  decay-guide and weave-guide how to write each, as markdown.

  mcp             Serve project_check, play_run, capture and capabilities
                  as MCP tools on standard input and output, for an agent
                  with no shell.
                  Read-only: the tools answer with the commands' own JSON,
                  and capture with the pictures themselves.

  mcp install     Register `sindri mcp` with an agent, by this binary's own
                  path, so it runs from anywhere. --client is claude (the
                  default; Claude Code), claude-desktop, cursor, vscode or
                  codex. --scope user (the default) serves every project;
                  --scope project writes the project's own config (.mcp.json,
                  .cursor/mcp.json, .vscode/mcp.json) in PATH or here.
                  --dry-run says what it would do. Run it again after the
                  binary moves.

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
  film:<seconds>x<count>  <count> frames across <seconds>, as one picture
                        (film-N.png, or film:<name>=1x6 as <name>.png)
  debug:<layers>        later pictures carry overlays: colliders (green, a
                        sensor yellow), ui (laid-out elements magenta, ones
                        that take presses cyan), all, or off
  probe:<x>,<y>         report what is at a pixel: the element, the one a
                        press reaches, the world point, colliders holding it
  expect:<what><op><value>  checked where it stands; a failed one fails the
                        run and is reported with what was there. <what> is a
                        board value, <entity>.x or .y, scene or printed; <op>
                        is = != < <= > >= (scene and printed: = and != only)
  for example: --steps "wait:0.5 hold:D wait:1 release:D expect:hero.x>3 shot:walked"

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
    client: Option<String>,
    scope: Option<String>,
    dry_run: bool,
    component: Option<String>,
    script: Option<String>,
    prefab: Option<String>,
    id: Option<String>,
    asset: Option<String>,
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
            client: None,
            scope: None,
            dry_run: false,
            component: None,
            script: None,
            prefab: None,
            id: None,
            asset: None,
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
                "--client" => options.client = Some(value("--client")?),
                "--scope" => options.scope = Some(value("--scope")?),
                "--dry-run" => options.dry_run = true,
                "--component" => options.component = Some(value("--component")?),
                "--script" => options.script = Some(value("--script")?),
                "--prefab" => options.prefab = Some(value("--prefab")?),
                "--id" => options.id = Some(value("--id")?),
                "--asset" => options.asset = Some(value("--asset")?),
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

/// The questions about a project: `None` when `words` asks something else.
fn query(words: &[&str], options: &Options) -> Option<Result<ExitCode, String>> {
    let answer = |target: &[&str]| -> Result<ExitCode, String> {
        let read = query::Project::read(&project(target)?);
        match words {
            ["project", "overview", ..] => {
                let report = query::overview(&read);
                options.report(&report, &report.human(), true)
            }
            ["scene", "outline", ..] => {
                let report = query::outline(&read, options.scene.as_deref())?;
                options.report(&report, &report.human(), true)
            }
            ["entity", "find", ..] => {
                let report = query::find(
                    &read,
                    &query::Filter {
                        component: options.component.as_deref(),
                        name: options.name.as_deref(),
                        script: options.script.as_deref(),
                        prefab: options.prefab.as_deref(),
                        scene: options.scene.as_deref(),
                    },
                );
                options.report(&report, &report.human(), true)
            }
            ["entity", "show", ..] => {
                let id = options.id.as_deref().ok_or("entity show needs --id ID")?;
                let report = query::show(&read, id, options.scene.as_deref())?;
                // The entity is JSON whatever the format: it is what the file says.
                println!("{}", json(&report)?);
                Ok(ExitCode::SUCCESS)
            }
            _ => {
                let asset = options
                    .asset
                    .as_deref()
                    .ok_or("asset uses needs --asset FILE")?;
                let report = query::uses(&read, asset);
                options.report(&report, &report.human(), true)
            }
        }
    };
    match words {
        ["project", "overview", target @ ..]
        | ["scene", "outline", target @ ..]
        | ["entity", "find" | "show", target @ ..]
        | ["asset", "uses", target @ ..] => Some(answer(target)),
        _ => None,
    }
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let options = Options::parse(args)?;
    let words: Vec<&str> = options.words.iter().map(String::as_str).collect();
    if let Some(answered) = query(&words, &options) {
        return answered;
    }
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
            // A reference is JSON whatever the format: a schema rendered for
            // a person is no longer a schema. A guide is read as written,
            // unless JSON is asked for.
            // Written rather than printed, so a reader that stops early,
            // such as `head`, ends the command instead of panicking it.
            let text = match reference
                .get("markdown")
                .and_then(serde_json::Value::as_str)
            {
                Some(markdown) if options.format == Format::Human => markdown.to_owned(),
                _ => json(&reference)? + "\n",
            };
            let _ = std::io::Write::write_all(&mut std::io::stdout(), text.as_bytes());
            Ok(ExitCode::SUCCESS)
        }
        ["mcp", "install", target @ ..] => {
            if target.len() > 1 {
                return Err("mcp install takes at most one folder".to_owned());
            }
            let client = options.client.as_deref().unwrap_or("claude");
            let scope = match &options.scope {
                Some(scope) => install::Scope::parse(scope)?,
                None => install::default_scope(client),
            };
            let exe = std::env::current_exe()
                .and_then(std::fs::canonicalize)
                .map_err(|error| format!("could not find this binary: {error}"))?;
            let project = std::fs::canonicalize(target.first().copied().unwrap_or("."))
                .map_err(|error| format!("could not find the folder: {error}"))?;
            let report = install::install(client, scope, &exe, &project, options.dry_run)?;
            options.report(&report, &report.human(), true)
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
