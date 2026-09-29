//! The script files a scene names, and their text.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use decay_ir::{IrFunction, lower_with_environment};
use decay_semantic::Environment;
use decay_syntax::{Item, parse};

use super::project::Project;

/// The lifecycle function called once, before the first update.
pub(super) const START: &str = "start";

/// The lifecycle function called every frame, with the frame's delta.
pub(super) const UPDATE: &str = "update";

/// A function the engine calls on a running script by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifecycleFunction {
    pub name: &'static str,
    /// Its parameters, as `(name, type)`.
    pub params: &'static [(&'static str, &'static str)],
    /// When the engine calls it.
    pub when: &'static str,
}

/// Every lifecycle function, in the order a script meets them.
pub const LIFECYCLE: [LifecycleFunction; 2] = [
    LifecycleFunction {
        name: START,
        params: &[],
        when: "once, before the script's first update",
    },
    LifecycleFunction {
        name: UPDATE,
        params: &[("dt", "f32")],
        when: "every frame, with the seconds since the last one",
    },
];

/// The `.decay` sources a world's scripts refer to, by asset ID.
///
/// This crate does no I/O — it has no more business opening a file than
/// `sindri-core` does, and staying out of it is what lets every test here run
/// with no filesystem and no browser. The host fills this the same way the
/// editor fills [`sindri_scene::TextureBindings`]: through `sindri-assets`,
/// which already knows how to fetch a logical ID on either target.
#[derive(Clone, Debug, Default)]
pub struct ScriptSources {
    sources: BTreeMap<String, String>,
    /// Every script's declared shape, and the environment a script in this
    /// project compiles against. Worked out the first time it is asked for and
    /// forgotten whenever a source changes.
    project: OnceLock<(Project, Environment)>,
    /// Every shared function in the project, lowered from the file that
    /// declares it, for a script's program to link in. Forgotten with the
    /// project whenever a source changes.
    shared: OnceLock<SharedFunctions>,
}

/// The project's shared functions, and the files declaring some that do not
/// compile, whose functions are therefore missing.
#[derive(Clone, Debug, Default)]
pub(crate) struct SharedFunctions {
    pub(crate) functions: Vec<IrFunction>,
    /// Each file that declares shared functions and does not compile, with
    /// the functions it declares and the first of its errors.
    pub(crate) broken: Vec<(String, Vec<String>, Vec<String>)>,
}

impl ScriptSources {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, id: impl Into<String>, source: impl Into<String>) {
        self.sources.insert(id.into(), source.into());
        self.project = OnceLock::new();
        self.shared = OnceLock::new();
    }

    pub fn remove(&mut self, id: &str) -> Option<String> {
        self.project = OnceLock::new();
        self.shared = OnceLock::new();
        self.sources.remove(id)
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&str> {
        self.sources.get(id).map(String::as_str)
    }

    /// What a script in this project compiles against: the engine's surface,
    /// and every script in the project as a type it can name.
    #[must_use]
    pub fn environment(&self) -> &Environment {
        &self.project().1
    }

    /// The declared shape of every script, which a compiled program depends on
    /// beyond its own text.
    #[must_use]
    pub fn project_key(&self) -> &str {
        self.project().0.key()
    }

    /// Every script's declared shape.
    pub(crate) fn project_ref(&self) -> &Project {
        &self.project().0
    }

    /// Every shared function in the project, lowered once from the file that
    /// declares it.
    pub(crate) fn shared_functions(&self) -> &SharedFunctions {
        self.shared.get_or_init(|| {
            let mut shared = SharedFunctions::default();
            for (id, source) in &self.sources {
                let declared: Vec<String> = parse(source)
                    .program
                    .items
                    .into_iter()
                    .flat_map(|item| match item {
                        Item::Function(function) if function.shared => vec![function.name],
                        // A struct's methods are every file's, as the struct is.
                        Item::Struct(declared) => declared
                            .methods
                            .iter()
                            .map(|method| {
                                decay_semantic::method_function(&declared.name, &method.name)
                            })
                            .collect(),
                        _ => Vec::new(),
                    })
                    .collect();
                if declared.is_empty() {
                    continue;
                }
                let lowered = lower_with_environment(source, self.environment());
                match lowered.program {
                    Some(program) => {
                        // Only what it shares: its other top-level functions
                        // are its own scripts' business.
                        if let Some(functions) = program.functions() {
                            shared.functions.extend(
                                functions
                                    .functions
                                    .iter()
                                    .filter(|function| declared.contains(&function.name))
                                    .cloned(),
                            );
                        }
                    }
                    None => shared.broken.push((
                        id.clone(),
                        declared,
                        lowered
                            .analysis
                            .diagnostics
                            .iter()
                            .map(|diagnostic| {
                                format!(
                                    "{}:{}: {}",
                                    diagnostic.line, diagnostic.column, diagnostic.message
                                )
                            })
                            .collect(),
                    )),
                }
            }
            shared
        })
    }

    fn project(&self) -> &(Project, Environment) {
        self.project.get_or_init(|| {
            let mut environment = super::environment::environment();
            let project = Project::read(self.sources.values().map(String::as_str), &environment);
            project.describe(&mut environment);
            (project, environment)
        })
    }
}
