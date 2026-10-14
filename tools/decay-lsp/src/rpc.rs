//! The server loop: JSON-RPC error replies, and which messages the LSP
//! lifecycle lets through.

use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

use crate::{
    Server,
    protocol::{Incoming, read_message, write_message},
};

pub(crate) const PARSE_ERROR: i64 = -32700;
pub(crate) const INVALID_REQUEST: i64 = -32600;
pub(crate) const METHOD_NOT_FOUND: i64 = -32601;
pub(crate) const SERVER_NOT_INITIALIZED: i64 = -32002;

pub(crate) fn error(
    output: &mut impl Write,
    id: &Value,
    code: i64,
    message: &str,
) -> io::Result<()> {
    write_message(
        output,
        &json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}}),
    )
}

/// What the server does with a message, given where it is in its lifecycle.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Gate {
    Handle,
    /// A request answered with this error instead of being handled.
    Reject(i64, &'static str),
    /// A notification, or a reply to a request the server never sends,
    /// dropped without an answer.
    Drop,
}

/// LSP 3.17: before `initialize`, requests fail with `ServerNotInitialized`
/// and notifications are dropped; after `shutdown`, requests are invalid and
/// notifications are dropped. `exit` is honoured at any point.
pub(crate) fn gate(message: &Value, initialized: bool, shut_down: bool) -> Gate {
    let id = message.get("id");
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        let is_reply =
            id.is_some() && (message.get("result").is_some() || message.get("error").is_some());
        return if is_reply {
            Gate::Drop
        } else {
            Gate::Reject(
                INVALID_REQUEST,
                "a message must be an object naming a method",
            )
        };
    };
    let is_request = id.is_some();
    match (method, is_request) {
        ("exit", _) => Gate::Handle,
        ("initialize", true) if initialized => {
            Gate::Reject(INVALID_REQUEST, "the server is already initialized")
        }
        (_, true) if shut_down => Gate::Reject(INVALID_REQUEST, "the server is shutting down"),
        (_, false) if shut_down => Gate::Drop,
        ("initialize", true) => Gate::Handle,
        (_, true) if !initialized => Gate::Reject(
            SERVER_NOT_INITIALIZED,
            "the server has not been initialized",
        ),
        (_, false) if !initialized => Gate::Drop,
        _ => Gate::Handle,
    }
}

/// Serves one client until `exit` or the end of input, and returns the
/// process exit code LSP 3.17 asks for: 0 if `shutdown` came first, else 1.
pub(crate) fn serve(input: &mut impl BufRead, output: &mut impl Write) -> io::Result<i32> {
    let mut server = Server::new();
    while let Some(incoming) = read_message(input)? {
        let message = match incoming {
            Incoming::Message(message) => message,
            Incoming::Unparsable(problem) => {
                eprintln!("decay-lsp: ignoring a message that is not JSON: {problem}");
                error(output, &Value::Null, PARSE_ERROR, &problem.to_string())?;
                continue;
            }
        };
        if !server.handle(&message, output)? {
            break;
        }
    }
    Ok(i32::from(!server.shutdown))
}
