use sindri_core::EntityId;

use crate::ScriptFailure;

/// Something a script said, and which script said it.
///
/// The entity is carried because "player moved" is not something an author can
/// act on when six entities run the same script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptMessage {
    pub entity: EntityId,
    pub message: String,
}

/// How long one script took in a pass, over every entity that runs it.
///
/// Named by its source and container rather than by entity, because the
/// question a profiler answers is "which script is slow", and six wisps
/// running one script are one answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptTiming {
    pub source: String,
    pub script: String,
    /// How many entities ran it.
    pub runs: u32,
    pub time: std::time::Duration,
}

/// What a frame of scripts produced.
///
/// One value rather than two channels: what was printed and what went wrong are
/// both "what the scripts had to say this frame", and a caller that had to
/// remember to drain a second place would eventually not.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScriptReport {
    /// What scripts printed, in the order they ran.
    pub printed: Vec<ScriptMessage>,
    /// What went wrong, per script. One failing script does not stop the rest.
    pub failures: Vec<ScriptFailure>,
    /// How long each script's tick took, when the host asked for timings
    /// with [`crate::Scripts::set_measuring`]. Empty otherwise.
    pub timings: Vec<ScriptTiming>,
}

impl ScriptReport {
    /// Whether the frame was uneventful, which is the common case and the one
    /// a caller should be able to check without looking at two fields.
    #[must_use]
    pub fn is_quiet(&self) -> bool {
        self.printed.is_empty() && self.failures.is_empty()
    }
}
