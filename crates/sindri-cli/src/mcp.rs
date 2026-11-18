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

use crate::{check, play, query};

mod tools;

use tools::tools;

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
            "instructions": "Checks, plays, photographs and answers questions about a Sindri project; it never changes one. \
                             Change a project by editing its files (scenes and prefabs are JSON, \
                             behaviour is Decay, UI is Weave). In a project you have not seen, start \
                             with project_overview, and read a scene through scene_outline, \
                             find_entities and show_entity rather than whole. Before writing a file, \
                             read its shape from capabilities rather than guessing; before renaming \
                             or removing one, ask asset_uses. Check a Decay draft with check_script \
                             before writing it, and look names up with decay_api; for Weave, \
                             style_match a selector before using it and style_show an element that \
                             looks wrong. After a change, call project_check, then \
                             play_run with expect: steps for what the change should do, and capture or \
                             film: steps to look at it.",
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

/// The tools that need no project: the references and the host API.
fn projectless(params: &Value, text: &dyn Fn(&str) -> Option<String>) -> Option<Value> {
    if params.get("name").and_then(Value::as_str) == Some("decay_api") {
        let name = text("name").unwrap_or_default();
        let answer = query::script::api(&name);
        return Some(match serde_json::to_value(&answer) {
            Ok(value) => json!({
                "content": [{ "type": "text", "text": answer.human() }],
                "structuredContent": value,
                "isError": answer.found.is_empty(),
            }),
            Err(error) => failed(&error.to_string()),
        });
    }
    if params.get("name").and_then(Value::as_str) == Some("capabilities") {
        let name = text("name").unwrap_or_default();
        return Some(match sindri_capabilities::reference(&name) {
            Ok(Some(document)) => json!({
                // A guide is read as written; a reference is its JSON.
                "content": [{ "type": "text", "text": document
                    .get("markdown")
                    .and_then(Value::as_str)
                    .map_or_else(|| document.to_string(), str::to_owned) }],
                "structuredContent": document,
                "isError": false,
            }),
            Ok(None) => failed(&format!("no capability document {name:?}")),
            Err(error) => failed(&error.to_string()),
        });
    }
    None
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
    if let Some(answer) = projectless(params, &text) {
        return answer;
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
        Some(
            tool @ ("project_overview" | "scene_outline" | "find_entities" | "show_entity"
            | "asset_uses" | "style_show" | "style_match" | "style_unused" | "check_script"),
        ) => return query_call(tool, &project, &arguments),
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

/// A Weave question's answer, at the size a capture takes by default.
fn style_call(
    tool: &str,
    project: &std::path::Path,
    read: &query::Project,
    text: &dyn Fn(&str) -> Option<String>,
) -> Result<Value, String> {
    let viewport = weave::Viewport {
        width: 1280.0,
        height: 720.0,
    };
    let value = |result: Result<Value, serde_json::Error>| result.map_err(|e| e.to_string());
    match tool {
        "style_match" => {
            let selector = text("selector").ok_or("give the selector as `selector`")?;
            value(serde_json::to_value(query::style::matching(
                read, &selector, viewport,
            )?))
        }
        "style_unused" => {
            let sheets = query::style::stylesheets(project)?;
            value(serde_json::to_value(query::style::Unused::of(
                read, &sheets, viewport,
            )))
        }
        _ => {
            let id = text("id").ok_or("give the element's ID as `id`")?;
            let sheets = query::style::stylesheets(project)?;
            let scene = text("scene");
            value(serde_json::to_value(query::style::show(
                read,
                &sheets,
                &id,
                scene.as_deref(),
                viewport,
            )?))
        }
    }
}

/// A query tool's answer: its command's JSON, from the project as it reads.
fn query_call(tool: &str, project: &std::path::Path, arguments: &Value) -> Value {
    let text = |key: &str| {
        arguments
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
    };

    let read = query::Project::read(project);
    let scene = text("scene");
    let answered = match tool {
        "project_overview" => {
            serde_json::to_value(query::overview(&read)).map_err(|e| e.to_string())
        }
        "scene_outline" => query::outline(&read, scene.as_deref())
            .and_then(|outline| serde_json::to_value(outline).map_err(|e| e.to_string())),
        "find_entities" => {
            let (component, name, script, prefab) = (
                text("component"),
                text("name"),
                text("script"),
                text("prefab"),
            );
            serde_json::to_value(query::find(
                &read,
                &query::Filter {
                    component: component.as_deref(),
                    name: name.as_deref(),
                    script: script.as_deref(),
                    prefab: prefab.as_deref(),
                    scene: scene.as_deref(),
                },
            ))
            .map_err(|e| e.to_string())
        }
        "show_entity" => match text("id") {
            Some(id) => query::show(&read, &id, scene.as_deref())
                .and_then(|shown| serde_json::to_value(shown).map_err(|e| e.to_string())),
            None => Err("give the entity's ID as `id`".to_owned()),
        },
        "asset_uses" => match text("asset") {
            Some(asset) => {
                serde_json::to_value(query::uses(&read, &asset)).map_err(|e| e.to_string())
            }
            None => Err("give the file as `asset`".to_owned()),
        },
        "check_script" => match text("source") {
            Some(source) => {
                let file = text("file").unwrap_or_else(|| "draft.decay".to_owned());
                serde_json::to_value(query::script::check(&read, &file, &source))
                    .map_err(|e| e.to_string())
            }
            None => Err("give the script's text as `source`".to_owned()),
        },
        style => style_call(style, project, &read, &text),
    };
    match answered {
        Ok(value) => json!({
            "content": [{ "type": "text", "text": value.to_string() }],
            "structuredContent": value,
            "isError": false,
        }),
        Err(message) => failed(&message),
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
mod tests;
