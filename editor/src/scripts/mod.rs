//! The Decay scripts an open scene runs.
//!
//! The mirror of [`crate::textures`], and for the same reason: a scene's script
//! references are the statement of what it needs, the directory it lives in is
//! where they resolve, and `sindri-assets` is the one thing that knows how to
//! fetch a logical ID on either target. `sindri-decay` deliberately does no I/O,
//! so this is where a `.decay` file becomes text — and a text asset is all the
//! pipeline needs to know it is.
//!
//! Hot reload comes free of that arrangement. A script is watched once it has
//! loaded, a changed file is fetched again, and the next frame compiles the new
//! source because [`Scripts`] recompiles when the text it holds stops matching.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use sindri_assets::{
    AssetLoadOutcome, AssetLoadQueueConfig, AssetLoader, AssetWatch, FileSystemAssetSource,
    TextAssetDecoder,
};
use sindri_core::{
    AssetId, AssetStatus, ComponentSchemaRegistry, PrefabDocument, ProfileDocument, World,
};
use sindri_decay::{
    Physics2d, PrefabSources, ProfileSources, ScriptExport, ScriptFailure, ScriptFrame,
    ScriptReport, ScriptSources, Scripts, referenced_sources,
};
use sindri_platform::InputState;

/// What one editor frame gives the scripts in it.
///
/// Bundled for the same reason the engine's `ScriptFrame` is: every capability
/// Play grows adds a parameter, and a list of eight is one nobody reads.
pub struct EditorFrame<'a> {
    pub input: &'a InputState,
    /// The physics Play is stepping, so a script can drive a body and be told
    /// what it touched. `None` while the scene is at rest, which is when
    /// nothing is stepping and a `Physics.*` call should say so rather than
    /// answer about a simulation nobody is running.
    pub physics: Option<Physics2d<'a>>,
    pub physics3d: Option<sindri_decay::Physics3d<'a>>,
    /// Scene-owned movement input and results for Play.
    pub characters: Option<sindri_decay::Characters2d<'a>>,
    pub screen_ui: &'a sindri_scene::ScreenUi,
    pub random: &'a mut sindri_core::Rng,
    pub saves: &'a mut sindri_core::SaveStore,
    pub effects: &'a mut sindri_scene::Effects2d,
    /// Where each animated sprite has got to, so a script can ask whether a
    /// clip has finished and play one again from its start.
    pub animations: &'a mut sindri_scene::SpriteAnimations,
    /// Where each playing sequence has got to, so a script can wait on a cue.
    pub sequences: &'a mut sindri_scene::Sequences,
    pub delta_seconds: f32,
}

/// One worker, because scripts, prefabs and profiles are small text files; and
/// no limit on how many wait, because opening a project asks for every one of
/// them at once. There used to be a limit of sixteen, which Orbital Baked's 59
/// scripts overran on every open: the rest were refused, compiled late against
/// a half-loaded project, and filled the console with errors that went away by
/// themselves.
const QUEUE: AssetLoadQueueConfig = AssetLoadQueueConfig::unbounded(1);

/// How often the files behind the loaded scripts are examined. The same second
/// textures use, for the same reason.
const WATCH_INTERVAL: Duration = Duration::from_secs(1);

/// Something worth telling the author about a script.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScriptNote {
    Loaded(String),
    Reloaded(String),
    Failed(String),
}

/// Every script the open scene runs, and the sources behind them.
pub struct SceneScripts {
    /// `None` when there is no directory to resolve against — a scene that has
    /// never been saved, or one that failed to open.
    loader: Option<AssetLoader<TextAssetDecoder>>,
    watch: Option<AssetWatch>,
    last_examined: Instant,
    /// Every script, prefab and profile in the project, whether or not the
    /// scene names it.
    ///
    /// Scripts, because any script may name any other by type (`Bolt.on(hit)`)
    /// or call its shared functions, so compiling one needs all of them.
    /// Prefabs and profiles, because a prefab spawned at runtime can name
    /// another — an enemy that drops a power-up — which nothing in the scene
    /// mentions. Listed when the scene opens and again on the watch interval,
    /// so a file created a moment ago is picked up a moment later.
    project_scripts: Vec<String>,
    sources: ScriptSources,
    /// The prefabs the scene's scripts can spawn.
    ///
    /// Loaded through the same text loader as the scripts, because a prefab is
    /// a JSON document and the pipeline has no reason to know more than that.
    /// A document that will not parse is reported once, here, rather than on
    /// the frame a script spawns it.
    prefabs: PrefabSources,
    /// Counts changes to `prefabs`, so the textures a prefab draws with can be
    /// asked for when one arrives rather than on the next edit.
    prefab_revision: u64,
    profiles: ProfileSources,
    scripts: Scripts,
}

