//! What this engine version can be told to do, written down for a machine.
//!
//! Two questions an agent — or a script author, or CI — asks on arriving at a
//! Sindri project, and neither is answerable by reading Rust:
//!
//! - *What gameplay code am I allowed to write?* Answered by the Decay host
//!   surface: every namespace, call, and member, with its types.
//! - *What am I allowed to author?* Answered by the component registry: every
//!   built-in component, what fields it has, and what a fresh one is.
//!
//! Both already have exactly one authority in the repository. The Decay
//! surface is described once in `sindri-decay` and read by the analyzer and
//! the host alike; the component schemas are registered once in
//! `sindri-scene`. This crate does not add a third: it *reads* those two and
//! writes what it found to `docs/generated/`, and a test fails when the files
//! on disk disagree with the code.
//!
//! That is the whole design constraint. A hand-maintained reference is a
//! reference that is wrong one release later, and an agent that has read a
//! wrong reference writes code that compiles against nothing.

mod components;
mod decay;
mod decay_language;
mod entries;
mod markdown;

use std::{fmt, path::Path};

use sindri_scene::SceneExtractError;

/// The version of the generated documents themselves.
///
/// Bumped when the shape of a file changes, so a reader that was written
/// against an older layout can say so rather than silently misreading it.
pub const SCHEMA_VERSION: u32 = 1;

/// The command that rewrites the generated files, quoted in the files and in
/// the failure a stale checkout produces.
pub const REGENERATE_COMMAND: &str = "cargo run -p sindri-capabilities -- --write";

/// One file this tool owns, and what it should contain right now.
pub struct GeneratedDocument {
    /// Where it lives, relative to the repository root.
    pub path: String,
    pub contents: String,
}

/// Everything that stopped a description being produced.
#[derive(Debug)]
pub enum CapabilitiesError {
    /// The built-in component schemas did not register.
    ///
    /// Not a failure of this tool: the same registration runs when anything
    /// opens a scene, so this is the engine refusing to start.
    Components(SceneExtractError),
    Json(serde_json::Error),
    /// A capability source in `docs/capabilities/` failed its checks; one line
    /// per problem.
    Entries(Vec<String>),
}

impl fmt::Display for CapabilitiesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Components(error) => {
                write!(
                    formatter,
                    "the built-in components did not register: {error}"
                )
            }
            Self::Json(error) => write!(formatter, "the description would not serialize: {error}"),
            Self::Entries(problems) => {
                write!(formatter, "the capability entries do not check:")?;
                for problem in problems {
                    write!(formatter, "\n  {problem}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for CapabilitiesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Components(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Entries(_) => None,
        }
    }
}

impl From<SceneExtractError> for CapabilitiesError {
    fn from(error: SceneExtractError) -> Self {
        Self::Components(error)
    }
}

impl From<serde_json::Error> for CapabilitiesError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// Every generated file, in the order a reader meets them.
///
/// One function rather than three, because the caller that writes them and the
/// test that checks them must agree on the set: a document added to one and
/// not the other is a file nothing keeps current, which is the failure this
/// crate exists to prevent.
pub fn documents() -> Result<Vec<GeneratedDocument>, CapabilitiesError> {
    let decay_api = decay::describe();
    let capabilities = components::describe()?;
    let pages = entries::pages(repository_root()).map_err(CapabilitiesError::Entries)?;

    let mut documents = vec![
        GeneratedDocument {
            path: "docs/generated/decay-api.json".to_owned(),
            contents: to_json(&decay_api.to_json())?,
        },
        GeneratedDocument {
            path: "docs/generated/decay-api.md".to_owned(),
            contents: markdown::render(&decay_api),
        },
        GeneratedDocument {
            path: "docs/generated/decay-language.json".to_owned(),
            contents: to_json(&decay_language::describe())?,
        },
        GeneratedDocument {
            path: "docs/generated/sindri-capabilities.json".to_owned(),
            contents: to_json(&capabilities)?,
        },
    ];
    documents.extend(pages.into_iter().map(|page| GeneratedDocument {
        path: page.path,
        contents: page.contents,
    }));
    Ok(documents)
}

/// The workspace root, from where Cargo put this crate rather than from the
/// current directory, so the tool reads and writes the right files from anywhere.
pub fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate lives two directories below the repository root")
}

/// Pretty-printed and newline-terminated, because these files are read in diffs
/// and by people as often as they are parsed.
fn to_json(value: &serde_json::Value) -> Result<String, CapabilitiesError> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    Ok(text)
}
