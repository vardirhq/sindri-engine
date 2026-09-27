//! Doing the setup: fetch, unpack, start, check — on a worker, reporting as it
//! goes, and stoppable at every step.
//!
//! Each step is skipped when its result is already on disk, so running setup
//! again after an interruption costs only what is missing, and a download that
//! stopped part-way resumes rather than starting over.

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use super::fetch::{Asset, Trouble, fetch, watched_transport};
use super::managed::{Home, Saved, unpack};
use super::server::{Server, StartError};
use super::verify::verify;
use super::{Feature, Step};

/// Where a running setup has got to, read by the panel every frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Progress {
    pub step: Option<Step>,
    /// Bytes arrived and expected, for a download.
    pub arrived: u64,
    pub total: u64,
    /// Whether a finished download is being checked against its hash.
    pub checking: bool,
}

/// What the worker and the panel share.
#[derive(Default)]
pub struct Shared {
    progress: Mutex<Progress>,
    pub cancel: AtomicBool,
}

impl Shared {
    pub fn progress(&self) -> Progress {
        self.progress
            .lock()
            .map(|progress| *progress)
            .unwrap_or_default()
    }

    fn set(&self, change: impl FnOnce(&mut Progress)) {
        if let Ok(mut progress) = self.progress.lock() {
            change(&mut progress);
        }
    }

    fn begin(&self, step: Step, total: u64) {
        self.set(|progress| {
            *progress = Progress {
                step: Some(step),
                arrived: 0,
                total,
                checking: false,
            };
        });
    }

    pub fn stop(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Why setup did not finish, and at which step.
#[derive(Debug, thiserror::Error)]
pub enum Failure {
    #[error("{step:?}: {trouble}")]
    Download { step: Step, trouble: Trouble },
    #[error("unpacking the runner: {0}")]
    Unpack(std::io::Error),
    #[error("starting: {0}")]
    Start(StartError),
    #[error("saving what was checked: {0}")]
    Save(std::io::Error),
}

impl Failure {
    pub const fn step(&self) -> Step {
        match self {
            Self::Download { step, .. } => *step,
            Self::Unpack(_) => Step::Runner,
            Self::Start(_) => Step::Start,
            Self::Save(_) => Step::Check,
        }
    }

    pub const fn cancelled(&self) -> bool {
        matches!(
            self,
            Self::Download {
                trouble: Trouble::Cancelled,
                ..
            } | Self::Start(StartError::Cancelled)
        )
    }

    /// What to tell a person.
    pub fn friendly(&self) -> String {
        match self {
            Self::Download { trouble, .. } => trouble.friendly(),
            Self::Unpack(_) => "The runner could not be unpacked. Try again; if it keeps \
                                happening, remove the assistant and set it up afresh."
                .to_owned(),
            Self::Start(error) => error.friendly(),
            Self::Save(_) => "The model passed, but Sindri could not write down that it \
                              did. Check that your disk is not full."
                .to_owned(),
        }
    }

    /// The runner's own words, for a failure it explained.
    pub fn details(&self) -> Option<&str> {
        match self {
            Self::Start(StartError::Exited { log }) => Some(log),
            _ => None,
        }
    }
}

/// A finished setup: the server, running, and what the model proved.
pub struct Finished {
    pub server: Server,
    pub verified: Vec<Feature>,
}

/// Runs every step that is not already done.
pub fn install(
    home: &Home,
    runtime: &Asset,
    model: &Asset,
    shared: &Shared,
) -> Result<Finished, Failure> {
    let server = start(home, runtime, model, shared)?;
    shared.begin(Step::Check, 0);
    let verdict = verify(&mut server.model(), &shared.cancel);
    if shared.cancel.load(Ordering::Relaxed) {
        return Err(Failure::Start(StartError::Cancelled));
    }
    home.save(&Saved::new(model, runtime, &verdict.verified))
        .map_err(Failure::Save)?;
    Ok(Finished {
        server,
        verified: verdict.verified,
    })
}

/// Gets the runner and model onto disk if they are not, and starts the server.
///
/// What a repair uses when the assistant is set up but not running, as well as
/// the first half of setup.
pub fn start(
    home: &Home,
    runtime: &Asset,
    model: &Asset,
    shared: &Shared,
) -> Result<Server, Failure> {
    let binary = if let Some(binary) = home.server(runtime) {
        binary
    } else {
        let archive = home
            .downloads()
            .join(runtime.url.rsplit('/').next().unwrap_or("runner"));
        download(Step::Runner, runtime, &archive, shared)?;
        let binary = unpack(&archive, &home.runtime_dir(runtime)).map_err(Failure::Unpack)?;
        // The archive has done its job; the unpacked runner is what stays.
        let _ = std::fs::remove_file(&archive);
        binary
    };
    if !home.has_model(model) {
        download(Step::Model, model, &home.model_path(model), shared)?;
    }
    shared.begin(Step::Start, 0);
    Server::start(
        &binary,
        &home.model_path(model),
        &home.log(),
        &shared.cancel,
    )
    .map_err(Failure::Start)
}

fn download(step: Step, asset: &Asset, into: &Path, shared: &Shared) -> Result<(), Failure> {
    shared.begin(step, asset.size);
    let fail = |trouble| Failure::Download { step, trouble };
    let transport = watched_transport(&shared.cancel, |arrived| {
        shared.set(|progress| progress.arrived = arrived);
    });
    fetch(asset, into, |url, part| {
        transport(url, part)?;
        // The bytes are in; hashing five gigabytes takes a few seconds, and
        // the bar should say so rather than sit at full.
        shared.set(|progress| {
            progress.arrived = progress.total;
            progress.checking = true;
        });
        Ok(())
    })
    .map(|_| ())
    .map_err(fail)
}

#[cfg(test)]
mod tests;
