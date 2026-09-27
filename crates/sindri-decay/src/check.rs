//! Checking a Decay source against Sindri's host without running it.
//!
//! The same analysis a scene's scripts are compiled with, offered for text that
//! belongs to no scene yet: a file being previewed, or a candidate an editor
//! tool is weighing before it is written anywhere. Anything that asks "would
//! this compile here?" asks it through this, so the answer cannot drift from
//! what Play will actually do.

use decay_ir::lower_with_environment;
use decay_semantic::DiagnosticPhase;
use decay_syntax::Item;

use crate::scripts::environment;

/// Which part of the compiler objected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckPhase {
    /// The text does not parse.
    Syntax,
    /// It parses, but names something that does not exist or does not fit.
    Semantic,
}

/// One objection, located in the text it was raised against.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDiagnostic {
    pub phase: CheckPhase,
    pub message: String,
    /// One-based, as a person counts them.
    pub line: usize,
    pub column: usize,
    /// Byte offsets into the source.
    pub start: usize,
    pub end: usize,
}

/// What checking a source found.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceCheck {
    pub diagnostics: Vec<SourceDiagnostic>,
    /// The `script` and `component` containers the text declares, in order.
    ///
    /// Read from what parsed, so a source that fails analysis still says what
    /// it meant to declare — which is what lets a caller notice that a
    /// proposed replacement quietly dropped a script a scene names.
    pub declared: Vec<String>,
    compiled: bool,
}

impl SourceCheck {
    /// Whether the source compiles against Sindri's host exactly as a scene's
    /// script would.
    #[must_use]
    pub const fn compiles(&self) -> bool {
        self.compiled
    }
}

/// Checks a source against the host surface every Sindri script runs with.
#[must_use]
pub fn check_source(source: &str) -> SourceCheck {
    let lowered = lower_with_environment(source, &environment());
    let declared = lowered
        .analysis
        .program
        .items
        .iter()
        .map(|item| match item {
            Item::Script(container) | Item::Component(container) => container.name.clone(),
        })
        .collect();
    let diagnostics = lowered
        .analysis
        .diagnostics
        .into_iter()
        .map(|diagnostic| SourceDiagnostic {
            phase: match diagnostic.phase {
                DiagnosticPhase::Syntax => CheckPhase::Syntax,
                DiagnosticPhase::Semantic => CheckPhase::Semantic,
            },
            message: diagnostic.message,
            line: diagnostic.line,
            column: diagnostic.column,
            start: diagnostic.span.start,
            end: diagnostic.span.end,
        })
        .collect();
    SourceCheck {
        diagnostics,
        declared,
        compiled: lowered.program.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_valid_script_compiles_and_names_what_it_declares() {
        let check =
            check_source("script Hud {\n    fn update(dt: f32) {\n        let x = dt;\n    }\n}\n");
        assert!(check.compiles(), "{:?}", check.diagnostics);
        assert!(check.diagnostics.is_empty());
        assert_eq!(check.declared, ["Hud"]);
    }

    #[test]
    fn an_unknown_host_name_is_located_and_does_not_compile() {
        let source = "script Hud {\n    fn start() {\n        Wrold.find(\"Banner\");\n    }\n}\n";
        let check = check_source(source);
        assert!(!check.compiles());
        let first = check.diagnostics.first().expect("a diagnostic");
        assert_eq!(first.phase, CheckPhase::Semantic);
        assert_eq!(first.line, 3);
        assert!(source[first.start..].starts_with("Wrold"), "{first:?}");
        // What it meant to declare survives the failure.
        assert_eq!(check.declared, ["Hud"]);
    }

    #[test]
    fn text_that_does_not_parse_says_so() {
        let check = check_source("script Hud {\n    fn start( {\n}\n");
        assert!(!check.compiles());
        assert!(
            check
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.phase == CheckPhase::Syntax)
        );
    }
}
