//! Doing the setup: fetch, unpack, start, test — on a worker, reporting as it
//! goes, and stoppable at every step.
//!
//! The test at the end is of the assistant, not of any one feature: a question
//! with a known answer, so "set up" means a model that runs here and replies
//! correctly. What it can do in the editor is tested afterwards, feature by
//! feature, by [`check_features`].
//!
//! Each step is skipped when its result is already on disk, so running setup
//! again after an interruption costs only what is missing, and a download that
//! stopped part-way resumes rather than starting over.

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use super::chat::{Message, Model, Role};
use super::fetch::{Asset, Trouble, fetch, watched_transport};
use super::managed::{Home, Saved, unpack};
use super::server::{Server, StartError};
use super::verify::{Verdict, verify};
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
    /// It started, but did not answer the test question correctly.
    #[error("the test question was answered with {0:?}")]
    Wrong(String),
    #[error("saving the result: {0}")]
    Save(std::io::Error),
}

impl Failure {
    pub const fn step(&self) -> Step {
        match self {
            Self::Download { step, .. } => *step,
            Self::Unpack(_) => Step::Runner,
            Self::Start(_) => Step::Start,
            Self::Wrong(_) | Self::Save(_) => Step::Answer,
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
            Self::Wrong(_) => "The model started but did not answer correctly. Try again; \
                               if it keeps happening, remove the assistant and set it up \
                               again, which replaces a damaged model file."
                .to_owned(),
            Self::Save(_) => "The model works, but Sindri could not write that down. Check \
                              that your disk is not full."
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

/// The question setup ends on, and the answer it must contain.
///
/// Arithmetic because the answer is not a matter of opinion or phrasing, and
/// small enough that any working model gets it: a wrong or empty reply means
/// the runtime, the model file or the chat template is broken, not that the
/// model is weak.
pub const TEST_QUESTION: &str = "What is 17 + 25? Reply with only the number.";
pub const TEST_ANSWER: &str = "42";

/// Whether a reply to the test question is right.
pub fn answers_correctly(reply: &str) -> bool {
    reply
        .split(|c: char| !c.is_ascii_digit())
        .any(|number| number == TEST_ANSWER)
}

/// Asks the test question.
pub fn test(model: &mut dyn Model) -> Result<(), Failure> {
    let reply = model
        .reply(&[Message::new(Role::User, TEST_QUESTION)])
        .map_err(|error| Failure::Wrong(error.to_string()))?;
    if answers_correctly(&reply) {
        Ok(())
    } else {
        Err(Failure::Wrong(reply.chars().take(80).collect()))
    }
}

/// Runs every step that is not already done, ending on the test question, and
/// hands back the running server.
pub fn install(
    home: &Home,
    runtime: &Asset,
    model: &Asset,
    shared: &Shared,
) -> Result<Server, Failure> {
    let server = start(home, runtime, model, shared)?;
    shared.begin(Step::Answer, 0);
    test(&mut server.model())?;
    if shared.cancel.load(Ordering::Relaxed) {
        return Err(Failure::Start(StartError::Cancelled));
    }
    // Kept over a record for these same files, so testing again after a
    // failed start does not forget what the features proved.
    let saved = home
        .load()
        .filter(|saved| saved.known_for(model, runtime).is_some())
        .unwrap_or_else(|| Saved::answering(model, runtime));
    home.save(&saved).map_err(Failure::Save)?;
    Ok(server)
}

/// Tests each feature the editor offers against a running model, and records
/// the outcome — passed or not — so it is not asked again for these files.
pub fn check_features(
    home: &Home,
    runtime: &Asset,
    model: &Asset,
    running: &mut dyn Model,
    cancel: &AtomicBool,
) -> Result<Verdict, Failure> {
    let verdict = verify(running, cancel);
    if cancel.load(Ordering::Relaxed) {
        return Err(Failure::Start(StartError::Cancelled));
    }
    let tested: Vec<Feature> = Feature::ALL
        .into_iter()
        .filter(|feature| feature.tested())
        .collect();
    let mut saved = home
        .load()
        .filter(|saved| saved.known_for(model, runtime).is_some())
        .unwrap_or_else(|| Saved::answering(model, runtime));
    saved.record(&tested, &verdict.verified);
    home.save(&saved).map_err(Failure::Save)?;
    Ok(verdict)
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
