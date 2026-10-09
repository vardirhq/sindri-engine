//! Working out what a project is made of.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sindri_assets::AssetKind;
use sindri_core::{PrefabDocument, SceneDocument, World};
use sindri_decay::{ScriptSources, Scripts};
use sindri_scene::{SceneExtractor, referenced_fonts, referenced_textures};

use crate::prefabs::DiskPrefabs;
use crate::write::ExportError;

/// A project's own description of itself.
#[derive(Debug, Deserialize)]
struct ProjectFile {
    project: ProjectSection,
    #[serde(default)]
    assets: AssetsSection,
    #[serde(default)]
    web: WebSection,
}

/// How a browser build presents itself.
#[derive(Debug, Default, Deserialize)]
struct WebSection {
    #[serde(default)]
    splash: Option<SplashSection>,
}

/// The project's own brand, shown after the Sindri mark while the game loads.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SplashSection {
    /// An image, relative to the project: a logo or a title card.
    image: Option<String>,
    /// Text under the image, or in its place.
    title: Option<String>,
    /// A smaller line under the title.
    caption: Option<String>,
    /// The page behind it, as `#rrggbb`.
    background: Option<String>,
    /// How long it shows at least, even when the game is ready sooner.
    #[serde(default = "default_splash_seconds")]
    seconds: f64,
}

const fn default_splash_seconds() -> f64 {
    1.5
}

/// What a project ships that its scene does not mention.
///
/// Almost everything is found by walking the scene, which is what makes an
/// export impossible to forget to update. This is the exception, and it exists
/// because a script can name a clip at run time — `Audio.play("pickup.wav")` is
/// a string inside a program, and no walk of a scene can see it. Guessing by
/// scanning script text for anything that looks like a path would ship whatever
/// a comment mentioned and miss whatever was built from a variable.
#[derive(Debug, Default, Deserialize)]
struct AssetsSection {
    #[serde(default)]
    include: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectSection {
    name: String,
    main_scene: String,
    /// Scenes the project can reach besides the one it opens on.
    ///
    /// Declared rather than discovered, which is the one place this exporter
    /// takes a list. A scene reached by `Scene.go("house")` is a string inside
    /// a program, and the comment below about prefabs says why that cannot be
    /// found by looking: a field declared `String` is text however much it
    /// resembles a path. Naming them in `[assets] include` would ship the scene
    /// files and none of their textures, scripts or prefabs, which is the kind
    /// of export that looks complete.
    #[serde(default)]
    scenes: Vec<String>,
}

/// One file the export will ship, and what it is.
#[derive(Clone, Debug)]
pub struct GatheredAsset {
    /// What the scene calls it, which is what a host asks for.
    pub id: String,
    pub kind: AssetKind,
    pub bytes: Vec<u8>,
}

/// Everything a project ships, worked out from its scenes.
#[derive(Debug)]
pub struct ProjectExport {
    pub name: String,
    pub assets: Vec<GatheredAsset>,
    /// The scene a host opens on, by the ID it is shipped under.
    ///
    /// Held rather than looked for: with more than one scene shipping, "the
    /// first scene asset" is whichever one happened to be gathered first.
    main_scene: String,
    /// The project's brand for the browser's loading screen, if it has one.
    pub splash: Option<crate::page::Splash>,
}

/// A splash as the page shows it, with its image read and its settings
/// checked: a color that is not one, or a time that is not a time, is a
/// mistake to report now rather than a page that quietly ignores it.
fn read_splash(project: &Path, splash: SplashSection) -> Result<crate::page::Splash, ExportError> {
    let bad = |what: String| ExportError::Project(format!("[web.splash] {what}"));
    if !(0.0..=10.0).contains(&splash.seconds) {
        return Err(bad(format!(
            "seconds is {}, and a splash shows for 0 to 10 seconds",
            splash.seconds
        )));
    }
    if let Some(background) = &splash.background {
        let digits = background.strip_prefix('#').unwrap_or_default();
        if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(bad(format!(
                "background {background:?} is not a `#rrggbb` color"
            )));
        }
    }
    let image = match &splash.image {
        Some(relative) => {
            let path = project.join(relative);
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .filter(|extension| {
                    matches!(
                        extension.as_str(),
                        "png" | "jpg" | "jpeg" | "webp" | "svg" | "gif"
                    )
                })
                .ok_or_else(|| {
                    bad(format!(
                        "image {relative:?} is not a png, jpg, webp, svg or gif"
                    ))
                })?;
            let bytes =
                std::fs::read(&path).map_err(|error| ExportError::unreadable(&path, &error))?;
            Some((extension, bytes))
        }
        None => None,
    };
    if image.is_none() && splash.title.is_none() {
        return Err(bad("needs an image, a title, or both".to_owned()));
    }
    Ok(crate::page::Splash {
        image,
        title: splash.title,
        caption: splash.caption,
        background: splash.background,
        seconds: splash.seconds,
    })
}

