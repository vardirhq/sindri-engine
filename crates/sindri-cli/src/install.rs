//! `sindri mcp install`: registers `sindri mcp` with an agent, from anywhere.
//!
//! The server is registered by the absolute path of the binary running this
//! command, so neither the agent nor the person needs to be in the folder the
//! binary lives in. The tools default to the project the agent starts the
//! server in, so one registration serves every project.
//!
//! Each client keeps its servers somewhere of its own. Claude Code is asked
//! through its own `claude mcp add`, which knows where its scopes live; the
//! others are JSON or TOML files merged into, never written over: an existing
//! `sindri` entry is replaced and everything else is left as it was.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::report::SCHEMA_VERSION;

/// The name the server is registered under.
const NAME: &str = "sindri";

/// The agents this knows how to register with.
pub const CLIENTS: &str = "claude, claude-desktop, cursor, vscode, codex";

/// Where a registration goes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Every project, for this person.
    User,
    /// This project only, in a file that can be committed with it.
    Project,
}

impl Scope {
    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "user" => Ok(Self::User),
            "project" => Ok(Self::Project),
            other => Err(format!("--scope is user or project, not {other}")),
        }
    }
}

/// What `mcp install` did, or with `--dry-run` would do.
#[derive(Serialize)]
pub struct InstallReport {
    pub schema_version: u32,
    pub client: String,
    pub scope: &'static str,
    /// The command the agent will run.
    pub command: Vec<String>,
    /// The file written, or the command run, to register it.
    pub registered_in: String,
    pub dry_run: bool,
    /// Worth knowing: a binary inside a build folder, say.
    pub notes: Vec<String>,
}

impl InstallReport {
    #[must_use]
    pub fn human(&self) -> String {
        let verb = if self.dry_run {
            "Would register"
        } else {
            "Registered"
        };
        let mut text = format!(
            "{verb} sindri with {} ({} scope), in {}:\n  {}\n",
            self.client,
            self.scope,
            self.registered_in,
            self.command.join(" ")
        );
        for note in &self.notes {
            text.push_str("\nNote: ");
            text.push_str(note);
            text.push('\n');
        }
        if !self.dry_run {
            text.push_str("\nRestart the agent, or start a new session, to pick it up.\n");
        }
        text
    }
}

/// Registers `exe mcp` with `client`.
///
/// # Errors
/// An unknown client, a config file that is not valid to merge into, a
/// missing `claude` command, or a write that failed.
pub fn install(
    client: &str,
    scope: Scope,
    exe: &Path,
    project: &Path,
    dry_run: bool,
) -> Result<InstallReport, String> {
    let exe_text = exe.display().to_string();
    let command = vec![exe_text.clone(), "mcp".to_owned()];
    let home = home();
    let registered_in = match (client, scope) {
        ("claude", _) => {
            let scope_word = if scope == Scope::User {
                "user"
            } else {
                "project"
            };
            let args = [
                "mcp", "add", "--scope", scope_word, NAME, "--", &exe_text, "mcp",
            ];
            let shown = format!("claude {}", args.join(" "));
            if !dry_run {
                claude(&args, scope_word, project)?;
            }
            shown
        }
        ("claude-desktop", Scope::User) => {
            let file = desktop_config(home.as_deref())?;
            merge_json(&file, "mcpServers", stdio(&command, false), dry_run)?;
            file.display().to_string()
        }
        ("claude-desktop", Scope::Project) => {
            return Err("Claude Desktop has no per-project servers; use --scope user".to_owned());
        }
        ("cursor", scope) => {
            let file = match scope {
                Scope::User => home.ok_or("no home folder to find ~/.cursor in")?,
                Scope::Project => project.to_path_buf(),
            }
            .join(".cursor/mcp.json");
            merge_json(&file, "mcpServers", stdio(&command, false), dry_run)?;
            file.display().to_string()
        }
        ("vscode", Scope::Project) => {
            let file = project.join(".vscode/mcp.json");
            merge_json(&file, "servers", stdio(&command, true), dry_run)?;
            file.display().to_string()
        }
        ("vscode", Scope::User) => {
            return Err(
                "VS Code keeps user servers in its own profile; use --scope project, which writes .vscode/mcp.json"
                    .to_owned(),
            );
        }
        ("codex", Scope::User) => {
            let file = home
                .ok_or("no home folder to find ~/.codex in")?
                .join(".codex/config.toml");
            merge_toml(&file, &command, dry_run)?;
            file.display().to_string()
        }
        ("codex", Scope::Project) => {
            return Err(
                "Codex reads servers from ~/.codex/config.toml; use --scope user".to_owned(),
            );
        }
        (other, _) => return Err(format!("no client {other}; one of {CLIENTS}")),
    };
    Ok(InstallReport {
        schema_version: SCHEMA_VERSION,
        client: client.to_owned(),
        scope: if scope == Scope::User {
            "user"
        } else {
            "project"
        },
        command,
        registered_in,
        dry_run,
        notes: notes(exe),
    })
}

