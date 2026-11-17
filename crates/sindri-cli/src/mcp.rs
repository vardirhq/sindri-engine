//! `sindri mcp`: the CLI's checks, served over MCP for an agent with no shell.
//!
//! An adapter, not an interface (`docs/agent-native-direction.md`): read-only,
//! it never writes a project file, and each tool answers with exactly the
//! JSON its command prints, from the same functions, so the two cannot drift.
//! An agent still changes a project by editing its files.
//!
//! The transport is MCP's stdio one: one JSON-RPC message a line on standard
//! input, one a line on standard output.

use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::{check, play};

/// The MCP revision this speaks.
const PROTOCOL: &str = "2025-06-18";

/// Serves until standard input closes.
pub fn serve() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = answer(&line) {
            let _ = writeln!(stdout, "{reply}");
            let _ = stdout.flush();
        }
    }
}

/// The reply to one message, or `None` for a notification.
#[must_use]
pub fn answer(line: &str) -> Option<Value> {
    let Ok(message) = serde_json::from_str::<Value>(line) else {
        return Some(error(&Value::Null, -32700, "parse error"));
    };
    let id = message.get("id").cloned()?;
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => json!({
            "protocolVersion": PROTOCOL,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "sindri", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Checks and plays a Sindri project; it never changes one. \
                             Change a project by editing its files (scenes and prefabs are JSON, \
                             behaviour is Decay, UI is Weave), then call project_check, and \
                             play_run to see what the change did.",
        }),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools() }),
        "tools/call" => call(&params),
        _ => return Some(error(&id, -32601, &format!("no method {method}"))),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn tools() -> Value {
    json!([
        {
            "name": "project_check",
            "description": "Check a Sindri project the way a run reads it: every scene, prefab, Decay script, Weave stylesheet and sindri.toml. Answers `sindri project check --format json`: every problem with a stable code, its file, and its line or entity. ok is false when there are errors.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": "The project folder, holding sindri.toml." }
                },
                "required": ["project"]
            }
        },
        {
            "name": "play_run",
            "description": "Play a Sindri project headless at 60 steps a second with scripted input. Answers `sindri play run --format json`: the shared board (Game state), where each tracked entity ended up, and what scripts printed and failed, by frame.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": "The project folder, holding sindri.toml." },
                    "scene": { "type": "string", "description": "A scene to start in, by file name, instead of the main one." },
                    "steps": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "In order: wait:<seconds>, key:<Key> (a tap), hold:<Key>, release:<Key>, set:<name>=<number>. Keys by name: D, Space, ArrowLeft, F."
                    },
                    "track": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Stable IDs of entities whose final position to report."
                    }
                },
                "required": ["project"]
            }
        }
    ])
}

/// A tool's answer: its command's JSON as text, and whether it went wrong.
fn call(params: &Value) -> Value {
    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
    let text = |key: &str| {
        arguments
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    let list = |key: &str| -> Vec<String> {
        arguments
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    };
    let Some(project) = text("project").map(PathBuf::from) else {
        return failed("give the project folder as `project`");
    };
    if !project.join("sindri.toml").is_file() {
        return failed(&format!("{} holds no sindri.toml", project.display()));
    }
    let answered = match params.get("name").and_then(Value::as_str) {
        Some("project_check") => {
            let report = check::check(&project);
            serde_json::to_value(&report).map(|value| (value, !report.ok))
        }
        Some("play_run") => {
            let scene = text("scene");
            let steps = list("steps");
            let track = list("track");
            match play::play(&play::Play {
                project: &project,
                scene: scene.as_deref(),
                steps: &steps,
                track: &track,
            }) {
                Ok(report) => serde_json::to_value(&report).map(|value| (value, !report.ok)),
                Err(message) => return failed(&message),
            }
        }
        other => return failed(&format!("no tool {other:?}")),
    };
    match answered {
        Ok((value, is_error)) => json!({
            "content": [{ "type": "text", "text": value.to_string() }],
            "structuredContent": value,
            "isError": is_error,
        }),
        Err(error) => failed(&error.to_string()),
    }
}

fn failed(message: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": message }], "isError": true })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(message: &Value) -> Value {
        answer(&message.to_string()).expect("a reply")
    }

    #[test]
    fn it_introduces_itself_and_lists_its_two_tools() {
        let hello = ask(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}));
        assert_eq!(hello["result"]["protocolVersion"], PROTOCOL);
        assert!(answer(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
        let listed = ask(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
        let names: Vec<_> = listed["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["project_check", "play_run"]);
    }

    #[test]
    fn a_tool_answers_with_its_command_s_json() {
        let project = concat!(env!("CARGO_MANIFEST_DIR"), "/../../games/flappy");
        let checked = ask(&json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {"name": "project_check", "arguments": {"project": project}}}));
        assert_eq!(checked["result"]["isError"], false);
        assert_eq!(checked["result"]["structuredContent"]["schema_version"], 1);
        assert_eq!(checked["result"]["structuredContent"]["ok"], true);
        let played = ask(&json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": {"name": "play_run", "arguments": {"project": project, "steps": ["wait:0.2"]}}}));
        assert_eq!(played["result"]["structuredContent"]["frames"], 12);
    }

    #[test]
    fn what_it_cannot_do_it_says() {
        let unknown = ask(&json!({"jsonrpc": "2.0", "id": 5, "method": "resources/write"}));
        assert_eq!(unknown["error"]["code"], -32601);
        let nowhere = ask(&json!({"jsonrpc": "2.0", "id": 6, "method": "tools/call",
            "params": {"name": "project_check", "arguments": {"project": "/"}}}));
        assert_eq!(nowhere["result"]["isError"], true);
        assert_eq!(answer("not json").unwrap()["error"]["code"], -32700);
    }
}