impl ProjectExport {
    /// Reads a project directory and works out what it is made of.
    ///
    /// Nothing here is configured. A texture ships because a component names
    /// it, a font because a text element does, a script because an entity runs
    /// one — so an asset that stopped being used stops being carried, and one
    /// that started being used cannot be forgotten.
    pub fn gather(project: &Path) -> Result<Self, ExportError> {
        let text = std::fs::read_to_string(project.join("sindri.toml"))
            .map_err(|error| ExportError::unreadable(&project.join("sindri.toml"), &error))?;
        let file: ProjectFile =
            toml::from_str(&text).map_err(|error| ExportError::Project(error.to_string()))?;

        let extractor = SceneExtractor::new().map_err(|error| {
            ExportError::Project(format!("the components do not register: {error}"))
        })?;
        // `sindri.script` is not a builtin: scripting is a layer above the
        // scene, and a host registers it. An export that did not register it
        // would carry a game with no code in it and look like it had worked.
        let mut components = extractor.components().clone();
        components
            .register::<sindri_decay::ScriptComponent>("Script")
            .map_err(|error| ExportError::Project(format!("sindri.script: {error}")))?;
        // Every scene the project declares, the one it opens on first. Each is
        // walked exactly as the main scene is: a second scene whose textures
        // did not ship would be a door that opened onto nothing.
        let mut assets = Vec::new();
        let mut worlds = Vec::new();
        let mut loaded: BTreeSet<String> = BTreeSet::new();
        let mut placed = DiskPrefabs::default();
        for path in std::iter::once(&file.project.main_scene).chain(&file.project.scenes) {
            // A project that lists its own main scene is not an error; it is
            // someone being explicit, and shipping it twice would be.
            if !loaded.insert(path.clone()) {
                continue;
            }
            let bytes = read(&project.join(path))?;
            let document: SceneDocument = serde_json::from_slice(&bytes).map_err(|error| {
                ExportError::Project(format!("scene {path} does not read: {error}"))
            })?;
            placed.read_placed_by(project, &document.entities)?;
            let mut world = World::from_scene_with(&document, &placed)
                .map_err(|error| {
                    ExportError::Project(format!("scene {path} does not load: {error}"))
                })?
                .world;
            everything_on(&mut world);
            worlds.push(world);
            assets.push(GatheredAsset {
                // A scene is named by its file, so two scenes in one project do
                // not collide, and so the host asks for the one it means.
                id: leaf(path),
                kind: AssetKind::Scene,
                bytes,
            });
        }

        // Ordered and de-duplicated, because two entities naming one texture is
        // one download.
        let mut wanted: BTreeMap<String, AssetKind> = BTreeMap::new();
        let mut physics_profiles = BTreeSet::new();
        // The prefabs a scene places ship as prefabs: a host makes each
        // instance from its prefab when the scene loads.
        for id in placed.ids() {
            wanted.insert(id.to_owned(), AssetKind::Prefab);
        }

        // A queue of documents rather than one, because a prefab is a document
        // too and everything it names has to ship for the same reason the
        // scene's does. The scene is the first; a prefab a script can spawn is
        // the next; a prefab spawned by *that* prefab's script is the one
        // after. A game whose every enemy is a prefab would otherwise export
        // as an empty world that looked complete.
        let mut pending = worlds;
        let mut sources = ScriptSources::new();
        let mut scripts = Scripts::new();
        let mut walked: BTreeSet<String> = BTreeSet::new();
        // Every script ships, not only those a scene names: any script may
        // name any other by type (`Bolt.on(hit)`), so each one compiles
        // against all of them, in the build as much as in the editor.
        let assets_root = if project.join("assets").is_dir() {
            project.join("assets")
        } else {
            project.to_path_buf()
        };
        for source in scripts_under(&assets_root) {
            if let Ok(text) = std::fs::read_to_string(resolve(project, &source)) {
                wanted.insert(source.clone(), AssetKind::Script);
                sources.insert(source, text);
            }
        }
        while let Some(world) = pending.pop() {
            super::models::references(&world, &mut wanted);
            for reference in referenced_textures(&world) {
                wanted.insert(reference, AssetKind::Texture);
            }
            for tile_set in sindri_scene::referenced_tile_sets(&world) {
                wanted.insert(tile_set, AssetKind::TileSet);
            }
            for font in referenced_fonts(&world) {
                wanted.insert(font, AssetKind::Font);
            }
            // Audio is named by a component like anything else, and was the one
            // kind with no walker in the engine — a scene's music would have
            // been left behind by an export that looked complete.
            for (_, data) in world.entities() {
                if let Some(payload) = data.components.get("sindri.audio.source")
                    && let Some(clip) = payload.get("clip").and_then(serde_json::Value::as_str)
                {
                    wanted.insert(clip.to_owned(), AssetKind::Audio);
                }
            }
            // A sequence's cues name theirs too, in the scene where a walk
            // can see them rather than in a script where it cannot.
            for clip in sindri_scene::referenced_sounds(&world) {
                wanted.insert(clip, AssetKind::Audio);
            }

            // The scripts are read before they are asked about, because which
            // prefabs a world can spawn is the *declared type* of a script's
            // exported fields — which is not known until the script compiles.
            // That is also why a prefab cannot be found by looking for strings
            // that resemble paths: a field declared `String` is text however
            // much it looks like one.
            for source in sindri_decay::referenced_sources(&world, &components) {
                wanted.insert(source.clone(), AssetKind::Script);
                if sources.get(&source).is_none()
                    && let Ok(text) = std::fs::read_to_string(resolve(project, &source))
                {
                    sources.insert(source, text);
                }
            }
            // A script that does not compile is not an export failure here: it
            // is reported where scripts are reported, and an export that
            // refused would make a broken script un-shippable to the person
            // trying to debug it in a build.
            let _ = scripts.compile(&world, &components, &sources);

            for id in scripts.referenced_prefabs(&world, &components) {
                if !walked.insert(id.clone()) {
                    continue;
                }
                wanted.insert(id.clone(), AssetKind::Prefab);
                let text = std::fs::read_to_string(resolve(project, &id))
                    .map_err(|error| ExportError::unreadable(&resolve(project, &id), &error))?;
                let prefab = PrefabDocument::from_json(&text)
                    .map_err(|error| ExportError::Project(format!("{id}: {error}")))?;
                // The prefabs nested in it ship too, and are made with it.
                placed.read_placed_by(project, &prefab.entities)?;
                for nested in placed.ids() {
                    wanted.insert(nested.to_owned(), AssetKind::Prefab);
                }
                // Spawned into a world of its own so the same walkers answer
                // for it. A prefab is a fragment of a scene, so what finds a
                // scene's textures finds a prefab's.
                let mut world = World::default();
                world
                    .spawn_prefab_from(&prefab, &placed)
                    .map_err(|error| ExportError::Project(format!("{id}: {error}")))?;
                everything_on(&mut world);
                pending.push(world);
            }
            for id in sindri_scene::referenced_physics_materials(&world, &components)
                .map_err(|error| ExportError::Project(error.to_string()))?
            {
                physics_profiles.insert(id.clone());
                wanted.insert(id, AssetKind::Profile);
            }
            for id in scripts.referenced_profiles(&world, &components) {
                wanted.insert(id, AssetKind::Profile);
            }
        }

        // Tile-set faces name textures indirectly through a reusable asset.
        // Resolve those references before sheets, so their named sprites bring
        // the sidecars that slice them just like scene sprites do.
        let tile_sets = wanted
            .iter()
            .filter(|(_, kind)| **kind == AssetKind::TileSet)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in tile_sets.into_iter().filter(|id| !engine_provided(id)) {
            let text = std::fs::read_to_string(resolve(project, &id))
                .map_err(|error| ExportError::unreadable(&resolve(project, &id), &error))?;
            let tile_set = sindri_core::TileSetDocument::from_json(&text)
                .map_err(|error| ExportError::Project(format!("{id}: {error}")))?;
            for texture in sindri_scene::tile_set_textures(&tile_set) {
                wanted.insert(texture, AssetKind::Texture);
            }
        }

        // A sheet is not named by anything: it is found beside its texture, and
        // a texture cut into sprites that shipped without one would draw whole
        // images where frames should be.
        let sheets: Vec<String> = wanted
            .iter()
            .filter(|(_, kind)| **kind == AssetKind::Texture)
            .filter_map(|(id, _)| {
                sindri_core::AssetId::new(id)
                    .ok()
                    .and_then(|texture| sindri_core::sheet_id_for(&texture))
            })
            .map(|id| id.as_str().to_owned())
            // Resolved the same way every other asset is: a project that keeps
            // its art outside `assets/` and names it from the root still has
            // its sheets found. Looking only under `assets/` shipped those
            // textures with no slices, so an animated sprite drew its whole
            // sheet squeezed into one quad.
            .filter(|id| resolve(project, id).exists())
            .collect();
        for id in sheets {
            wanted.insert(id, AssetKind::Sheet);
        }

        // What the scene cannot name. A listed Weave entry is special only in
        // one useful way: its `@use` graph is declarative, so the exporter can
        // follow it instead of making authors mirror imports in this list.
        for id in &file.assets.include {
            wanted.insert(id.clone(), AssetKind::for_id(id));
            if id.ends_with(".weave") {
                gather_weave_dependencies(project, id, &mut wanted)?;
            }
        }

        for (id, kind) in wanted {
            // A procedural texture or a built-in block set is the engine's own
            // and has no file in the project. It is named like an asset and is
            // not one, so shipping it would mean failing to find it.
            if engine_provided(&id) {
                continue;
            }
            let path = resolve(project, &id);
            let bytes = read(&path)?;
            if kind == AssetKind::Profile {
                validate_profile(&id, &bytes, physics_profiles.contains(&id))?;
            }
            assets.push(GatheredAsset {
                id: id.clone(),
                kind,
                bytes,
            });
        }

        super::models::validate(&assets)?;
        let splash = file
            .web
            .splash
            .map(|splash| read_splash(project, splash))
            .transpose()?;
        Ok(Self {
            name: file.project.name,
            assets,
            main_scene: leaf(&file.project.main_scene),
            splash,
        })
    }

