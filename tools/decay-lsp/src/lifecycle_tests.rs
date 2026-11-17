//! Drives the whole server through its framed stdin/stdout loop, as a client
//! would, and reads back every message it writes.

use serde_json::{Value, json};

use crate::protocol::{Incoming, read_message};
use crate::rpc::{self, serve};

fn frame(message: &Value) -> Vec<u8> {
    let body = message.to_string();
    format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
}

fn frame_raw(body: &str) -> Vec<u8> {
    format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
}

fn request(id: i64, method: &str, params: &Value) -> Vec<u8> {
    frame(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
}

fn notification(method: &str, params: &Value) -> Vec<u8> {
    frame(&json!({"jsonrpc":"2.0","method":method,"params":params}))
}

fn initialize() -> Vec<u8> {
    let mut stream = request(1, "initialize", &json!({"capabilities":{}}));
    stream.extend(notification("initialized", &json!({})));
    stream
}

/// Runs the server over `stream` and returns its exit code and output.
fn run(stream: &[Vec<u8>]) -> (i32, Vec<Value>) {
    let input = stream.concat();
    let mut output = Vec::new();
    let code = serve(&mut input.as_slice(), &mut output).expect("server runs");
    let mut written = output.as_slice();
    let mut messages = Vec::new();
    while let Some(incoming) = read_message(&mut written).expect("server output is framed") {
        let Incoming::Message(message) = incoming else {
            panic!("server wrote a body that is not JSON");
        };
        messages.push(message);
    }
    (code, messages)
}

fn reply(messages: &[Value], id: i64) -> &Value {
    messages
        .iter()
        .find(|message| message["id"] == id)
        .unwrap_or_else(|| panic!("no reply to request {id} in {messages:#?}"))
}

fn error_code(messages: &[Value], id: i64) -> Option<i64> {
    reply(messages, id).pointer("/error/code")?.as_i64()
}

#[test]
fn shutdown_then_exit_exits_cleanly() {
    let (code, messages) = run(&[
        initialize(),
        request(2, "shutdown", &Value::Null),
        notification("exit", &Value::Null),
    ]);
    assert_eq!(code, 0);
    assert_eq!(reply(&messages, 2)["result"], Value::Null);
    assert!(reply(&messages, 2).get("error").is_none());
}

#[test]
fn exit_without_shutdown_exits_with_an_error_code() {
    let (code, _) = run(&[initialize(), notification("exit", &Value::Null)]);
    assert_eq!(code, 1);
}

#[test]
fn input_closing_without_shutdown_is_an_unclean_exit() {
    assert_eq!(run(&[initialize()]).0, 1);
    let (code, _) = run(&[initialize(), request(2, "shutdown", &Value::Null)]);
    assert_eq!(code, 0);
}

#[test]
fn exit_stops_reading_further_messages() {
    let (_, messages) = run(&[
        initialize(),
        notification("exit", &Value::Null),
        request(2, "textDocument/hover", &json!({})),
    ]);
    assert!(messages.iter().all(|message| message["id"] != 2));
}

#[test]
fn initialize_advertises_the_implemented_features() {
    let (_, messages) = run(&[initialize()]);
    let capabilities = &reply(&messages, 1)["result"]["capabilities"];
    assert_eq!(capabilities["textDocumentSync"]["openClose"], true);
    assert_eq!(capabilities["hoverProvider"], true);
    assert_eq!(capabilities["documentSymbolProvider"], true);
    assert!(capabilities["completionProvider"].is_object());
}

#[test]
fn requests_before_initialize_are_refused_and_notifications_dropped() {
    let (_, messages) = run(&[
        request(5, "textDocument/hover", &json!({})),
        notification(
            "textDocument/didOpen",
            &json!({"textDocument":{"uri":"file:///a.decay","text":"fn (","version":1}}),
        ),
    ]);
    assert_eq!(error_code(&messages, 5), Some(rpc::SERVER_NOT_INITIALIZED));
    assert_eq!(messages.len(), 1, "a dropped didOpen publishes nothing");
}

#[test]
fn a_second_initialize_is_invalid() {
    let (_, messages) = run(&[initialize(), request(2, "initialize", &json!({}))]);
    assert_eq!(error_code(&messages, 2), Some(rpc::INVALID_REQUEST));
}

#[test]
fn requests_after_shutdown_are_invalid() {
    let (_, messages) = run(&[
        initialize(),
        request(2, "shutdown", &Value::Null),
        request(3, "textDocument/hover", &json!({})),
        notification(
            "textDocument/didOpen",
            &json!({"textDocument":{"uri":"file:///a.decay","text":"fn (","version":1}}),
        ),
    ]);
    assert_eq!(error_code(&messages, 3), Some(rpc::INVALID_REQUEST));
    assert!(
        messages
            .iter()
            .all(|message| message["method"] != "textDocument/publishDiagnostics")
    );
}

#[test]
fn unknown_requests_are_method_not_found_and_unknown_notifications_ignored() {
    let (code, messages) = run(&[
        initialize(),
        request(2, "textDocument/rename", &json!({})),
        notification("$/setTrace", &json!({"value":"off"})),
        notification("workspace/didChangeConfiguration", &json!({})),
        request(3, "shutdown", &Value::Null),
        notification("exit", &Value::Null),
    ]);
    assert_eq!(error_code(&messages, 2), Some(rpc::METHOD_NOT_FOUND));
    assert_eq!(code, 0);
    // initialize, the rename error and the shutdown reply: nothing for the
    // notifications.
    assert_eq!(messages.len(), 3, "{messages:#?}");
}

#[test]
fn malformed_json_is_a_parse_error_and_the_server_carries_on() {
    let (code, messages) = run(&[
        initialize(),
        frame_raw("{\"id\": 2, \"method\""),
        request(3, "shutdown", &Value::Null),
        notification("exit", &Value::Null),
    ]);
    let parse_error = messages
        .iter()
        .find(|message| message.pointer("/error/code") == Some(&json!(rpc::PARSE_ERROR)))
        .expect("parse error reply");
    assert_eq!(parse_error["id"], Value::Null);
    assert_eq!(reply(&messages, 3)["result"], Value::Null);
    assert_eq!(code, 0);
}

#[test]
fn messages_that_are_not_requests_are_invalid_and_client_replies_dropped() {
    let (_, messages) = run(&[
        initialize(),
        frame(&json!([1, 2])),
        frame(&json!({"jsonrpc":"2.0","id":9})),
        frame(&json!({"jsonrpc":"2.0","id":10,"result":null})),
    ]);
    let invalid = messages
        .iter()
        .filter(|message| message.pointer("/error/code") == Some(&json!(rpc::INVALID_REQUEST)))
        .collect::<Vec<_>>();
    assert_eq!(invalid.len(), 2, "{messages:#?}");
    assert_eq!(error_code(&messages, 9), Some(rpc::INVALID_REQUEST));
    assert!(messages.iter().all(|message| message["id"] != 10));
}

#[test]
fn a_document_publishes_diagnostics_and_clears_them_on_close() {
    let uri = "file:///broken.decay";
    let (_, messages) = run(&[
        initialize(),
        notification(
            "textDocument/didOpen",
            &json!({"textDocument":{"uri":uri,"text":"script Broken { fn update( }","version":1}}),
        ),
        notification(
            "textDocument/didClose",
            &json!({"textDocument":{"uri":uri}}),
        ),
    ]);
    let published = messages
        .iter()
        .filter(|message| message["method"] == "textDocument/publishDiagnostics")
        .collect::<Vec<_>>();
    assert_eq!(published.len(), 2);
    assert!(
        !published[0]["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(published[1]["params"]["diagnostics"], json!([]));
}

#[test]
fn completion_and_hover_offer_every_reserved_keyword() {
    let uri = "file:///keywords.decay";
    let source = "script Words { fn update(dt: f32) { m } }";
    let character = source.find(" m ").unwrap() + 2;
    let position = json!({"textDocument":{"uri":uri},"position":{"line":0,"character":character}});
    let (_, messages) = run(&[
        initialize(),
        notification(
            "textDocument/didOpen",
            &json!({"textDocument":{"uri":uri,"text":source,"version":1}}),
        ),
        request(2, "textDocument/completion", &position),
    ]);
    let labels = reply(&messages, 2)["result"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["detail"] == "Decay keyword")
        .map(|item| item["label"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    let reserved = decay_syntax::vocabulary::KEYWORDS
        .iter()
        .map(|(word, _)| (*word).to_owned())
        .collect::<Vec<_>>();
    assert_eq!(labels, reserved);

    let source = "script Words { fn update(dt: f32) { while true { break; } } }";
    let character = source.find("while").unwrap() + 1;
    let (_, messages) = run(&[
        initialize(),
        notification(
            "textDocument/didOpen",
            &json!({"textDocument":{"uri":uri,"text":source,"version":1}}),
        ),
        request(
            2,
            "textDocument/hover",
            &json!({"textDocument":{"uri":uri},"position":{"line":0,"character":character}}),
        ),
    ]);
    assert_eq!(
        reply(&messages, 2)["result"]["contents"]["value"],
        "`while` · Decay keyword"
    );
}

/// What completing `prefix` at the end of a line of `update` offers for
/// `label`: its insert text.
fn completes(prefix: &str, label: &str) -> Option<String> {
    let uri = "file:///calls.decay";
    let source = format!(
        "script Calls {{\n    fn helper() {{ }}\n    fn heal(amount: number) {{ }}\n    fn mix(a: number, b: bool, c: String) {{ }}\n    fn update(dt: number) {{\n        {prefix}\n    }}\n}}"
    );
    let character = 8 + prefix.len();
    let (_, messages) = run(&[
        initialize(),
        notification(
            "textDocument/didOpen",
            &json!({"textDocument":{"uri":uri,"text":source,"version":1}}),
        ),
        request(
            2,
            "textDocument/completion",
            &json!({"textDocument":{"uri":uri},"position":{"line":5,"character":character}}),
        ),
    ]);
    reply(&messages, 2)["result"]
        .as_array()?
        .iter()
        .find(|item| item["label"] == label)
        .map(|item| {
            item["insertText"]
                .as_str()
                .unwrap_or_else(|| item["label"].as_str().unwrap())
                .to_owned()
        })
}

#[test]
fn a_completed_call_has_a_placeholder_for_each_argument() {
    // The host's own functions: none, one and two arguments.
    assert_eq!(completes("Scene.", "current").as_deref(), Some("current()"));
    assert_eq!(completes("World.", "find").as_deref(), Some("find($1)"));
    assert_eq!(
        completes("Animation.", "fired").as_deref(),
        Some("fired($1, $2)")
    );
    // The script's own functions, completed by name where they are called.
    assert_eq!(completes("hel", "helper").as_deref(), Some("helper()"));
    assert_eq!(completes("hea", "heal").as_deref(), Some("heal($1)"));
    assert_eq!(completes("mi", "mix").as_deref(), Some("mix($1, $2, $3)"));
}

/// Positions are UTF-16 code units, as LSP's default encoding has them: a
/// rocket before the cursor is two of them, though four bytes in the source.
#[test]
fn positions_after_an_emoji_are_counted_in_utf16() {
    let uri = "file:///rocket.decay";
    let line = "        let label = \"🚀\"; let bad: bool = 1;";
    let source = format!("script Rocket {{\n    fn update(dt: number) {{\n{line}\n    }}\n}}");
    let (_, messages) = run(&[
        initialize(),
        notification(
            "textDocument/didOpen",
            &json!({"textDocument":{"uri":uri,"text":source,"version":1}}),
        ),
    ]);
    let diagnostics = messages
        .iter()
        .find(|message| message["method"] == "textDocument/publishDiagnostics")
        .expect("diagnostics are published")["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .clone();
    let mismatch = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["range"]["start"]["line"] == 2)
        .expect("the bad binding is reported on its line");
    let column = mismatch["range"]["start"]["character"].as_u64().unwrap();
    let before = &line[..line.find("let bad").unwrap()];
    let utf16 = before.encode_utf16().count() as u64;
    assert_eq!(column, utf16, "{mismatch}");
    assert_ne!(column, before.len() as u64, "not bytes");

    // And completion after one, on the same line, still finds the call.
    let completing = "        let label = \"🚀\"; Scene.";
    let source = format!("script Rocket {{\n    fn update(dt: number) {{\n{completing}\n    }}\n}}");
    let character = completing.encode_utf16().count();
    let (_, messages) = run(&[
        initialize(),
        notification(
            "textDocument/didOpen",
            &json!({"textDocument":{"uri":uri,"text":source,"version":1}}),
        ),
        request(
            2,
            "textDocument/completion",
            &json!({"textDocument":{"uri":uri},"position":{"line":2,"character":character}}),
        ),
    ]);
    let labels = reply(&messages, 2)["result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label == "current"), "{labels:?}");
}
