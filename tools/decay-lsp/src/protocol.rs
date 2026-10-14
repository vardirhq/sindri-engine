use std::io::{self, BufRead, Write};

use serde_json::Value;

/// One framed message as read off the wire.
pub(crate) enum Incoming {
    Message(Value),
    /// A well-framed body that is not JSON. The frame's length was honoured,
    /// so the stream is still in step and the next message can be read.
    Unparsable(serde_json::Error),
}

/// Reads the next framed message, or `None` at a clean end of input.
///
/// A header block with no usable `Content-Length` is an error rather than an
/// end of input: without a length the body cannot be skipped, so the stream
/// cannot be resynchronised, and stopping silently would look like the client
/// closing it.
pub(crate) fn read_message(input: &mut impl BufRead) -> io::Result<Option<Incoming>> {
    let mut content_length = None;
    let mut read_header = false;
    loop {
        let mut header = String::new();
        if input.read_line(&mut header)? == 0 {
            if read_header {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "input ended inside a message header",
                ));
            }
            return Ok(None);
        }
        if header == "\r\n" || header == "\n" {
            break;
        }
        read_header = true;
        // Header names are case-insensitive, as in HTTP.
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("Content-Length")
        {
            let value = value.trim();
            content_length = Some(value.parse::<usize>().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid Content-Length header {value:?}"),
                )
            })?);
        }
    }

    let Some(length) = content_length else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "message header has no Content-Length",
        ));
    };
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    Ok(Some(match serde_json::from_slice(&body) {
        Ok(value) => Incoming::Message(value),
        Err(error) => Incoming::Unparsable(error),
    }))
}

pub(crate) fn write_message(output: &mut impl Write, value: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(value).map_err(io::Error::other)?;
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(&body)?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(body: &str) -> String {
        format!("Content-Length: {}\r\n\r\n{body}", body.len())
    }

    fn read_all(stream: &str) -> Vec<io::Result<Option<Incoming>>> {
        let mut input = stream.as_bytes();
        let mut results = Vec::new();
        loop {
            let result = read_message(&mut input);
            let done = !matches!(result, Ok(Some(_)));
            results.push(result);
            if done {
                return results;
            }
        }
    }

    fn message(result: &io::Result<Option<Incoming>>) -> &Value {
        match result {
            Ok(Some(Incoming::Message(value))) => value,
            _ => panic!("expected a message"),
        }
    }

    #[test]
    fn reads_sequential_messages_then_a_clean_end() {
        let stream = frame(r#"{"id":1}"#) + &frame(r#"{"id":2}"#);
        let results = read_all(&stream);
        assert_eq!(results.len(), 3);
        assert_eq!(message(&results[0])["id"], 1);
        assert_eq!(message(&results[1])["id"], 2);
        assert!(matches!(results[2], Ok(None)));
    }

    #[test]
    fn ignores_other_headers_and_header_name_case() {
        let body = r#"{"id":7}"#;
        let stream = format!(
            "Content-Type: application/vscode-jsonrpc; charset=utf-8\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        assert_eq!(message(&read_all(&stream)[0])["id"], 7);
    }

    #[test]
    fn counts_the_length_in_bytes_not_characters() {
        let stream = frame(r#"{"text":"blåbær ✓"}"#) + &frame(r#"{"id":2}"#);
        let results = read_all(&stream);
        assert_eq!(message(&results[0])["text"], "blåbær ✓");
        assert_eq!(message(&results[1])["id"], 2);
    }

    #[test]
    fn malformed_json_keeps_the_stream_in_step() {
        let stream = frame("{not json") + &frame(r#"{"id":2}"#);
        let results = read_all(&stream);
        assert!(matches!(results[0], Ok(Some(Incoming::Unparsable(_)))));
        assert_eq!(message(&results[1])["id"], 2);
    }

    #[test]
    fn a_missing_or_invalid_length_is_an_error_not_an_end() {
        for stream in [
            "Content-Type: x\r\n\r\n{}",
            "Content-Length: twelve\r\n\r\n{}",
            "Content-Length: -1\r\n\r\n{}",
        ] {
            let error = read_all(stream)
                .pop()
                .unwrap()
                .err()
                .expect("framing error");
            assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{stream:?}");
        }
    }

    #[test]
    fn input_ending_mid_message_is_an_error() {
        for stream in ["Content-Length: 10\r\n", "Content-Length: 10\r\n\r\n{}"] {
            let error = read_all(stream)
                .pop()
                .unwrap()
                .err()
                .expect("truncated input");
            assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof, "{stream:?}");
        }
    }

    #[test]
    fn written_messages_read_back() {
        let mut buffer = Vec::new();
        write_message(&mut buffer, &serde_json::json!({"id":3,"text":"ø"})).unwrap();
        let mut input = buffer.as_slice();
        let Some(Incoming::Message(value)) = read_message(&mut input).unwrap() else {
            panic!("message");
        };
        assert_eq!(value["text"], "ø");
    }
}