    /// The scene a host opens on, by the ID it ships under.
    #[must_use]
    pub fn scene_id(&self) -> Option<&str> {
        self.assets
            .iter()
            .find(|asset| asset.kind == AssetKind::Scene && asset.id == self.main_scene)
            .map(|asset| asset.id.as_str())
    }

    /// Every scene the export ships, in the order they were gathered, the one
    /// it opens on first.
    pub fn scene_ids(&self) -> impl Iterator<Item = &str> {
        self.assets
            .iter()
            .filter(|asset| asset.kind == AssetKind::Scene)
            .map(|asset| asset.id.as_str())
    }
}

fn gather_weave_dependencies(
    project: &Path,
    entry: &str,
    wanted: &mut BTreeMap<String, AssetKind>,
) -> Result<(), ExportError> {
    let mut pending = vec![entry.to_owned()];
    let mut walked = BTreeSet::new();

    while let Some(id) = pending.pop() {
        if !walked.insert(id.clone()) {
            continue;
        }
        wanted.insert(id.clone(), AssetKind::for_id(&id));

        let path = resolve(project, &id);
        let source = std::fs::read_to_string(&path)
            .map_err(|error| ExportError::unreadable(&path, &error))?;
        let imports = weave::imports(&source)
            .map_err(|error| ExportError::Project(format!("{id}: {error}")))?;
        for reference in imports {
            let dependency = weave::resolve_import(&id, &reference)
                .map_err(|error| ExportError::Project(error.to_string()))?;
            if !walked.contains(&dependency) {
                pending.push(dependency);
            }
        }
    }

    Ok(())
}

