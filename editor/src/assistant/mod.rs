//! Getting from no local AI at all to an assistant that has proved itself,
//! without leaving the editor.
//!
//! The setup is the feature. A person who cannot get a local model running has
//! no opinion about how good Sindri's assistant is — so there is no installer
//! to run, no download page, no terminal, and nothing to type. One button
//! fetches a pinned llama.cpp server and a pinned model into the person's own
//! folder, checks both against their SHA-256, starts the server, and puts the
//! model through Sindri's own repair cases before anything is offered.
//!
//! What that buys has to be paid for in care, because putting software on
//! someone's machine is a real thing to do. So: the sources are pinned rather
//! than discovered, the cost is stated before anything starts, nothing is
//! installed system-wide or asks for a password, every step can be stopped,
//! and removing it all is one button.
//!
//! [`setup`] reads what is on the machine and says which of a handful of
//! states it is in. The panel draws that; it does not decide it.

pub mod catalogue;
pub mod chat;
pub mod diff;
pub mod fetch;
pub mod hardware;
pub mod install;
pub mod managed;
pub mod repair;
pub mod server;
pub mod verify;

pub use catalogue::{DEFAULT_CONTEXT, Profile, Supports, Tier};

use fetch::Asset;

/// One thing Sindri verified the chosen model can actually do.
///
/// Reported individually rather than as a connection light, because "connected"
/// is not a capability and a model that answers HTTP but cannot hold a schema
/// will fail every proposal it is asked for. A feature that fails verification
/// stays switched off rather than failing later in front of a person's scene.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Feature {
    /// Answers in a strict schema.
    StructuredOutput,
    /// Calls a read-only tool with valid arguments.
    ToolCalling,
    /// Resolves a stable entity reference from a small fixture.
    ReferenceResolution,
    /// Produces a valid component proposal from a supplied schema.
    ComponentProposals,
    /// Writes a small valid Decay function.
    DecayAuthoring,
    /// Corrects an invalid Decay function from compiler diagnostics.
    DecayRepair,
    /// Reads an image.
    Vision,
}

impl Feature {
    /// Every feature verification can report on, in the order it is shown.
    pub const ALL: [Self; 7] = [
        Self::StructuredOutput,
        Self::ToolCalling,
        Self::ReferenceResolution,
        Self::ComponentProposals,
        Self::DecayAuthoring,
        Self::DecayRepair,
        Self::Vision,
    ];

    /// A stable name for saving, which a label is not.
    pub const fn id(self) -> &'static str {
        match self {
            Self::StructuredOutput => "structured_output",
            Self::ToolCalling => "tool_calling",
            Self::ReferenceResolution => "reference_resolution",
            Self::ComponentProposals => "component_proposals",
            Self::DecayAuthoring => "decay_authoring",
            Self::DecayRepair => "decay_repair",
            Self::Vision => "vision",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|feature| feature.id() == id)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::StructuredOutput => "Structured answers",
            Self::ToolCalling => "Tool calling",
            Self::ReferenceResolution => "Scene inspection",
            Self::ComponentProposals => "Component proposals",
            Self::DecayAuthoring => "Decay editing",
            Self::DecayRepair => "Decay repair",
            Self::Vision => "Vision",
        }
    }

    /// Whether verification has a case for it, which is also whether the
    /// editor offers it: nothing is offered that was not tested, and each
    /// tested feature is switched on by its own result alone, so a model that
    /// fails one keeps the others.
    ///
    /// A feature with no case is not reported as failed — nothing asked the
    /// model to do it — but as not offered yet, which is the true answer.
    pub const fn tested(self) -> bool {
        matches!(self, Self::DecayRepair)
    }

    /// How to use it, once it has passed.
    pub const fn how_to_use(self) -> &'static str {
        match self {
            Self::DecayRepair => {
                "Open a script that does not compile and press Propose a fix under its errors."
            }
            _ => "",
        }
    }

    /// What testing it involves, said while the test runs.
    pub const fn test(self) -> &'static str {
        match self {
            Self::DecayRepair => {
                "Sindri gives it two broken scripts and checks both fixes compile."
            }
            _ => "",
        }
    }
}

