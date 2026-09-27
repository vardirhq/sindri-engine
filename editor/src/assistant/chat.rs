//! Asking the local model something and getting its answer back as text.
//!
//! [`Model`] is the whole boundary: a conversation goes in, a reply comes out.
//! Everything that decides what to ask and whether the answer is any good lives
//! above it, which is what lets the repair loop and the verification cases run
//! in tests against a scripted model with no runner, no GPU and no network.
//!
//! The transport is written against the standard library: it only ever talks to
//! the runner on a loopback address, so it needs no TLS, and the editor stays
//! free of an HTTP stack. `server` owns the runner this talks to.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use serde_json::{Value, json};

/// Who said a line of the conversation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    /// Sindri's standing rules for the task.
    System,
    /// What is being asked, and the material it is about.
    User,
    /// What the model said earlier in the same task.
    Assistant,
}

impl Role {
    const fn wire(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
}

/// Why no reply came back. Said specifically, because each has a different
/// next step for the person reading it.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ModelError {
    #[error("the model runner is not answering")]
    Unreachable,
    #[error("the model took longer than {0} seconds")]
    TimedOut(u64),
    #[error("the model runner refused the request: {0}")]
    Refused(String),
    #[error("the model runner's answer could not be read: {0}")]
    Unreadable(String),
}

/// Anything that can answer a conversation.
pub trait Model {
    fn reply(&mut self, conversation: &[Message]) -> Result<String, ModelError>;
}

/// How long a single answer may take.
///
/// Generous, because a local model rewriting a two-hundred-line script on a
/// modest GPU is honestly a minute or more; short enough that a runner that has
/// wedged is reported rather than waited on forever.
const ANSWER_TIMEOUT: Duration = Duration::from_mins(5);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(600);

/// A conversation as the OpenAI-style chat API every local runner speaks.
pub fn request(model: &str, conversation: &[Message]) -> Value {
    json!({
        "model": model,
        "messages": conversation
            .iter()
            .map(|message| json!({ "role": message.role.wire(), "content": message.content }))
            .collect::<Vec<_>>(),
        "stream": false,
        // Low, because the task is to produce text a compiler will accept,
        // not to be inventive about it.
        "temperature": 0.2,
    })
}

/// The reply text out of a `/v1/chat/completions` answer.
pub fn content(body: &str) -> Result<String, ModelError> {
    let value: Value =
        serde_json::from_str(body).map_err(|error| ModelError::Unreadable(error.to_string()))?;
    value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ModelError::Unreadable("no message content".to_owned()))
}

/// What a refused request said about itself, or its status.
pub fn refusal(status: u16, body: &str) -> ModelError {
    let reason = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            let error = value.get("error")?;
            error
                .get("message")
                .or(Some(error))?
                .as_str()
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("HTTP {status}"));
    ModelError::Refused(reason)
}

/// Sends one request and returns the raw response.
pub fn post(endpoint: &str, path: &str, body: &str) -> Result<Vec<u8>, ModelError> {
    let address: SocketAddr = endpoint.parse().map_err(|_| ModelError::Unreachable)?;
    let mut stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT)
        .map_err(|_| ModelError::Unreachable)?;
    stream
        .set_read_timeout(Some(ANSWER_TIMEOUT))
        .and_then(|()| stream.set_write_timeout(Some(ANSWER_TIMEOUT)))
        .map_err(|_| ModelError::Unreachable)?;
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .map_err(|_| ModelError::Unreachable)?;
    let mut raw = Vec::new();
    match stream.read_to_end(&mut raw) {
        Ok(_) => Ok(raw),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            Err(ModelError::TimedOut(ANSWER_TIMEOUT.as_secs()))
        }
        Err(error) => Err(ModelError::Unreadable(error.to_string())),
    }
}

/// One GET, returning its status, with a short wait: used to ask a runner
/// whether it is ready, which it answers at once or not at all.
pub fn get_status(endpoint: &str, path: &str) -> Option<u16> {
    let address: SocketAddr = endpoint.parse().ok()?;
    let mut stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut raw = Vec::new();
    let _ = stream.read_to_end(&mut raw);
    parse_response(&raw).ok().map(|(status, _)| status)
}

