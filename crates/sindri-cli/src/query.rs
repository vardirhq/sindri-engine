//! Questions about a project, answered without reading every file.
//!
//! A scene is a long JSON file, and an agent that reads one whole to find an
//! entity spends most of its context on what it did not need. These answer
//! the questions it is reading for: what the project holds
//! (`project overview`), how a scene is put together (`scene outline`), where
//! the entities with some component, name, script or prefab are
//! (`entity find`), one entity in full (`entity show`), and what refers to a
//! file before it is renamed or removed (`asset uses`).
//!
//! They read the files as they are, leniently: a scene that does not pass
//! `project check` is still outlined as far as it reads, because the agent
//! asking is often the one fixing it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Serialize;
use serde_json::Value;
use sindri_core::{SceneDocument, SceneEntity, split_part};
use sindri_runtime::project::{assets_root, files_under, manifest};

use crate::report::SCHEMA_VERSION;

mod outline;
pub mod script;
pub mod style;

pub use outline::outline;

/// A project's documents, read once.
pub struct Project {
    pub root: String,
    pub main_scene: Option<String>,
    /// Scenes and prefabs by asset ID, with the ones that would not read.
    pub documents: BTreeMap<String, Result<SceneDocument, String>>,
    pub scripts: BTreeMap<String, Vec<String>>,
    pub stylesheets: Vec<String>,
    /// The text of every script and stylesheet, which can name files too.
    pub sources: BTreeMap<String, String>,
    /// Every other file under the assets, by asset ID.
    pub files: BTreeSet<String>,
}

impl Project {
    /// Reads every scene, prefab and script under the project's assets.
    #[must_use]
    pub fn read(project: &Path) -> Self {
        let assets = assets_root(project);
        let text = |suffix: &str| -> BTreeMap<String, String> {
            files_under(&assets, suffix)
                .into_iter()
                .map(|(id, bytes)| (id, String::from_utf8_lossy(&bytes).into_owned()))
                .collect()
        };
        let mut documents = BTreeMap::new();
        for suffix in [".scene", ".prefab"] {
            for (id, json) in text(suffix) {
                let read =
                    serde_json::from_str::<SceneDocument>(&json).map_err(|error| error.to_string());
                documents.insert(id, read);
            }
        }
        let sources = text(".decay");
        let mut environment = sindri_decay::ScriptSources::new();
        for (id, source) in &sources {
            environment.insert(id.clone(), source.clone());
        }
        let environment = environment.environment().clone();
        let scripts = sources
            .iter()
            .map(|(id, source)| {
                let checked = sindri_decay::check_source_in(source, &environment);
                (id.clone(), checked.declared)
            })
            .collect();
        let files = names_under(&assets)
            .into_iter()
            .filter(|id| {
                ![".scene", ".prefab", ".decay", ".weave"]
                    .iter()
                    .any(|suffix| id.to_ascii_lowercase().ends_with(suffix))
            })
            .collect();
        let stylesheets = text(".weave");
        let mut texts = sources;
        texts.extend(stylesheets.clone());
        Self {
            root: project.display().to_string(),
            main_scene: manifest(project).ok().map(|(scene, _, _)| scene),
            documents,
            scripts,
            stylesheets: stylesheets.into_keys().collect(),
            sources: texts,
            files,
        }
    }