impl SceneScripts {
    pub fn for_scene(scene: Option<&Path>) -> Self {
        let root = root_of(scene);
        Self {
            loader: root.as_deref().and_then(|root| {
                AssetLoader::new(FileSystemAssetSource::new(root), QUEUE, TextAssetDecoder).ok()
            }),
            watch: root.clone().map(AssetWatch::new),
            last_examined: Instant::now(),
            project_scripts: root.as_deref().map_or_else(Vec::new, list_scripts),
            sources: ScriptSources::new(),
            prefabs: PrefabSources::new(),
            prefab_revision: 0,
            profiles: ProfileSources::new(),
            scripts: Scripts::new(),
        }
    }

    /// Asks for every source the world's scripts name, and lets go of the rest.
    pub fn request(
        &mut self,
        world: &World,
        components: &ComponentSchemaRegistry,
    ) -> Vec<ScriptNote> {
        let mut notes = Vec::new();
        let mut referenced = referenced_sources(world, components);
        // Prefabs are named by the *declared type* of a script's exported
        // fields, so they only become visible once the script has compiled.
        // Asking every frame is what makes a prefab authored a moment ago load
        // a moment later rather than at the next scene open.
        referenced.extend(self.project_scripts.iter().cloned());
        referenced.extend(self.scripts.referenced_prefabs(world, components));
        referenced.extend(self.scripts.referenced_profiles(world, components));
        match sindri_scene::referenced_physics_materials(world, components) {
            Ok(materials) => referenced.extend(materials),
            Err(error) => notes.push(ScriptNote::Failed(error.to_string())),
        }
        let wanted: BTreeSet<AssetId> = referenced
            .iter()
            .filter_map(|reference| AssetId::new(reference.clone()).ok())
            .collect();

        let Self {
            loader: Some(loader),
            watch,
            sources,
            prefabs,
            prefab_revision,
            profiles,
            ..
        } = self
        else {
            for reference in referenced {
                notes.push(ScriptNote::Failed(format!(
                    "{reference}: the scene has no directory to load scripts from"
                )));
            }
            return notes;
        };

        // A reference an edit removed stops being held, and its source goes with
        // it — otherwise a renamed script would keep running from the text of
        // the file it used to be.
        for released in loader.retain(&wanted) {
            sources.remove(released.as_str());
            if prefabs.remove(released.as_str()).is_some() {
                *prefab_revision += 1;
            }
            profiles.remove(released.as_str());
        }
        if let Some(watch) = watch.as_mut() {
            watch.retain(&wanted);
        }
        for id in &wanted {
            if sources.get(id.as_str()).is_some()
                || prefabs.get(id.as_str()).is_some()
                || profiles.get(id.as_str()).is_some()
            {
                continue;
            }
            if let Err(error) = loader.request(id.clone()) {
                notes.push(ScriptNote::Failed(format!("{id}: {error}")));
            }
        }
        // A reference that is not a valid asset ID will never resolve, and
        // saying so once is better than a script that silently never runs.
        for reference in &referenced {
            if AssetId::new(reference.clone()).is_err() {
                notes.push(ScriptNote::Failed(format!(
                    "{reference}: not a usable asset id"
                )));
            }
        }
        notes
    }

    /// Resolved materials come from the same asynchronous loader as profiles.
    pub fn physics_materials(
        &self,
    ) -> Result<sindri_scene::PhysicsMaterialSources, sindri_scene::PhysicsMaterialError> {
        sindri_scene::PhysicsMaterialSources::from_profiles(
            self.profiles
                .ids()
                .filter_map(|id| self.profiles.get(id).map(|profile| (id, profile))),
        )
    }

    /// Whether any of the project is still on its way.
    ///
    /// Until it has all arrived nothing compiles or runs: a script compiled
    /// before the file declaring `state Game` or a shared function has landed
    /// reports names that are not missing at all, and a spawn before its
    /// prefab has loaded fails.
    pub fn loading(&self) -> bool {
        let Some(loader) = self.loader.as_ref() else {
            return false;
        };
        loader.outstanding() > 0
            || self
                .project_scripts
                .iter()
                .filter_map(|asset| AssetId::new(asset.clone()).ok())
                .any(|id| loader.status(&id).is_none())
    }

    /// Asks for any of the project not asked for yet, such as a file created
    /// since the scene opened.
    fn request_project(&mut self) {
        let Some(loader) = self.loader.as_mut() else {
            return;
        };
        for id in self
            .project_scripts
            .iter()
            .filter_map(|asset| AssetId::new(asset.clone()).ok())
        {
            if loader.status(&id).is_none() {
                // A full queue is not a failure; it is asked again next frame.
                let _ = loader.request(id);
            }
        }
    }

