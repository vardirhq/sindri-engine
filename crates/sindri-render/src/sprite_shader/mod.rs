//! Project-authored sprite shaders.
//!
//! A sprite shader is one WGSL function a project writes,
//! `fn sprite_shader(input: SpriteShaderInput) -> vec4<f32>`, and the engine
//! supplies everything around it: the vertex stage, the bindings, the input it
//! is handed and the fragment entry points that call it. The built-in sprite
//! look is the same arrangement with the one-line function that returns
//! `input.color`, so a project shader is never a second pipeline layout or a
//! second vertex format, only a different body.
//!
//! A source is checked when it is made, not when it is first drawn. A shader
//! that does not compile is an error naming the line in the project's file,
//! handed to whoever loaded it, and is never given to the GPU: a frame drawing
//! a sprite whose shader was refused draws it with the built-in look instead.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use thiserror::Error;

const PRELUDE: &str = include_str!("prelude.wgsl");
const EPILOGUE: &str = include_str!("epilogue.wgsl");

/// The body the built-in sprite look is made from.
pub(crate) const BUILT_IN: &str =
    "fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {\n    return input.color;\n}\n";

/// Hands each accepted source an identity of its own.
///
/// Two loads of the same file are two shaders: a reload replaces the pipelines
/// a renderer built for the old one rather than finding them by name and
/// keeping them.
static NEXT_REVISION: AtomicU64 = AtomicU64::new(1);

/// A checked sprite shader, cheap to clone and to carry in a frame.
#[derive(Clone, Debug)]
pub struct SpriteShader {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    name: String,
    revision: u64,
    composed: String,
}

impl PartialEq for SpriteShader {
    fn eq(&self, other: &Self) -> bool {
        self.inner.revision == other.inner.revision
    }
}

impl Eq for SpriteShader {}

impl SpriteShader {
    /// Checks a project's shader and keeps it ready to draw with.
    ///
    /// `name` is the asset path, used in errors and as the key a renderer
    /// replaces pipelines under.
    pub fn new(name: impl Into<String>, source: &str) -> Result<Self, SpriteShaderError> {
        let name = name.into();
        let composed = compose(source);
        check(&name, source, &composed)?;
        Ok(Self {
            inner: Arc::new(Inner {
                name,
                revision: NEXT_REVISION.fetch_add(1, Ordering::Relaxed),
                composed,
            }),
        })
    }

    /// The asset path this shader was loaded from.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Which load of `name` this is. Different for every accepted source.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.inner.revision
    }

    /// The whole module the GPU compiles: prelude, project source, epilogue.
    pub(crate) fn composed(&self) -> &str {
        &self.inner.composed
    }
}

/// The engine's prelude, the project's source, and the engine's epilogue.
pub(crate) fn compose(source: &str) -> String {
    let mut composed = String::with_capacity(PRELUDE.len() + source.len() + EPILOGUE.len() + 1);
    composed.push_str(PRELUDE);
    composed.push_str(source);
    if !source.ends_with('\n') {
        composed.push('\n');
    }
    composed.push_str(EPILOGUE);
    composed
}

/// How many lines come before the project's own first line.
#[cfg(not(target_arch = "wasm32"))]
fn prelude_lines() -> u32 {
    u32::try_from(PRELUDE.lines().count()).unwrap_or(u32::MAX)
}

/// The project source's own line for a line of the composed module, if the
/// composed line is one of the project's.
#[cfg(not(target_arch = "wasm32"))]
fn project_line(composed_line: u32, source: &str) -> Option<u32> {
    let first = prelude_lines() + 1;
    let count = u32::try_from(source.lines().count()).unwrap_or(u32::MAX);
    (composed_line >= first && composed_line < first + count).then(|| composed_line - first + 1)
}

/// Parses and validates the composed module, naming the project's line.
fn check(name: &str, source: &str, composed: &str) -> Result<(), SpriteShaderError> {
    // Without the function, the parser reports the epilogue's call to it as an
    // unknown name on one of the engine's own lines. Say what is missing.
    if !defines_entry(source) {
        return Err(SpriteShaderError::Invalid {
            name: name.to_owned(),
            line: None,
            message: MISSING_FUNCTION.to_owned(),
        });
    }
    validate(name, source, composed)
}

/// The full parse and validation, wherever wgpu carries naga.
///
/// The browser's WebGPU backend hands WGSL to the browser and does not link
/// naga, so there only the entry check above runs. A browser plays exports,
/// and `sindri-export` refuses a shader this would refuse, so what reaches a
/// browser has already passed it.
#[cfg(not(target_arch = "wasm32"))]
fn validate(name: &str, source: &str, composed: &str) -> Result<(), SpriteShaderError> {
    use wgpu::naga;

    let refused = |line: Option<u32>, message: String| SpriteShaderError::Invalid {
        name: name.to_owned(),
        line,
        message,
    };
    let module = naga::front::wgsl::parse_str(composed).map_err(|error| {
        let line = error
            .location(composed)
            .and_then(|at| project_line(at.line_number, source));
        refused(line, error.message().to_owned())
    })?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator.validate(&module).map_err(|error| {
        let line = error
            .location(composed)
            .and_then(|at| project_line(at.line_number, source));
        refused(line, describe(error.as_inner()))
    })?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[allow(clippy::unnecessary_wraps)]
fn validate(_name: &str, _source: &str, _composed: &str) -> Result<(), SpriteShaderError> {
    Ok(())
}

/// Whether `source` declares `sprite_shader` as a function, comments aside.
fn defines_entry(source: &str) -> bool {
    source.lines().any(|line| {
        let code = line.split("//").next().unwrap_or_default();
        let mut words = code.split(|c: char| !(c.is_alphanumeric() || c == '_'));
        let mut previous = "";
        words.any(|word| {
            let found = previous == "fn" && word == "sprite_shader";
            if !word.is_empty() {
                previous = word;
            }
            found
        })
    })
}

const MISSING_FUNCTION: &str =
    "does not define `fn sprite_shader(input: SpriteShaderInput) -> vec4<f32>`";

/// An error and the errors under it, on one line.
#[cfg(not(target_arch = "wasm32"))]
fn describe(error: &dyn std::error::Error) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

/// Why a sprite shader was refused.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SpriteShaderError {
    #[error("{name}{}: {message}", line.map(|line| format!(":{line}")).unwrap_or_default())]
    Invalid {
        name: String,
        /// The line in the project's own file, when the fault is on one.
        line: Option<u32>,
        message: String,
    },
}

#[cfg(test)]
mod tests;
