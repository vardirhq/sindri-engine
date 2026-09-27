//! Starting, stopping and talking to the runner, with stand-ins for it.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

use super::super::chat::Role;
use super::*;

fn scratch(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("sindri-server-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a scratch directory");
    directory
}

#[test]
fn the_server_listens_only_on_this_machine() {
    let arguments = arguments(Path::new("/m/model.gguf"), 4242);
    let after = |flag: &str| {
        arguments
            .iter()
            .position(|argument| argument == flag)
            .and_then(|index| arguments.get(index + 1))
            .map(String::as_str)
    };
    assert_eq!(after("--host"), Some("127.0.0.1"));
    assert_eq!(after("--port"), Some("4242"));
    assert_eq!(after("--model"), Some("/m/model.gguf"));
    assert_eq!(after("--ctx-size"), Some("16384"));
}

/// The chat path over a real socket, against a stand-in that answers the way
/// llama.cpp's server does.
#[test]
fn a_reply_travels_over_a_real_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let endpoint = listener.local_addr().expect("an address").to_string();
    let stand_in = std::thread::spawn(move || {
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
        std::io::Read::read_exact(&mut reader, &mut body).expect("the body");
        let answer = r#"{"choices":[{"message":{"role":"assistant","content":"fixed"}}]}"#;
        let mut stream = reader.into_inner();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{answer}",
            answer.len()
        )
        .expect("an answer");
        (request_line, String::from_utf8(body).expect("text"))
    });
    let mut runner = Runner { endpoint };
    let reply = runner.reply(&[Message::new(Role::User, "fix it")]);
    let (request_line, body) = stand_in.join().expect("the stand-in");
    assert_eq!(reply, Ok("fixed".to_owned()));
    assert!(request_line.starts_with("POST /v1/chat/completions "));
    let sent: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(sent["messages"][0]["content"], "fix it");
}

#[test]
fn a_runner_that_is_not_there_is_unreachable() {
    let endpoint = TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("a port")
        .to_string();
    let mut runner = Runner { endpoint };
    assert_eq!(runner.reply(&[]), Err(ModelError::Unreachable));
}

#[cfg(unix)]
fn stand_in_binary(directory: &Path, script: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = directory.join("llama-server");
    std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).expect("written");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("executable");
    path
}

/// A runner that dies while loading is reported with what it said, and a
/// memory failure reads as one.
#[cfg(unix)]
#[test]
fn a_runner_that_dies_while_loading_says_why() {
    let directory = scratch("dies");
    let binary = stand_in_binary(
        &directory,
        "echo 'failed to allocate: out of memory' >&2; exit 1",
    );
    let log = directory.join("runner.log");
    let error = Server::start(&binary, Path::new("/m.gguf"), &log, &AtomicBool::new(false))
        .err()
        .expect("it does not start");
    match &error {
        StartError::Exited { log } => assert!(log.contains("out of memory"), "{log}"),
        other => panic!("{other:?}"),
    }
    assert!(error.friendly().contains("memory"));
}

/// Stopping a start in progress stops the process with it.
#[cfg(unix)]
#[test]
fn a_stopped_start_leaves_nothing_running() {
    let directory = scratch("stopped");
    let marker = directory.join("still-running");
    let binary = stand_in_binary(
        &directory,
        &format!("sleep 2; touch '{}'", marker.display()),
    );
    let error = Server::start(
        &binary,
        Path::new("/m.gguf"),
        &directory.join("runner.log"),
        &AtomicBool::new(true),
    )
    .err()
    .expect("it does not start");
    assert_eq!(error, StartError::Cancelled);
    std::thread::sleep(Duration::from_secs(3));
    assert!(!marker.exists(), "the runner outlived its cancellation");
}

#[test]
fn a_missing_binary_is_a_launch_failure() {
    let directory = scratch("missing");
    let error = Server::start(
        &directory.join("nothing-here"),
        Path::new("/m.gguf"),
        &directory.join("runner.log"),
        &AtomicBool::new(false),
    )
    .err()
    .expect("it does not start");
    assert!(matches!(error, StartError::Launch(_)), "{error:?}");
}

#[test]
fn the_tail_of_a_log_is_its_end() {
    let directory = scratch("tail");
    let log = directory.join("log");
    std::fs::write(&log, "first line\nlast line\n").expect("written");
    assert_eq!(tail(&log, 10), "last line\n");
    assert_eq!(tail(&directory.join("absent"), 10), "");
}