/// The scope a client is registered in when none is asked for: where one
/// registration serves every project, if the client has such a place.
#[must_use]
pub fn default_scope(client: &str) -> Scope {
    if client == "vscode" {
        Scope::Project
    } else {
        Scope::User
    }
}

/// Asks Claude Code to register the server, replacing an earlier one.
fn claude(args: &[&str], scope: &str, project: &Path) -> Result<(), String> {
    let run = |args: &[&str]| {
        Command::new("claude")
            .args(args)
            .current_dir(project)
            .output()
            .map_err(|error| {
                format!(
                    "could not run `claude` ({error}); is Claude Code installed? Run this instead:\n  claude {}",
                    args.join(" ")
                )
            })
    };
    // Removed first, because `add` refuses a name that is already taken: an
    // install after the binary moved should move the registration with it.
    let _ = run(&["mcp", "remove", "--scope", scope, NAME]);
    let added = run(args)?;
    if added.status.success() {
        Ok(())
    } else {
        Err(format!(
            "claude mcp add failed: {}",
            String::from_utf8_lossy(&added.stderr).trim()
        ))
    }
}

/// A stdio server entry, as the JSON-configured clients write one.
fn stdio(command: &[String], typed: bool) -> Value {
    let mut entry = json!({ "command": command[0], "args": command[1..] });
    if typed {
        entry["type"] = json!("stdio");
    }
    entry
}

/// Sets `sindri` under `section` in a JSON config, keeping the rest.
fn merge_json(file: &Path, section: &str, entry: Value, dry_run: bool) -> Result<(), String> {
    let mut config = match std::fs::read_to_string(file) {
        Ok(text) if !text.trim().is_empty() => serde_json::from_str::<Value>(&text).map_err(
            |error| {
                format!(
                    "{} is not JSON this can merge into ({error}); fix it or add the server by hand",
                    file.display()
                )
            },
        )?,
        _ => Value::Object(Map::new()),
    };
    let root = config
        .as_object_mut()
        .ok_or_else(|| format!("{} does not hold a JSON object", file.display()))?;
    let servers = root
        .entry(section)
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| format!("{section} in {} is not an object", file.display()))?;
    servers.insert(NAME.to_owned(), entry);
    if dry_run {
        return Ok(());
    }
    write(
        file,
        &(serde_json::to_string_pretty(&config).map_err(|e| e.to_string())? + "\n"),
    )
}

/// Sets `[mcp_servers.sindri]` in a TOML config, keeping every other line.
fn merge_toml(file: &Path, command: &[String], dry_run: bool) -> Result<(), String> {
    let existing = std::fs::read_to_string(file).unwrap_or_default();
    let table = format!(
        "[mcp_servers.{NAME}]\ncommand = {}\nargs = {}\n",
        toml_string(&command[0]),
        format_args!(
            "[{}]",
            command[1..]
                .iter()
                .map(|arg| toml_string(arg))
                .collect::<Vec<_>>()
                .join(", ")
        )
    );
    let text = replace_table(&existing, &format!("[mcp_servers.{NAME}]"), &table);
    if dry_run {
        return Ok(());
    }
    write(file, &text)
}

