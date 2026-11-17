//! What a command found, as JSON first and a person's reading second.
//!
//! `docs/cli-conventions.md`: every document carries `schema_version`, every
//! diagnostic a stable `code` from the crate that raised it, and the human
//! form is rendered from the same data rather than the other way round.

use std::fmt::Write as _;

use serde::Serialize;

/// The layout of the JSON this command writes; a reader written against an
/// older one can say so rather than misread it.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The project will not run as written.
    Error,
    /// It runs, but probably not as meant: an unknown component, say.
    Warning,
}

/// One thing wrong, where it is.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    /// The file, relative to the project.
    pub path: String,
    /// One-based, where the problem has a place in a text file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    /// The scene or prefab entity it is about, by its stable ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
}

impl Diagnostic {
    pub fn error(code: &str, path: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            severity: Severity::Error,
            message: message.into(),
            path: path.to_owned(),
            line: None,
            column: None,
            entity: None,
        }
    }

    pub fn warning(code: &str, path: &str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(code, path, message)
        }
    }

    #[must_use]
    pub fn at(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    #[must_use]
    pub fn on(mut self, entity: &str) -> Self {
        self.entity = Some(entity.to_owned());
        self
    }
}

/// What `sindri project check` answers.
#[derive(Clone, Debug, Serialize)]
pub struct CheckReport {
    pub schema_version: u32,
    pub project: String,
    /// No errors; warnings do not fail a check.
    pub ok: bool,
    /// How many of each kind of file were read.
    pub checked: Checked,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Checked {
    pub scenes: usize,
    pub prefabs: usize,
    pub scripts: usize,
    pub stylesheets: usize,
}

impl CheckReport {
    /// The report as a person reads it: one line a problem, as a compiler
    /// prints them, then a summary.
    #[must_use]
    pub fn human(&self) -> String {
        let mut out = String::new();
        for diagnostic in &self.diagnostics {
            let place = match (diagnostic.line, diagnostic.column) {
                (Some(line), Some(column)) => format!("{}:{line}:{column}", diagnostic.path),
                _ => diagnostic.path.clone(),
            };
            let kind = match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            let entity = diagnostic
                .entity
                .as_deref()
                .map(|entity| format!(" (entity {entity})"))
                .unwrap_or_default();
            let _ = writeln!(
                out,
                "{place}: {kind}[{}]{entity}: {}",
                diagnostic.code, diagnostic.message
            );
        }
        let errors = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count();
        let warnings = self.diagnostics.len() - errors;
        let Checked {
            scenes,
            prefabs,
            scripts,
            stylesheets,
        } = self.checked;
        let _ = writeln!(
            out,
            "{}: {scenes} scenes, {prefabs} prefabs, {scripts} scripts, {stylesheets} stylesheets; {errors} errors, {warnings} warnings",
            if self.ok { "ok" } else { "failed" }
        );
        out
    }
}
