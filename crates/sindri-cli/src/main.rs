//! `sindri`: a project checked, played and captured from a shell.
//!
//! The grammar is `sindri <noun> <verb> [target] [--flags]`, JSON is the
//! contract and the human form is rendered from it (`docs/cli-conventions.md`).
//! Commands read a project; they never change one. A coding agent changes a
//! project by editing its files, and uses this to see whether it worked
//! (`docs/agent-native-direction.md`).

mod check;
mod mcp;
mod play;
mod report;

use std::path::PathBuf;
use std::process::ExitCode;

const HELP: &str = r#"sindri: check a Sindri project from a shell.

USAGE
  sindri project check [PATH] [--format human|json]
  sindri mcp
  sindri play run [PATH] [--scene NAME] [--steps "STEP ..."] [--track ID]...
                  [--format human|json]

  PATH is the project folder, the one holding sindri.toml (default: here).

COMMANDS
  project check   Read every scene, prefab, Decay script and Weave stylesheet,
                  and sindri.toml, the way a run reads them, and report every
                  problem with a stable code. Exits 1 when there are errors;
                  warnings do not fail it.

  play run        Play the project headless at 60 steps a second, from its
                  main scene or --scene, doing --steps in order, and report
                  the shared board, where each --track entity ended up, what
                  scripts printed and what failed. Exits 1 when a script
                  failed. A step is one of:
                    wait:<seconds>       play on
                    key:<Key>            tap a key (key:Space, key:F)
                    hold:<Key>           press a key until its release
                    release:<Key>
                    set:<name>=<number>  write a board value (set:wood=4)
                  for example: --steps "wait:0.5 hold:D wait:1 release:D"

  mcp             Serve project check and play run as MCP tools on standard
                  input and output, for an agent with no shell. Read-only:
                  the tools answer with the commands' own JSON.

  help            This text.

--format json writes one JSON document with schema_version; it is the
contract. --format human, the default, is rendered from it."#;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Human,
    Json,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("sindri: {message}\n\n{HELP}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let mut format = Format::Human;
    let mut words = Vec::new();
    let mut scene = None;
    let mut steps = Vec::new();
    let mut track = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = |flag: &str| {
            rest.next()
                .cloned()
                .ok_or_else(|| format!("{flag} wants a value"))
        };
        match arg.as_str() {
            "--format" => {
                format = match value("--format")?.as_str() {
                    "json" => Format::Json,
                    "human" => Format::Human,
                    other => return Err(format!("--format takes human or json, not {other}")),
                };
            }
            "--scene" => scene = Some(value("--scene")?),
            "--steps" => steps.extend(value("--steps")?.split_whitespace().map(str::to_owned)),
            "--track" => track.push(value("--track")?),
            "-h" | "--help" => words.insert(0, "help".to_owned()),
            flag if flag.starts_with('-') => return Err(format!("unknown flag {flag}")),
            word => words.push(word.to_owned()),
        }
    }
    let project = |target: &[&str]| -> Result<PathBuf, String> {
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
    };
    let exit = |ok: bool| {
        if ok {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        }
    };
    match words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] | ["help", ..] => {
            println!("{HELP}");
            Ok(ExitCode::SUCCESS)
        }
        ["project", "check", target @ ..] => {
            let report = check::check(&project(target)?);
            match format {
                Format::Json => println!("{}", json(&report)?),
                Format::Human => print!("{}", report.human()),
            }
            Ok(exit(report.ok))
        }
        ["mcp"] => {
            mcp::serve();
            Ok(ExitCode::SUCCESS)
        }
        ["play", "run", target @ ..] => {
            let project = project(target)?;
            let report = play::play(&play::Play {
                project: &project,
                scene: scene.as_deref(),
                steps: &steps,
                track: &track,
            })?;
            match format {
                Format::Json => println!("{}", json(&report)?),
                Format::Human => print!("{}", report.human()),
            }
            Ok(exit(report.ok))
        }
        other => Err(format!("unknown command {}", other.join(" "))),
    }
}

fn json(value: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|error| error.to_string())
}
