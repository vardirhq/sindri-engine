//! Running the model: the llama.cpp server Sindri unpacked, as a child of the
//! editor.
//!
//! Started only when something needs it, on a free loopback port, and stopped
//! when the editor lets go of it — a model holds gigabytes of memory, and one
//! left running after the editor closed would be Sindri taking something it
//! was not using. What it prints goes to a log in the assistant's folder, so a
//! failure to start can be explained from its own words.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::DEFAULT_CONTEXT;
use super::chat::{
    Message, Model, ModelError, content, get_status, parse_response, post, refusal, request,
};

/// How long loading a model may take before the start is called failed.
///
/// Reading five gigabytes from a slow disk and moving it onto a GPU is honestly
/// a minute or two.
const LOAD_TIMEOUT: Duration = Duration::from_mins(3);

/// A running server, stopped when dropped.
pub struct Server {
    child: Child,
    endpoint: String,
    log: PathBuf,
}

/// Why the server is not up.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum StartError {
    #[error("the model runner could not be started: {0}")]
    Launch(String),
    #[error("the model runner stopped while loading the model")]
    Exited { log: String },
    #[error("the model took longer than {} minutes to load", LOAD_TIMEOUT.as_secs() / 60)]
    TimedOut,
    #[error("stopped")]
    Cancelled,
}

impl StartError {
    /// What to tell a person.
    pub fn friendly(&self) -> String {
        match self {
            Self::Launch(_) => "Sindri could not start the model runner. Removing the \
                                assistant and setting it up again usually fixes this."
                .to_owned(),
            Self::Exited { log } if log.contains("out of memory") || log.contains("alloc") => {
                "The model did not fit in this computer's memory. Close other large \
                 programs and try again."
                    .to_owned()
            }
            Self::Exited { .. } => "The model runner stopped while loading the model. If it \
                                    keeps happening, remove the assistant and set it up again; \
                                    that replaces a damaged model file."
                .to_owned(),
            Self::TimedOut => "The model is taking unusually long to load. Try again; \
                               a second start is usually faster."
                .to_owned(),
            Self::Cancelled => "Stopped.".to_owned(),
        }
    }
}

/// The arguments the server is started with.
///
/// All of the model's layers on the GPU where there is one — llama.cpp keeps
/// on the CPU whatever does not fit, or everything on a machine without a
/// usable GPU — at the context the catalogue estimates memory against.
pub fn arguments(model: &Path, port: u16) -> Vec<String> {
    vec![
        "--model".to_owned(),
        model.display().to_string(),
        "--ctx-size".to_owned(),
        DEFAULT_CONTEXT.to_string(),
        "--n-gpu-layers".to_owned(),
        "999".to_owned(),
        "--jinja".to_owned(),
        "--host".to_owned(),
        "127.0.0.1".to_owned(),
        "--port".to_owned(),
        port.to_string(),
    ]
}

impl Server {
    /// Starts the server and waits for the model to load.
    pub fn start(
        binary: &Path,
        model: &Path,
        log: &Path,
        cancel: &AtomicBool,
    ) -> Result<Self, StartError> {
        let port = free_port().map_err(|error| StartError::Launch(error.to_string()))?;
        let output = File::create(log).map_err(|error| StartError::Launch(error.to_string()))?;
        let errors = output
            .try_clone()
            .map_err(|error| StartError::Launch(error.to_string()))?;
        let child = Command::new(binary)
            .args(arguments(model, port))
            // Beside its own libraries, which a release build looks for there.
            .current_dir(binary.parent().unwrap_or_else(|| Path::new(".")))
            .stdin(Stdio::null())
            .stdout(output)
            .stderr(errors)
            .spawn()
            .map_err(|error| StartError::Launch(error.to_string()))?;
        let mut server = Self {
            child,
            endpoint: format!("127.0.0.1:{port}"),
            log: log.to_path_buf(),
        };
        server.wait_until_loaded(cancel)?;
        Ok(server)
    }

    fn wait_until_loaded(&mut self, cancel: &AtomicBool) -> Result<(), StartError> {
        let started = Instant::now();
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(StartError::Cancelled);
            }
            if let Ok(Some(_)) = self.child.try_wait() {
                return Err(StartError::Exited {
                    log: tail(&self.log, 4096),
                });
            }
            // `/health` answers 503 while the model loads and 200 once it has.
            if get_status(&self.endpoint, "/health") == Some(200) {
                return Ok(());
            }
            if started.elapsed() > LOAD_TIMEOUT {
                return Err(StartError::TimedOut);
            }
            std::thread::sleep(Duration::from_millis(400));
        }
    }

    /// Whether it is still running.
    pub fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Something to ask, bound to this server.
    pub fn model(&self) -> Runner {
        Runner {
            endpoint: self.endpoint.clone(),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A model reached through a running server's chat endpoint.
#[derive(Clone, Debug)]
pub struct Runner {
    pub endpoint: String,
}

impl Model for Runner {
    fn reply(&mut self, conversation: &[Message]) -> Result<String, ModelError> {
        // The server holds one model and ignores the name; it is sent because
        // the API requires one.
        let body = request("local", conversation).to_string();
        let raw = post(&self.endpoint, "/v1/chat/completions", &body)?;
        let (status, body) = parse_response(&raw)?;
        if status != 200 {
            return Err(refusal(status, &body));
        }
        content(&body)
    }
}

/// A port nothing is listening on, found by asking the system for one.
fn free_port() -> std::io::Result<u16> {
    std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()
        .map(|address| address.port())
}

/// The last `bytes` of a file, as text.
pub fn tail(path: &Path, bytes: u64) -> String {
    let Ok(mut file) = File::open(path) else {
        return String::new();
    };
    let length = file.metadata().map_or(0, |meta| meta.len());
    let _ = file.seek(SeekFrom::Start(length.saturating_sub(bytes)));
    let mut text = Vec::new();
    let _ = file.read_to_end(&mut text);
    String::from_utf8_lossy(&text).into_owned()
}

#[cfg(test)]
mod tests;