/// `text` with the table headed `header` swapped for `table`, or `table`
/// added at the end when there is none.
fn replace_table(text: &str, header: &str, table: &str) -> String {
    let mut kept = Vec::new();
    let mut inside = false;
    let mut placed = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == header
                || trimmed.starts_with(&format!("{}.", &header[..header.len() - 1]));
            if trimmed == header && !placed {
                kept.push(table.trim_end().to_owned());
                placed = true;
            }
        }
        if !inside {
            kept.push(line.to_owned());
        }
    }
    let mut joined = kept.join("\n");
    if !placed {
        if joined.trim().is_empty() {
            joined.clear();
        } else {
            joined.push_str("\n\n");
        }
        joined.push_str(table.trim_end());
    }
    joined.push('\n');
    joined
}

/// A TOML basic string; JSON's escapes are TOML's.
fn toml_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| format!("\"{text}\""))
}

fn write(file: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{} could not be made: {error}", parent.display()))?;
    }
    std::fs::write(file, text)
        .map_err(|error| format!("{} could not be written: {error}", file.display()))
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Where Claude Desktop keeps its servers on this system.
fn desktop_config(home: Option<&Path>) -> Result<PathBuf, String> {
    let file = "Claude/claude_desktop_config.json";
    if cfg!(target_os = "windows") {
        return std::env::var_os("APPDATA")
            .map(|appdata| PathBuf::from(appdata).join(file))
            .ok_or_else(|| "no APPDATA folder to find Claude Desktop's settings in".to_owned());
    }
    let home = home.ok_or("no home folder to find Claude Desktop's settings in")?;
    Ok(if cfg!(target_os = "macos") {
        home.join("Library/Application Support").join(file)
    } else {
        home.join(".config").join(file)
    })
}

/// What a person should know about the binary being registered.
fn notes(exe: &Path) -> Vec<String> {
    let in_build = exe.components().any(|part| part.as_os_str() == "target");
    if in_build {
        vec![format!(
            "{} is a build in a target folder: a rebuild updates it, but `cargo clean` removes it. For a copy that stays, run `cargo install --path crates/sindri-cli` and then `sindri mcp install` again.",
            exe.display()
        )]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_json_config_keeps_what_it_had() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mcp.json");
        std::fs::write(
            &file,
            r#"{"theme": "dark", "mcpServers": {"other": {"command": "x"}, "sindri": {"command": "old"}}}"#,
        )
        .unwrap();
        let command = vec!["/opt/sindri".to_owned(), "mcp".to_owned()];
        merge_json(&file, "mcpServers", stdio(&command, false), false).unwrap();
        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(written["theme"], "dark");
        assert_eq!(written["mcpServers"]["other"]["command"], "x");
        assert_eq!(
            written["mcpServers"]["sindri"],
            json!({"command": "/opt/sindri", "args": ["mcp"]})
        );

        std::fs::write(&file, "{ not json").unwrap();
        assert!(merge_json(&file, "mcpServers", stdio(&command, false), false).is_err());
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "{ not json",
            "left alone"
        );
    }

    #[test]
    fn a_toml_config_swaps_only_its_own_table() {
        let before = "model = \"o3\"\n\n[mcp_servers.sindri]\ncommand = \"/old\"\nargs = [\"mcp\"]\n\n[mcp_servers.sindri.env]\nA = \"1\"\n\n[mcp_servers.other]\ncommand = \"x\"\n";
        let after = replace_table(
            before,
            "[mcp_servers.sindri]",
            "[mcp_servers.sindri]\ncommand = \"/new\"\n",
        );
        assert!(after.contains("model = \"o3\""));
        assert!(after.contains("command = \"/new\""));
        assert!(!after.contains("/old") && !after.contains("A = \"1\""));
        assert!(after.contains("[mcp_servers.other]\ncommand = \"x\""));
        let fresh = replace_table(
            "",
            "[mcp_servers.sindri]",
            "[mcp_servers.sindri]\ncommand = \"/new\"\n",
        );
        assert_eq!(fresh, "[mcp_servers.sindri]\ncommand = \"/new\"\n");
        assert_eq!(
            toml_string("C:\\bin\\sindri.exe"),
            "\"C:\\\\bin\\\\sindri.exe\""
        );
    }
}