/// Splits a raw HTTP/1.1 response into its status and body.
///
/// Handles both ways a runner may frame a body it is not streaming: a declared
/// length, or chunks. Anything else is read to the end of the connection, which
/// the request asked to be closed.
pub fn parse_response(raw: &[u8]) -> Result<(u16, String), ModelError> {
    let unreadable = |why: &str| ModelError::Unreadable(why.to_owned());
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| unreadable("no end of headers"))?;
    let head = std::str::from_utf8(&raw[..split]).map_err(|_| unreadable("headers"))?;
    let rest = &raw[split + 4..];
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| unreadable("no status"))?;
    let mut chunked = false;
    let mut length = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let (name, value) = (name.trim().to_ascii_lowercase(), value.trim());
        if name == "transfer-encoding" && value.eq_ignore_ascii_case("chunked") {
            chunked = true;
        } else if name == "content-length" {
            length = value.parse::<usize>().ok();
        }
    }
    let body = if chunked {
        dechunk(rest).ok_or_else(|| unreadable("malformed chunks"))?
    } else if let Some(length) = length {
        rest.get(..length)
            .ok_or_else(|| unreadable("body shorter than declared"))?
            .to_vec()
    } else {
        rest.to_vec()
    };
    String::from_utf8(body)
        .map(|body| (status, body))
        .map_err(|_| unreadable("body is not text"))
}

fn dechunk(mut rest: &[u8]) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let end = rest.windows(2).position(|window| window == b"\r\n")?;
        let size_line = std::str::from_utf8(&rest[..end]).ok()?;
        let size = usize::from_str_radix(size_line.split(';').next()?.trim(), 16).ok()?;
        rest = &rest[end + 2..];
        if size == 0 {
            return Some(body);
        }
        body.extend_from_slice(rest.get(..size)?);
        rest = rest.get(size + 2..)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sized_body_is_read_to_its_length() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhelloTRAILING";
        assert_eq!(parse_response(raw), Ok((200, "hello".to_owned())));
    }

    #[test]
    fn a_chunked_body_is_joined() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nhell\r\n1\r\no\r\n0\r\n\r\n";
        assert_eq!(parse_response(raw), Ok((200, "hello".to_owned())));
    }

    #[test]
    fn a_refusal_keeps_its_status() {
        let raw = b"HTTP/1.1 404 Not Found\r\ncontent-length: 2\r\n\r\n{}";
        assert_eq!(parse_response(raw), Ok((404, "{}".to_owned())));
    }

    #[test]
    fn a_response_without_headers_is_unreadable_rather_than_empty() {
        assert!(parse_response(b"HTTP/1.1 200 OK").is_err());
    }

    #[test]
    fn the_reply_is_the_first_choice() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"hi"}}]}"#;
        assert_eq!(content(body), Ok("hi".to_owned()));
        assert!(content(r#"{"choices":[]}"#).is_err());
    }

    #[test]
    fn a_refusal_carries_the_runner_s_own_reason() {
        assert_eq!(
            refusal(400, r#"{"error":{"message":"context too long"}}"#),
            ModelError::Refused("context too long".to_owned())
        );
        assert_eq!(
            refusal(500, "oops"),
            ModelError::Refused("HTTP 500".to_owned())
        );
    }

    #[test]
    fn the_request_does_not_stream_and_keeps_roles() {
        let request = request(
            "qwen",
            &[
                Message::new(Role::System, "rules"),
                Message::new(Role::User, "fix it"),
            ],
        );
        assert_eq!(request["model"], "qwen");
        assert_eq!(request["stream"], false);
        assert_eq!(request["messages"][0]["role"], "system");
        assert_eq!(request["messages"][1]["content"], "fix it");
    }

    #[test]
    fn nothing_listening_has_no_status() {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|listener| listener.local_addr())
            .expect("a port")
            .to_string();
        assert_eq!(get_status(&port, "/health"), None);
    }
}