    /// Takes delivery of whatever finished. Called once a frame.
    pub fn poll(&mut self) -> Vec<ScriptNote> {
        let mut notes = self.examine_files();
        self.request_project();
        let Self {
            loader: Some(loader),
            watch,
            sources,
            prefabs,
            prefab_revision,
            profiles,
            ..
        } = self
        else {
            return notes;
        };
        for outcome in loader.poll() {
            match outcome {
                AssetLoadOutcome::Ready(id) => {
                    let Some(text) = loader.get(&id) else {
                        continue;
                    };
                    if let Some(watch) = watch.as_mut() {
                        watch.watch(&id);
                    }
                    let again = sources.get(id.as_str()).is_some()
                        || prefabs.get(id.as_str()).is_some()
                        || profiles.get(id.as_str()).is_some();
                    if is_prefab(id.as_str()) {
                        match PrefabDocument::from_json(text) {
                            Ok(prefab) => {
                                prefabs.insert(id.as_str(), prefab);
                                *prefab_revision += 1;
                            }
                            Err(error) => {
                                notes.push(ScriptNote::Failed(format!("{id}: {error}")));
                                continue;
                            }
                        }
                    } else if is_profile(id.as_str()) {
                        match ProfileDocument::from_json(text) {
                            Ok(profile) => {
                                if let Err(error) =
                                    sindri_scene::physics_material_profile(id.as_str(), &profile)
                                {
                                    notes.push(ScriptNote::Failed(error.to_string()));
                                    continue;
                                }
                                profiles.insert(id.as_str(), profile);
                            }
                            Err(error) => {
                                notes.push(ScriptNote::Failed(format!("{id}: {error}")));
                                continue;
                            }
                        }
                    } else {
                        sources.insert(id.as_str(), text.clone());
                    }
                    notes.push(if again {
                        ScriptNote::Reloaded(format!("Reloaded {id}"))
                    } else {
                        ScriptNote::Loaded(format!("Loaded {id}"))
                    });
                }
                AssetLoadOutcome::Failed(error) => notes.push(ScriptNote::Failed(format!(
                    "{}: {}",
                    error.id(),
                    error.message()
                ))),
            }
        }
        notes
    }

    /// Compiles what the world names, without running anything.
    ///
    /// Called every frame regardless of the transport, so a script that will
    /// not compile says so when the scene opens and the inspector can show what
    /// a script wants authored without anyone pressing Play.
    ///
    /// A source that has not arrived yet is not a failure, and saying it is
    /// puts one permanent error per scripted entity in the console for every
    /// cold open. Loading is asynchronous by design, so between the scene
    /// landing and its scripts arriving every scripted entity is briefly
    /// missing its source; the log keeps what it is told, so the count stayed
    /// up long after the scripts had compiled and run. Opening the companion
    /// game showed twelve errors against a game that was working.
    ///
    /// A source that will never arrive still reports, twice over: the loader
    /// says so when the request fails, and this says so once the asset is out
    /// of flight.
    pub fn compile(
        &mut self,
        world: &World,
        components: &ComponentSchemaRegistry,
    ) -> Vec<ScriptFailure> {
        if self.loading() {
            return Vec::new();
        }
        let mut failures = self.scripts.compile(world, components, &self.sources);
        failures.retain(|failure| match failure {
            ScriptFailure::MissingSource { asset, .. } => !self.is_in_flight(asset),
            _ => true,
        });
        failures
    }

    /// Whether the loader is still working on `asset`, so its absence is a
    /// moment rather than a fault.
    fn is_in_flight(&self, asset: &str) -> bool {
        let Some(loader) = self.loader.as_ref() else {
            return false;
        };
        let Ok(id) = AssetId::new(asset.to_owned()) else {
            return false;
        };
        matches!(
            loader.status(&id),
            Some(AssetStatus::Queued | AssetStatus::Loading)
        )
    }

    /// What scripts asked to hear since this was last asked, oldest first.
    pub fn take_audio_commands(&mut self) -> Vec<sindri_decay::AudioCommand> {
        self.scripts.take_audio_commands()
    }

