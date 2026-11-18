//! `sindri mcp`: the CLI's checks, runs and pictures, served over MCP for an
//! agent with no shell.
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
            "instructions": "Checks, plays and photographs a Sindri project; it never changes one. \
                             Change a project by editing its files (scenes and prefabs are JSON, \
                             behaviour is Decay, UI is Weave), then call project_check, and \
                             play_run or capture to see what the change did. Before writing a \
                             file, read its shape from capabilities rather than guessing.",
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
                    "project": { "type": "string", "description": "The project folder, holding sindri.toml. The folder the server was started in unless given." }
                },
                "required": []
            }
        },
        {
            "name": "play_run",
            "description": "Play a Sindri project headless at 60 steps a second with scripted input, styled and laid out as it is drawn. Answers `sindri play run --format json`: the shared board (Game state), where each tracked entity ended up, and what scripts printed and failed, by frame; and each `shot` step's picture, inline.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": "The project folder, holding sindri.toml. The folder the server was started in unless given." },
                    "scene": { "type": "string", "description": "A scene to start in, by file name, instead of the main one." },
                    "steps": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": STEPS
                    },
                    "track": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Stable IDs of entities whose final position to report."
                    }
                },
                "required": []
            }
        },
        {
            "name": "capabilities",
            "description": "What this engine build can be told, as JSON, generated from the engine itself so it is never out of date. Read these instead of guessing: scene and prefab are the JSON Schemas for .scene and .prefab files; components lists every component with its fields, defaults and what each field means; decay is the script API with types; decay-language is the language; weave is every style property. Answers `sindri capabilities NAME`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "enum": sindri_capabilities::REFERENCES.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
                        "description": "Which document."
                    }
                },
                "required": ["name"]
            }
        },
        {
            "name": "capture",
            "description": "A picture of a Sindri project as a player would see it: opened at its main scene or `scene`, played through `steps`, then photographed. Answers the picture inline, and the run's report as `sindri render capture --format json` does.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": "The project folder, holding sindri.toml. The folder the server was started in unless given." },
                    "scene": { "type": "string", "description": "A scene to start in, by file name, instead of the main one." },
                    "steps": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": STEPS
                    },
                    "track": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Stable IDs of entities whose final position to report."
                    },
                    "width": { "type": "integer", "description": "Pixels across; 1280 unless given." },
                    "height": { "type": "integer", "description": "Pixels down; 720 unless given." }
                },
                "required": []
            }
        }
    ])
}

/// What a step may be, as the tools describe it.
const STEPS: &str = "In order: wait:<seconds>; key:<Key> (a tap: Space, F, ArrowLeft); hold:<Key> and release:<Key>; set:<name>=<number> (a shared board value); click:<entity> (a laid-out UI element, by ID); type:<text>; place:<entity>=<x>,<y>; near:<entity>=<name> (in front of the first thing of that name); shot or shot:<name> (a picture, answered inline); film:<seconds>x<count> (that many frames across that span, as one picture, answered inline: a jump's arc, a walk, a fade); expect:<what><op><value>, op one of = != < <= > >=, checked where it stands and failing the run when it does not hold: a board value (expect:score>=1), an entity's place by ID (expect:player.x>3), the scene playing (expect:scene=level2) or a printed line (expect:printed=Game over). Write the expectations a change should meet, then play_run them: ok is false and expectations says which failed and what was there instead.";

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
    if params.get("name").and_then(Value::as_str) == Some("capabilities") {
        let name = text("name").unwrap_or_default();
        return match sindri_capabilities::reference(&name) {
            Ok(Some(document)) => json!({
                "content": [{ "type": "text", "text": document.to_string() }],
                "structuredContent": document,
                "isError": false,
            }),
            Ok(None) => failed(&format!("no capability document {name:?}")),
            Err(error) => failed(&error.to_string()),
        };
    }
    // The folder the agent started the server in when none is named: an
    // agent opens a project and starts its servers there, so one
    // registration serves every project.
    let Some(project) = text("project")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
    else {
        return failed("give the project folder as `project`");
    };
    if !project.join("sindri.toml").is_file() {
        return failed(&format!(
            "{} holds no sindri.toml; name the project folder as `project`",
            project.display()
        ));
    }
    let answered = match params.get("name").and_then(Value::as_str) {
        Some("project_check") => {
            let report = check::check(&project);
            serde_json::to_value(&report).map(|value| (value, !report.ok))
        }
        Some(tool @ ("play_run" | "capture")) => {
            let scene = text("scene");
            let mut steps = list("steps");
            if tool == "capture" {
                steps.push("shot:capture".to_owned());
            }
            let track = list("track");
            let pixels = |key: &str, fallback: u32| {
                arguments
                    .get(key)
                    .and_then(Value::as_u64)
                    .and_then(|pixels| u32::try_from(pixels).ok())
                    .unwrap_or(fallback)
                    .clamp(16, 4096)
            };
            let shots = scratch();
            let played = play::play(&play::Play {
                project: &project,
                scene: scene.as_deref(),
                steps: &steps,
                track: &track,
                shots: &shots,
                size: [pixels("width", 1280), pixels("height", 720)],
            });
            let answer = match played {
                Ok(report) => pictured(&report),
                Err(message) => failed(&message),
            };
            let _ = std::fs::remove_dir_all(&shots);
            return answer;
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

/// A run's answer: its report as text and structured content, then each
/// picture it took, inline, so an agent sees it rather than a path.
fn pictured(report: &play::PlayReport) -> Value {
    let Ok(value) = serde_json::to_value(report) else {
        return failed("the report would not serialize");
    };
    let mut content = vec![json!({ "type": "text", "text": value.to_string() })];
    for shot in &report.shots {
        match std::fs::read(&shot.path) {
            Ok(png) => content.push(json!({
                "type": "image",
                "data": base64(&png),
                "mimeType": "image/png",
            })),
            Err(error) => content.push(json!({
                "type": "text",
                "text": format!("frame {}: the picture would not read: {error}", shot.frame),
            })),
        }
    }
    json!({ "content": content, "structuredContent": value, "isError": !report.ok })
}

/// A folder of its own for one call's pictures.
fn scratch() -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static CALLS: AtomicU32 = AtomicU32::new(0);
    std::env::temp_dir().join(format!(
        "sindri-mcp-{}-{}",
        std::process::id(),
        CALLS.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Standard base64, which MCP carries images in.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let triple = chunk.iter().enumerate().fold(0_u32, |all, (index, byte)| {
            all | u32::from(*byte) << (16 - 8 * index)
        });
        for place in 0..4 {
            if place <= chunk.len() {
                out.push(char::from(
                    ALPHABET[(triple >> (18 - 6 * place) & 63) as usize],
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
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
    fn it_introduces_itself_and_lists_its_tools() {
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
        assert_eq!(
            names,
            ["project_check", "play_run", "capabilities", "capture"]
        );
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
    fn it_hands_out_the_schemas_the_engine_writes() {
        let schema = ask(&json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call",
            "params": {"name": "capabilities", "arguments": {"name": "scene"}}}));
        assert_eq!(schema["result"]["isError"], false);
        assert_eq!(
            schema["result"]["structuredContent"]["title"],
            "Sindri scene"
        );
        let nothing = ask(&json!({"jsonrpc": "2.0", "id": 8, "method": "tools/call",
            "params": {"name": "capabilities", "arguments": {"name": "gossip"}}}));
        assert_eq!(nothing["result"]["isError"], true);
    }

    #[test]
    fn pictures_travel_as_base64() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xFF, 0xEE]), "/+4=");
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
