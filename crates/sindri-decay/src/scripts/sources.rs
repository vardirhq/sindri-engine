//! The script files a scene names, and their text.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use decay_semantic::Environment;

use super::project::Project;

/// The lifecycle function called once, before the first update.
pub(super) const START: &str = "start";

/// The lifecycle function called every frame, with the frame's delta.
pub(super) const UPDATE: &str = "update";

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
}

impl ScriptSources {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, id: impl Into<String>, source: impl Into<String>) {
        self.sources.insert(id.into(), source.into());
        self.project = OnceLock::new();
    }

    pub fn remove(&mut self, id: &str) -> Option<String> {
        self.project = OnceLock::new();
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

    fn project(&self) -> &(Project, Environment) {
        self.project.get_or_init(|| {
            let mut environment = super::environment::environment();
            let project = Project::read(self.sources.values().map(String::as_str), &environment);
            project.describe(&mut environment);
            (project, environment)
        })
    }
}