/// What is on this machine, as far as setup is concerned.
///
/// Gathered by [`Machine::look`]; kept apart from [`setup`] so every state can
/// be tested without a network, a GPU or a model.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Machine {
    /// The runner published for this platform, if one is.
    pub runtime: Option<Asset>,
    /// The model file Sindri sets up.
    pub model: Option<Asset>,
    /// Memory a model could have, in gigabytes, when that could be told.
    pub memory: Option<f32>,
    pub has_runtime: bool,
    pub has_model: bool,
    /// What is known about exactly these files; `None` until setup has seen
    /// the model answer.
    pub known: Option<managed::Known>,
    /// Bytes the assistant takes up on disk.
    pub footprint: u64,
}

impl Machine {
    /// Reads the machine. Quick: no hashing, no network, one or two small
    /// commands for the GPU.
    pub fn look(home: &managed::Home) -> Self {
        let runtime = managed::runtime();
        let model = managed::model_file(managed::MODEL);
        let has_runtime = runtime
            .as_ref()
            .is_some_and(|asset| home.server(asset).is_some());
        let has_model = model.as_ref().is_some_and(|asset| home.has_model(asset));
        let known = match (&runtime, &model, home.load()) {
            (Some(runtime), Some(model), Some(saved)) => saved.known_for(model, runtime),
            _ => None,
        };
        Self {
            footprint: home.footprint(),
            memory: hardware::available_memory(),
            runtime,
            model,
            has_runtime,
            has_model,
            known,
        }
    }
}

/// Where setup stands.
#[derive(Clone, Debug, PartialEq)]
pub enum Setup {
    /// Nothing is published for this kind of computer.
    Unavailable,
    /// Not set up, or set up part of the way: offer to finish it.
    Offer(Offer),
    /// Everything is on disk but the model has not been seen to answer.
    Unchecked,
    /// Set up and answering. Which features it offers is each feature's own
    /// result, in `known`.
    Ready(managed::Known),
}

/// What setting up will cost, said before anything starts.
#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    pub model_name: String,
    /// Bytes still to download.
    pub download: u64,
    /// How well the model would run here; `None` when memory could not be told.
    pub fit: Option<Tier>,
    /// Whether some of it is already here, so the button can say "Continue".
    pub partly_done: bool,
}

/// Reads a machine as one of the setup states.
pub fn setup(machine: &Machine) -> Setup {
    let (Some(runtime), Some(model)) = (&machine.runtime, &machine.model) else {
        return Setup::Unavailable;
    };
    if machine.has_runtime && machine.has_model {
        return machine.known.clone().map_or(Setup::Unchecked, Setup::Ready);
    }
    let profile = catalogue::profile_for(&model.id);
    Setup::Offer(Offer {
        model_name: profile.map_or_else(|| model.display_name.clone(), |p| p.display_name.clone()),
        download: if machine.has_runtime { 0 } else { runtime.size }
            + if machine.has_model { 0 } else { model.size },
        fit: profile
            .zip(machine.memory)
            .map(|(profile, memory)| profile.tier(memory, DEFAULT_CONTEXT)),
        partly_done: machine.has_runtime || machine.has_model,
    })
}

/// The steps setup takes, in order. Each is shown with its own state, so a
/// person always knows which part is happening and how much is left.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Step {
    Runner,
    Model,
    Start,
    Answer,
}

impl Step {
    pub const ALL: [Self; 4] = [Self::Runner, Self::Model, Self::Start, Self::Answer];

    pub const fn title(self) -> &'static str {
        match self {
            Self::Runner => "Get the model runner",
            Self::Model => "Download the model",
            Self::Start => "Start it",
            Self::Answer => "Test that it works",
        }
    }

    /// What the step is doing, in a sentence, while it does it.
    pub const fn doing(self) -> &'static str {
        match self {
            Self::Runner => "A small program that runs the model on your computer.",
            Self::Model => "The model itself. The largest part; it resumes if interrupted.",
            Self::Start => "Loading the model into memory. Usually under a minute.",
            Self::Answer => "Asks it a question with a known answer and checks the reply is right.",
        }
    }
}

/// Human sizes: "31 MB", "4.7 GB".
pub fn size(bytes: u64) -> String {
    // Rounded to the nearest tenth of a gigabyte, as a person would say it.
    let tenths = (bytes + 50_000_000) / 100_000_000;
    if tenths >= 10 {
        format!("{}.{} GB", tenths / 10, tenths % 10)
    } else {
        format!("{} MB", ((bytes + 500_000) / 1_000_000).max(1))
    }
}

#[cfg(test)]
mod tests;
