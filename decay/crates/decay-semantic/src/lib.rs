//! Semantic analysis for the Decay gameplay language.
//!
//! This crate remains engine-agnostic. A host such as Sindri supplies globals
//! and host types through [`Environment`] rather than being compiled into the
//! language itself.
//!
//! `types` and `environment` are what the analysis is told; `analyzer` is the
//! walk that checks a program against them, one file per kind of thing it
//! walks into; `diagnostic` is what it reports.

mod analyzer;
pub mod codes;
mod constant;
mod diagnostic;
mod environment;
pub mod members;
mod types;

#[cfg(test)]
mod tests;

use decay_syntax::parse;

use analyzer::Analyzer;

pub use constant::{ConstValue, FoldError, fold_constants};
pub use decay_syntax::{ListOp, MapOp, NumberOp, StringOp, TimerProperty, VectorOp};
pub use diagnostic::{
    Analysis, Binding, ConstantUses, Diagnostic, DiagnosticPhase, ValueMember, ValueMembers,
};
pub use environment::{Environment, ExternalSymbol, StateField};
pub use types::{
    BUILT_IN_TYPES, COMPONENTS, EMIT, FunctionType, HostType, LENGTH, LIST, MAP, OLD_LENGTH, TIMER,
    Type, VEC2, VEC3, enum_type, event_type, method_function,
};

#[must_use]
pub fn analyze(source: &str) -> Analysis {
    analyze_with_environment(source, &Environment::default())
}

// See `ValueMembers` for why the analysis carries a map of a zero-sized value.
#[allow(clippy::zero_sized_map_values)]
#[must_use]
pub fn analyze_with_environment(source: &str, environment: &Environment) -> Analysis {
    let parsed = parse(source);
    let mut diagnostics = parsed
        .diagnostics
        .into_iter()
        .map(|diagnostic| Diagnostic {
            phase: DiagnosticPhase::Syntax,
            code: diagnostic.code.id(),
            message: diagnostic.message,
            span: diagnostic.span,
            line: diagnostic.line,
            column: diagnostic.column,
        })
        .collect::<Vec<_>>();

    let mut value_members = ValueMembers::new();
    let mut analyzer = Analyzer::new(source, environment, &mut diagnostics, &mut value_members);
    analyzer.analyze_program(&parsed.program);
    let enums = analyzer.known_enums();
    let structs = analyzer.known_structs();
    let bindings = std::mem::take(&mut analyzer.bindings);
    let constant_uses = std::mem::take(&mut analyzer.constant_uses);
    let constants = std::mem::take(&mut analyzer.own_constants);
    let method_calls = std::mem::take(&mut analyzer.method_calls);
    let struct_methods = analyzer.known_methods();
    let struct_defaults = analyzer.known_defaults();

    Analysis {
        program: parsed.program,
        diagnostics,
        value_members,
        enums,
        structs,
        bindings,
        constant_uses,
        constants,
        method_calls,
        struct_methods,
        struct_defaults,
    }
}