/// Switches on every entity in a copy of a world about to be walked.
///
/// The walks are the runtime's, and the runtime's walks are active-only —
/// correctly, because nothing switched off is drawn or stepped. An export is
/// the other question: not what is running, but what this project could ever
/// need. A scene's menus are switched off until something shows them, so an
/// export that asked the runtime's question shipped a game with no pause
/// screen and no way to notice until someone pressed Escape in a build.
///
/// Done to the export's own throwaway world, so nothing that is saved or
/// played changes.
fn everything_on(world: &mut World) {
    let entities: Vec<_> = world.entities().map(|(entity, _)| entity).collect();
    for entity in entities {
        if let Some(data) = world.get_mut(entity) {
            data.disabled = false;
        }
    }
}

/// Where a project keeps the file an asset ID names.
///
/// Under `assets/` normally, and at the project root for a project that lays
/// itself out differently — the same two places the byte-reading loop looks,
/// because a script found one way and read the other would be found and then
/// not shipped.
pub(crate) fn resolve(project: &Path, id: &str) -> PathBuf {
    let path = project.join("assets").join(id);
    if path.exists() {
        path
    } else {
        project.join(id)
    }
}

fn read(path: &Path) -> Result<Vec<u8>, ExportError> {
    std::fs::read(path).map_err(|error| ExportError::unreadable(path, &error))
}

