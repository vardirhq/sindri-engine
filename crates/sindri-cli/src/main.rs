//! `sindri`: a project checked, played and captured from a shell.
//!
//! The grammar is `sindri <noun> <verb> [target] [--flags]`, JSON is the
//! contract and the human form is rendered from it (`docs/cli-conventions.md`).
//! Commands read a project; they never change one. A coding agent changes a
//! project by editing its files, and uses this to see whether it worked
//! (`docs/agent-native-direction.md`).

mod check;
mod report;

use std::path::PathBuf;
use std::process::ExitCode;

const HELP: &str = "\
sindri: check a Sindri project from a shell.

USAGE
  sindri project check [PATH] [--format human|json]

  PATH is the project folder, the one holding sindri.toml (default: here).

COMMANDS
  project check   Read every scene, prefab, Decay script and Weave stylesheet,
                  and sindri.toml, the way a run reads them, and report every
                  problem with a stable code. Exits 1 when there are errors;
                  warnings do not fail it.

  help            This text.

--format json writes one JSON document with schema_version; it is the
contract. --format human, the default, is rendered from it.";

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
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--format" => {
                format = match rest.next().map(String::as_str) {
                    Some("json") => Format::Json,
                    Some("human") => Format::Human,
                    other => return Err(format!("--format takes human or json, not {other:?}")),
                };
            }
            "-h" | "--help" => words.insert(0, "help".to_owned()),
            flag if flag.starts_with('-') => return Err(format!("unknown flag {flag}")),
            word => words.push(word.to_owned()),
        }
    }
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
            let project = match target {
                [] => PathBuf::from("."),
                [path] => PathBuf::from(path),
                _ => return Err("project check takes one project folder".to_owned()),
            };
            if !project.join("sindri.toml").is_file() {
                return Err(format!(
                    "{} holds no sindri.toml; give the project folder",
                    project.display()
                ));
            }
            let report = check::check(&project);
            match format {
                Format::Json => println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
                ),
                Format::Human => print!("{}", report.human()),
            }
            Ok(if report.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        other => Err(format!("unknown command {}", other.join(" "))),
    }
}