    /// The scene or prefab called `name`: its asset ID, or its file name with
    /// or without `.scene`.
    pub fn document(&self, name: &str) -> Result<(&str, &SceneDocument), String> {
        let found = self.documents.iter().find(|(id, _)| {
            let file = id.rsplit('/').next().unwrap_or(id);
            id.as_str() == name || file == name || file.strip_suffix(".scene") == Some(name)
        });
        match found {
            Some((id, Ok(document))) => Ok((id, document)),
            Some((id, Err(error))) => Err(format!("{id} does not read: {error}")),
            None => Err(format!(
                "no scene or prefab {name}; there are {}",
                self.documents
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }

    /// Every entity in every document that reads, with its document.
    fn entities(&self) -> impl Iterator<Item = (&str, &SceneEntity)> {
        self.documents
            .iter()
            .filter_map(|(id, read)| read.as_ref().ok().map(|doc| (id, doc)))
            .flat_map(|(id, document)| {
                document
                    .entities
                    .iter()
                    .map(move |entity| (id.as_str(), entity))
            })
    }
}

/// Every file's asset ID under `root`, without reading any of them: a
/// project's pictures and sounds are most of its bytes.
fn names_under(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(folder) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    found.sort();
    found
}

/// `sindri project overview`.
#[derive(Serialize)]
pub struct Overview {
    pub schema_version: u32,
    pub project: String,
    pub main_scene: Option<String>,
    pub scenes: Vec<DocumentSummary>,
    pub prefabs: Vec<DocumentSummary>,
    /// Each Decay file and the scripts it declares.
    pub scripts: BTreeMap<String, Vec<String>>,
    pub stylesheets: Vec<String>,
    /// How many entities carry each component, across every document.
    pub components: BTreeMap<String, usize>,
    /// Other files under the assets, counted by extension.
    pub files: BTreeMap<String, usize>,
}

#[derive(Serialize)]
pub struct DocumentSummary {
    pub id: String,
    pub entities: usize,
    /// Why it does not read, when it does not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[must_use]
pub fn overview(project: &Project) -> Overview {
    let mut scenes = Vec::new();
    let mut prefabs = Vec::new();
    for (id, read) in &project.documents {
        let summary = DocumentSummary {
            id: id.clone(),
            entities: read.as_ref().map_or(0, |document| document.entities.len()),
            error: read.as_ref().err().cloned(),
        };
        if id.ends_with(".prefab") {
            prefabs.push(summary);
        } else {
            scenes.push(summary);
        }
    }
    let mut components = BTreeMap::new();
    for (_, entity) in project.entities() {
        for key in entity.components.keys() {
            *components.entry(split_part(key).0.to_owned()).or_insert(0) += 1;
        }
    }
    let mut files = BTreeMap::new();
    for id in &project.files {
        let extension = Path::new(id).extension().map_or_else(
            || "(none)".to_owned(),
            |extension| extension.to_string_lossy().to_ascii_lowercase(),
        );
        *files.entry(extension).or_insert(0) += 1;
    }
    Overview {
        schema_version: SCHEMA_VERSION,
        project: project.root.clone(),
        main_scene: project.main_scene.clone(),
        scenes,
        prefabs,
        scripts: project.scripts.clone(),
        stylesheets: project.stylesheets.clone(),
        components,
        files,
    }
}

impl Overview {
    #[must_use]
    pub fn human(&self) -> String {
        use std::fmt::Write as _;
        let mut out = format!(
            "{} (main scene {})\n",
            self.project,
            self.main_scene.as_deref().unwrap_or("none")
        );
        for (title, documents) in [("scenes", &self.scenes), ("prefabs", &self.prefabs)] {
            let _ = writeln!(out, "{title}:");
            for document in documents {
                match &document.error {
                    Some(error) => {
                        let _ = writeln!(out, "  {} (does not read: {error})", document.id);
                    }
                    None => {
                        let _ = writeln!(out, "  {} ({} entities)", document.id, document.entities);
                    }
                }
            }
        }
        let _ = writeln!(out, "scripts:");
        for (file, declared) in &self.scripts {
            let _ = writeln!(out, "  {file}: {}", declared.join(", "));
        }
        if !self.stylesheets.is_empty() {
            let _ = writeln!(out, "stylesheets: {}", self.stylesheets.join(", "));
        }
        let _ = writeln!(
            out,
            "components: {}",
            self.components
                .iter()
                .map(|(name, count)| format!("{name} ×{count}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let _ = writeln!(
            out,
            "other files: {}",
            self.files
                .iter()
                .map(|(extension, count)| format!("{count} .{extension}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        out
    }
}

/// What `entity find` matches; every given filter must hold.
#[derive(Default)]
pub struct Filter<'a> {
    /// A component type, matching its named parts too.
    pub component: Option<&'a str>,
    /// Part of the entity's name or ID, ignoring case.
    pub name: Option<&'a str>,
    /// A script it runs, by container name or source file.
    pub script: Option<&'a str>,
    /// A prefab it is an instance of, by file name or asset ID.
    pub prefab: Option<&'a str>,
    /// One scene or prefab to look in.
    pub scene: Option<&'a str>,
}

/// One entity `entity find` found.
#[derive(Serialize)]
pub struct Found {
    pub scene: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub components: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefab: Option<String>,
}

#[derive(Serialize)]
pub struct Finding {
    pub schema_version: u32,
    pub found: Vec<Found>,
    /// More matched than were listed.
    pub more: usize,
}

/// The most entities `entity find` lists, so a broad filter cannot flood a
/// context; `more` says how many it left out.
const MOST_FOUND: usize = 200;

#[must_use]
pub fn find(project: &Project, filter: &Filter<'_>) -> Finding {
    let holds = |needle: Option<&str>, test: &dyn Fn(&str) -> bool| needle.is_none_or(test);
    let mut found = Vec::new();
    let mut more = 0;
    for (scene, entity) in project.entities() {
        let in_scene = holds(filter.scene, &|wanted| {
            project.document(wanted).is_ok_and(|(id, _)| id == scene)
        });
        let component = holds(filter.component, &|wanted| {
            entity
                .components
                .keys()
                .any(|key| key == wanted || split_part(key).0 == wanted)
        });
        let name = holds(filter.name, &|wanted| {
            let wanted = wanted.to_lowercase();
            entity.id.as_str().to_lowercase().contains(&wanted)
                || entity
                    .name
                    .as_deref()
                    .is_some_and(|name| name.to_lowercase().contains(&wanted))
        });
        let script = holds(filter.script, &|wanted| {
            entity.components.iter().any(|(key, payload)| {
                split_part(key).0 == "sindri.script"
                    && ["script", "source"].iter().any(|field| {
                        payload
                            .get(field)
                            .and_then(Value::as_str)
                            .is_some_and(|value| {
                                value == wanted || value.rsplit('/').next() == Some(wanted)
                            })
                    })
            })
        });
        let prefab = holds(filter.prefab, &|wanted| {
            entity.prefab.as_ref().is_some_and(|instance| {
                instance.source == wanted || instance.source.rsplit('/').next() == Some(wanted)
            })
        });
        if !(in_scene && component && name && script && prefab) {
            continue;
        }
        if found.len() == MOST_FOUND {
            more += 1;
            continue;
        }
        found.push(Found {
            scene: scene.to_owned(),
            id: entity.id.as_str().to_owned(),
            name: entity.name.clone(),
            parent: entity
                .parent
                .as_ref()
                .map(|parent| parent.as_str().to_owned()),
            components: entity.components.keys().cloned().collect(),
            prefab: entity
                .prefab
                .as_ref()
                .map(|instance| instance.source.clone()),
        });
    }
    Finding {
        schema_version: SCHEMA_VERSION,
        found,
        more,
    }
}

impl Finding {
    #[must_use]
    pub fn human(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        for found in &self.found {
            let named = found
                .name
                .as_deref()
                .map(|name| format!(" \"{name}\""))
                .unwrap_or_default();
            let prefab = found
                .prefab
                .as_deref()
                .map(|prefab| format!(" (instance of {prefab})"))
                .unwrap_or_default();
            let _ = writeln!(
                out,
                "{}: {}{named}{prefab}: {}",
                found.scene,
                found.id,
                found.components.join(", ")
            );
        }
        let _ = match self.more {
            0 => writeln!(out, "{} found", self.found.len()),
            more => writeln!(
                out,
                "{} found, and {more} more not listed",
                self.found.len()
            ),
        };
        out
    }
}

/// `entity show`: one entity as written, with where it sits.
#[derive(Serialize)]
pub struct Shown {
    pub schema_version: u32,
    pub scene: String,
    pub entity: SceneEntity,
    pub children: Vec<String>,
}

/// The entity `id`, in `scene` or wherever it is when that is one place.
///
/// # Errors
/// No such entity, or one ID in several documents and no scene named.
pub fn show(project: &Project, id: &str, scene: Option<&str>) -> Result<Shown, String> {
    let scene_id = scene
        .map(|scene| project.document(scene).map(|(id, _)| id.to_owned()))
        .transpose()?;
    let matches: Vec<(&str, &SceneEntity)> = project
        .entities()
        .filter(|(at, entity)| {
            entity.id.as_str() == id && scene_id.as_deref().is_none_or(|wanted| wanted == *at)
        })
        .collect();
    let (at, entity) = match matches.as_slice() {
        [one] => *one,
        [] => {
            return Err(format!(
                "no entity {id}; `sindri entity find --name {id}` looks wider"
            ));
        }
        several => {
            return Err(format!(
                "{id} is in {}; name one with --scene",
                several
                    .iter()
                    .map(|(at, _)| *at)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    };
    let children = project
        .entities()
        .filter(|(here, child)| {
            *here == at
                && child
                    .parent
                    .as_ref()
                    .is_some_and(|parent| parent.as_str() == id)
        })
        .map(|(_, child)| child.id.as_str().to_owned())
        .collect();
    Ok(Shown {
        schema_version: SCHEMA_VERSION,
        scene: at.to_owned(),
        entity: entity.clone(),
        children,
    })
}

/// `asset uses`: everything that names a file.
#[derive(Serialize)]
pub struct Usage {
    pub schema_version: u32,
    pub asset: String,
    pub uses: Vec<Use>,
}

#[derive(Serialize)]
pub struct Use {
    /// The scene, prefab or script that names it.
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    /// Where in the entity: a component's field path, or `prefab`.
    pub at: String,
}

/// What names `asset`, by asset ID or by file name: component fields,
/// prefab instances, and, for a script, the scripts that `use` it.
#[must_use]
pub fn uses(project: &Project, asset: &str) -> Usage {
    let asset = asset.strip_prefix("assets/").unwrap_or(asset);
    // A picture is also named by a region of it: `textures/decor.png#tufts`.
    let names = |value: &str| {
        let value = value.split('#').next().unwrap_or(value);
        value == asset || value.strip_prefix("assets/") == Some(asset)
    };
    let mut uses = Vec::new();
    for (file, entity) in project.entities() {
        if entity
            .prefab
            .as_ref()
            .is_some_and(|instance| names(&instance.source))
        {
            uses.push(Use {
                file: file.to_owned(),
                entity: Some(entity.id.as_str().to_owned()),
                at: "prefab".to_owned(),
            });
        }
        for (key, payload) in &entity.components {
            let mut paths = Vec::new();
            strings_naming(payload, key, &names, &mut paths);
            uses.extend(paths.into_iter().map(|at| Use {
                file: file.to_owned(),
                entity: Some(entity.id.as_str().to_owned()),
                at,
            }));
        }
    }
    // A script or stylesheet naming it in a string, line by line.
    let file_name = asset.rsplit('/').next().unwrap_or(asset);
    for (file, text) in &project.sources {
        for (index, line) in text.lines().enumerate() {
            let quoted = ['"', '\'']
                .iter()
                .any(|quote| line.contains(&format!("{quote}{asset}{quote}")));
            if quoted || (file_name != asset && line.contains(&format!("\"{file_name}\""))) {
                uses.push(Use {
                    file: file.clone(),
                    entity: None,
                    at: format!("line {}", index + 1),
                });
            }
        }
    }
    Usage {
        schema_version: SCHEMA_VERSION,
        asset: asset.to_owned(),
        uses,
    }
}

/// The paths under `value` of every string `names` accepts.
fn strings_naming(value: &Value, at: &str, names: &dyn Fn(&str) -> bool, out: &mut Vec<String>) {
    match value {
        Value::String(text) if names(text) => out.push(at.to_owned()),
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                strings_naming(item, &format!("{at}.{index}"), names, out);
            }
        }
        Value::Object(fields) => {
            for (name, field) in fields {
                strings_naming(field, &format!("{at}.{name}"), names, out);
            }
        }
        _ => {}
    }
}

impl Usage {
    #[must_use]
    pub fn human(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        for found in &self.uses {
            let entity = found
                .entity
                .as_deref()
                .map(|entity| format!(" {entity}"))
                .unwrap_or_default();
            let _ = writeln!(out, "{}:{entity} at {}", found.file, found.at);
        }
        let _ = writeln!(out, "{} uses of {}", self.uses.len(), self.asset);
        out
    }
}

#[cfg(test)]
mod tests;