/// The part of a path a host names an asset by.
fn leaf(path: &str) -> String {
    PathBuf::from(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned())
}

/// Whether a reference names something the engine provides rather than a file:
/// `procedural:checkerboard`, `builtin:blocks` and whatever it names. A colon
/// cannot appear in an asset ID, which is what keeps the two kinds apart.
fn engine_provided(reference: &str) -> bool {
    reference.contains(':')
}

/// Every `.decay` file under an asset root, by the ID a scene would name it.
///
/// Walked here rather than through `sindri-assets`, whose filesystem source is
/// native-only: this crate is checked for the browser target too.
fn scripts_under(root: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, depth: usize, into: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            if path.is_dir() {
                if depth < 6 {
                    walk(root, &path, depth + 1, into);
                }
            } else if path.extension().is_some_and(|found| found == "decay")
                && let Ok(relative) = path.strip_prefix(root)
            {
                into.push(
                    relative
                        .components()
                        .map(|part| part.as_os_str().to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
        }
    }
    let mut found = Vec::new();
    walk(root, root, 0, &mut found);
    found.sort();
    found
}

fn validate_profile(id: &str, bytes: &[u8], physics: bool) -> Result<(), ExportError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ExportError::Project(format!("{id}: {error}")))?;
    let profile = sindri_core::ProfileDocument::from_json(text)
        .map_err(|error| ExportError::Project(format!("{id}: {error}")))?;
    let material = sindri_scene::physics_material_profile(id, &profile)
        .map_err(|error| ExportError::Project(error.to_string()))?;
    if physics && material.is_none() {
        return Err(ExportError::Project(format!(
            "{id}: physics material reference needs type physics_material"
        )));
    }
    Ok(())
}