    /// Moves every script in the world on by one frame.
    pub fn advance(
        &mut self,
        world: &mut World,
        components: &ComponentSchemaRegistry,
        frame: EditorFrame<'_>,
    ) -> ScriptReport {
        // Nothing starts against half a project; see `loading`.
        if self.loading() {
            return ScriptReport::default();
        }
        let EditorFrame {
            input,
            physics,
            physics3d,
            characters,
            screen_ui,
            random,
            saves,
            effects,
            animations,
            sequences,
            delta_seconds,
        } = frame;
        let mut frame = ScriptFrame::new(&self.sources, input, delta_seconds)
            .with_prefabs(&self.prefabs)
            .with_profiles(&self.profiles)
            .with_screen_ui(screen_ui)
            .with_random(random)
            .with_saves(saves)
            .with_effects(effects)
            .with_animations(animations)
            .with_sequences(sequences);
        if let Some(physics) = physics {
            frame = frame.with_physics(physics);
        }
        if let Some(physics) = physics3d {
            frame = frame.with_physics3d(physics);
        }
        if let Some(characters) = characters {
            frame = frame.with_characters(characters);
        }
        // Always timed: the Profiler is the editor's, and a tick's timing is
        // two clock reads beside a script's own work.
        self.scripts.set_measuring(true);
        self.scripts.advance(world, components, frame)
    }

    /// What one script declares it wants authored, for the inspector to draw.
    ///
    /// `None` when the source has not compiled — still loading, or it will not
    /// compile at all — which the panel shows as "waiting" rather than as "no
    /// properties". Those are different, and confusing them would silently hide
    /// an author's fields.
    #[must_use]
    pub fn exports(&self, source: &str, script: &str) -> Option<Vec<ScriptExport>> {
        self.scripts.exports(source, script)
    }

    /// The scripts one loaded source declares, for the panel that offers them.
    #[must_use]
    pub fn declared(&self, source: &str) -> Vec<String> {
        self.scripts.declared(source)
    }

    /// Forgets every running instance, keeping the loaded sources.
    ///
    /// An instance belongs to the world it was started against, so a world that
    /// was reloaded or restored needs new ones. The text does not change when
    /// the world does, and re-fetching it would be a load for nothing.
    /// Whether a prefab has been loaded and can be spawned.
    ///
    /// For anything checking what the editor actually has, rather than what it
    /// meant to fetch: a prefab that was never loaded is an enemy that never
    /// arrives, and the difference is invisible until something spawns one.
    #[must_use]
    /// What a script in this project compiles against: the engine, and every
    /// script in the project as a type another may name.
    pub fn environment(&self) -> &sindri_decay::ScriptEnvironment {
        self.sources.environment()
    }

    /// Every prefab loaded, for whatever else needs to know what one draws.
    pub const fn prefabs(&self) -> &PrefabSources {
        &self.prefabs
    }

    /// Changes whenever a prefab arrives, changes or goes.
    pub const fn prefab_revision(&self) -> u64 {
        self.prefab_revision
    }

    pub fn has_prefab(&self, id: &str) -> bool {
        self.prefabs.get(id).is_some()
    }

    pub fn restart(&mut self) {
        self.scripts.clear();
    }

    fn examine_files(&mut self) -> Vec<ScriptNote> {
        if self.last_examined.elapsed() < WATCH_INTERVAL {
            return Vec::new();
        }
        self.last_examined = Instant::now();
        if let Some(root) = self.watch.as_ref().map(|watch| watch.root().to_path_buf()) {
            self.project_scripts = list_scripts(&root);
        }
        let Self {
            loader: Some(loader),
            watch: Some(watch),
            ..
        } = self
        else {
            return Vec::new();
        };
        let mut notes = Vec::new();
        for id in watch.changed() {
            if let Err(error) = loader.reload(&id) {
                notes.push(ScriptNote::Failed(format!("{id}: {error}")));
            }
        }
        notes
    }
}

/// Every script, prefab and profile under a project's asset root, by the ID a
/// scene uses.
fn list_scripts(root: &Path) -> Vec<String> {
    let source = FileSystemAssetSource::new(root);
    let mut assets = source.assets_with_extension("decay");
    assets.extend(source.assets_with_extension("prefab"));
    assets.extend(source.assets_with_extension("profile"));
    assets
}

fn root_of(scene: Option<&Path>) -> Option<PathBuf> {
    scene.and_then(Path::parent).map(Path::to_path_buf)
}

/// Whether a delivered asset is a prefab rather than a script.
///
/// By suffix, because both arrive as text and the loader has no reason to know
/// the difference. `PREFAB_SUFFIX` is the same convention scenes use, and it is
/// what the editor writes when it makes one.
fn is_prefab(id: &str) -> bool {
    id.ends_with(PREFAB_SUFFIX)
}

fn is_profile(id: &str) -> bool {
    id.ends_with(sindri_core::PROFILE_SUFFIX)
}

pub use sindri_core::PREFAB_SUFFIX;

#[cfg(test)]
mod tests;
