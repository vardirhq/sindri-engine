//! Asking the local model something and getting its answer back as text.
//!
//! [`Model`] is the whole boundary: a conversation goes in, a reply comes out.
//! Everything that decides what to ask and whether the answer is any good lives
//! above it, which is what lets the repair loop and the verification cases run
//! in tests against a scripted model with no runner, no GPU and no network.
//!
//! [`Ollama`] is the one transport today, written against the standard library
//! for the reason `probe` gives: it talks to a loopback address, so it needs no
//! TLS, and the editor stays free of an HTTP stack.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use serde_json::{Value, json};

use super::DEFAULT_CONTEXT;
use super::probe::ENDPOINT;

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

/// A model served by a local Ollama runner.
pub struct Ollama {
    pub model: String,
    /// Where the runner listens: its default port, unless a test says otherwise.
    pub endpoint: String,
}

impl Ollama {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            endpoint: ENDPOINT.to_owned(),
        }
    }

    /// The request body, separate from sending it so its shape is testable.
    pub fn request(&self, conversation: &[Message]) -> Value {
        json!({
            "model": self.model,
            "messages": conversation
                .iter()
                .map(|message| json!({ "role": message.role.wire(), "content": message.content }))
                .collect::<Vec<_>>(),
            "stream": false,
            // Low, because the task is to produce text a compiler will accept,
            // not to be inventive about it.
            "options": { "temperature": 0.2, "num_ctx": DEFAULT_CONTEXT },
        })
    }
}

impl Model for Ollama {
    fn reply(&mut self, conversation: &[Message]) -> Result<String, ModelError> {
        let body = self.request(conversation).to_string();
        let raw = post(&self.endpoint, "/api/chat", &body)?;
        let (status, body) = parse_response(&raw)?;
        if status != 200 {
            let reason = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|value| value.get("error")?.as_str().map(str::to_owned))
                .unwrap_or_else(|| format!("HTTP {status}"));
            return Err(ModelError::Refused(reason));
        }
        content(&body)
    }
}

/// The reply text out of a `/api/chat` answer.
pub fn content(body: &str) -> Result<String, ModelError> {
    let value: Value =
        serde_json::from_str(body).map_err(|error| ModelError::Unreadable(error.to_string()))?;
    value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ModelError::Unreadable("no message content".to_owned()))
}

fn post(endpoint: &str, path: &str, body: &str) -> Result<Vec<u8>, ModelError> {
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
    fn the_reply_is_the_message_content() {
        let body = r#"{"model":"m","message":{"role":"assistant","content":"hi"},"done":true}"#;
        assert_eq!(content(body), Ok("hi".to_owned()));
        assert!(content(r#"{"done":true}"#).is_err());
    }

    /// The whole wire path over a real socket, against a stand-in that answers
    /// the way the runner does. Nothing else in the suite opens a connection.
    #[test]
    fn a_reply_travels_over_a_real_connection() {
        use std::io::{BufRead, BufReader};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let endpoint = listener.local_addr().expect("an address").to_string();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("a connection");
            let mut reader = BufReader::new(stream);
            let mut request_line = String::new();
            reader.read_line(&mut request_line).expect("a request line");
            let mut length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).expect("a header");
                if header.trim().is_empty() {
                    break;
                }
                if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().expect("a length");
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).expect("the body");
            let answer = r#"{"message":{"role":"assistant","content":"fixed"},"done":true}"#;
            let mut stream = reader.into_inner();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{answer}",
                answer.len()
            )
            .expect("an answer");
            (request_line, String::from_utf8(body).expect("text"))
        });
        let mut model = Ollama {
            model: "qwen".to_owned(),
            endpoint,
        };
        let reply = model.reply(&[Message::new(Role::User, "fix it")]);
        let (request_line, body) = server.join().expect("the stand-in");
        assert_eq!(reply, Ok("fixed".to_owned()));
        assert!(request_line.starts_with("POST /api/chat "));
        let sent: Value = serde_json::from_str(&body).expect("json");
        assert_eq!(sent["messages"][0]["content"], "fix it");
    }

    #[test]
    fn a_runner_that_is_not_there_is_unreachable() {
        let mut model = Ollama {
            model: "qwen".to_owned(),
            // A port nothing listens on: bound, read, and let go.
            endpoint: std::net::TcpListener::bind("127.0.0.1:0")
                .and_then(|listener| listener.local_addr())
                .expect("a port")
                .to_string(),
        };
        assert_eq!(model.reply(&[]), Err(ModelError::Unreachable));
    }

    #[test]
    fn the_request_names_the_model_and_does_not_stream() {
        let request = Ollama::new("qwen").request(&[
            Message::new(Role::System, "rules"),
            Message::new(Role::User, "fix it"),
        ]);
        assert_eq!(request["model"], "qwen");
        assert_eq!(request["stream"], false);
        assert_eq!(request["messages"][0]["role"], "system");
        assert_eq!(request["messages"][1]["content"], "fix it");
    }
}
