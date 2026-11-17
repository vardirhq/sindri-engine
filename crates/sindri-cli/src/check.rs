//! `sindri project check`: everything a project is made of, read the way a run
//! reads it, every problem reported rather than the first.
//!
//! A run stops at the first scene that will not load; an agent or a person
//! fixing a project wants the whole list. So each file is read on its own and
//! what it says is collected, with the code the crate that refused it gives.
//! See `docs/agent-native-direction.md`, which this is step one of.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;
use sindri_core::{
    ComponentRegistryError, ComponentSchemaRegistry, PrefabDocument, SceneDocument, SceneEntity,
    UnknownComponentPolicy, split_part,
};
use sindri_decay::{CheckPhase, ScriptSources, check_source_in};
use sindri_runtime::project::{assets_root, files_under, manifest, scene_extractor};

use crate::report::{CheckReport, Checked, Diagnostic, SCHEMA_VERSION, Severity};

const SCRIPT: &str = "sindri.script";
const SPRITE: &str = "sindri.sprite";
/// Weave's styling metadata, kept on entities beside their components but
/// deliberately not one: Weave reads it, not the engine.
const WEAVE_STYLE: &str = "weave.style";

/// Checks the project at `project`.
#[must_use]
pub fn check(project: &Path) -> CheckReport {
    let mut diagnostics = Vec::new();
    let assets = assets_root(project);
    let prefix = assets
        .strip_prefix(project)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .filter(|relative| !relative.is_empty())
        .map(|relative| format!("{relative}/"))
        .unwrap_or_default();
    let shown = |id: &str| format!("{prefix}{id}");

    let main_scene = check_manifest(project, &assets, &mut diagnostics);
    let registry = match scene_extractor() {
        Ok(extractor) => extractor.components().clone(),
        Err(error) => {
            diagnostics.push(Diagnostic::error("ENGINE_REGISTRY", "", error));
            return finish(project, Checked::default(), diagnostics);
        }
    };

    let scripts = text_files(&assets, ".decay");
    let declared = check_scripts(&scripts, &shown, &mut diagnostics);
    let images: BTreeSet<String> = [".png", ".jpg", ".jpeg"]
        .iter()
        .flat_map(|suffix| files_under(&assets, suffix).into_keys())
        .collect();
    let fields = component_fields();
    let refs = References {
        registry: &registry,
        fields: &fields,
        declared: &declared,
        images: &images,
    };

    let scenes = text_files(&assets, ".scene");
    for (id, text) in &scenes {
        check_scene(&shown(id), text, &refs, &mut diagnostics);
    }
    if let Some(main) = main_scene
        && !scenes.contains_key(&main)
    {
        diagnostics.push(Diagnostic::error(
            "PROJECT_MISSING_MAIN_SCENE",
            "sindri.toml",
            format!("the main scene {} is not under the assets", shown(&main)),
        ));
    }
    let prefabs = text_files(&assets, ".prefab");
    for (id, text) in &prefabs {
        check_prefab(&shown(id), text, &refs, &mut diagnostics);
    }
    let stylesheets = text_files(&assets, ".weave");
    check_stylesheets(&stylesheets, &shown, &mut diagnostics);

    let checked = Checked {
        scenes: scenes.len(),
        prefabs: prefabs.len(),
        scripts: scripts.len(),
        stylesheets: stylesheets.len(),
    };
    finish(project, checked, diagnostics)
}

fn finish(project: &Path, checked: Checked, mut diagnostics: Vec<Diagnostic>) -> CheckReport {
    diagnostics.sort_by(|a, b| {
        (&a.path, a.line, a.column, &a.code).cmp(&(&b.path, b.line, b.column, &b.code))
    });
    CheckReport {
        schema_version: SCHEMA_VERSION,
        project: project.display().to_string(),
        ok: diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != Severity::Error),
        checked,
        diagnostics,
    }
}

/// The text files under `root` ending in `suffix`, by asset ID. One that is
/// not UTF-8 is left out here and has nothing to check.
fn text_files(root: &Path, suffix: &str) -> BTreeMap<String, String> {
    files_under(root, suffix)
        .into_iter()
        .filter_map(|(id, bytes)| String::from_utf8(bytes).ok().map(|text| (id, text)))
        .collect()
}

/// `sindri.toml`, and the main scene it names as an asset ID.
fn check_manifest(
    project: &Path,
    assets: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    match manifest(project) {
        Ok((main, sheets, scenes)) => {
            for sheet in sheets {
                if !assets.join(&sheet).is_file() {
                    diagnostics.push(Diagnostic::error(
                        "PROJECT_MISSING_STYLESHEET",
                        "sindri.toml",
                        format!("[assets] includes {sheet}, which is not there"),
                    ));
                }
            }
            for scene in scenes {
                if !project.join(&scene).is_file() {
                    diagnostics.push(Diagnostic::error(
                        "PROJECT_MISSING_SCENE",
                        "sindri.toml",
                        format!("[project] scenes lists {scene}, which is not there"),
                    ));
                }
            }
            Some(main)
        }
        Err(error) => {
            diagnostics.push(Diagnostic::error("PROJECT_MANIFEST", "sindri.toml", error));
            None
        }
    }
}

/// Every Decay script, against the project's own scripts and states; answers
/// which containers each file declares.
fn check_scripts(
    scripts: &BTreeMap<String, String>,
    shown: &impl Fn(&str) -> String,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<String, Vec<String>> {
    let mut sources = ScriptSources::new();
    for (id, text) in scripts {
        sources.insert(id.clone(), text.clone());
    }
    let environment = sources.environment().clone();
    let mut declared = BTreeMap::new();
    for (id, text) in scripts {
        let checked = check_source_in(text, &environment);
        for found in &checked.diagnostics {
            let code = match found.phase {
                CheckPhase::Syntax => format!("DECAY_SYNTAX_{}", screaming(found.code)),
                CheckPhase::Semantic => format!("DECAY_{}", screaming(found.code)),
            };
            diagnostics.push(
                Diagnostic::error(&code, &shown(id), found.message.clone())
                    .at(found.line, found.column),
            );
        }
        declared.insert(id.clone(), checked.declared);
    }
    declared
}

/// `unknown-name` as `UNKNOWN_NAME`, so every code in a report reads alike.
fn screaming(code: &str) -> String {
    code.replace('-', "_").to_ascii_uppercase()
}

/// What a scene's entities may refer to.
struct References<'a> {
    registry: &'a ComponentSchemaRegistry,
    /// Each component's own field names, where the scene schema closes them.
    fields: &'a BTreeMap<String, BTreeSet<String>>,
    declared: &'a BTreeMap<String, Vec<String>>,
    images: &'a BTreeSet<String>,
}

fn check_scene(path: &str, text: &str, refs: &References<'_>, diagnostics: &mut Vec<Diagnostic>) {
    let document = match SceneDocument::from_json(text) {
        Ok(document) => document,
        Err(error) => {
            diagnostics.push(Diagnostic::error(error.code(), path, error.to_string()));
            // A scene whose structure is wrong (a missing parent, say) still
            // reads as entities, and what they name is worth checking too.
            match serde_json::from_str::<SceneDocument>(text) {
                Ok(document) => document,
                Err(_) => return,
            }
        }
    };
    // The structure was reported above; this is what each component holds.
    if document.validate().is_ok()
        && let Err(error) = refs
            .registry
            .validate_scene(&document, UnknownComponentPolicy::Preserve)
        // A payload that does not decode is reported per entity below, with
        // serde's reason; the scene check stops at the first one.
        && !matches!(error, ComponentRegistryError::InvalidPayload { .. })
    {
        diagnostics.push(Diagnostic::error(error.code(), path, error.to_string()));
    }
    for entity in &document.entities {
        check_entity(path, entity, refs, diagnostics);
    }
}

fn check_prefab(path: &str, text: &str, refs: &References<'_>, diagnostics: &mut Vec<Diagnostic>) {
    let document = match PrefabDocument::from_json(text) {
        Ok(document) => document,
        Err(error) => {
            diagnostics.push(Diagnostic::error(error.code(), path, error.to_string()));
            return;
        }
    };
    for entity in &document.entities {
        check_entity(path, entity, refs, diagnostics);
    }
}

/// What one entity names: components the engine knows, a script that exists
/// and declares what it is asked for, a picture that is there.
fn check_entity(
    path: &str,
    entity: &SceneEntity,
    refs: &References<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let id = entity.id.as_str();
    for (type_name, payload) in &entity.components {
        let base = split_part(type_name).0;
        if base == WEAVE_STYLE {
            continue;
        }
        if refs.registry.metadata(base).is_none() {
            let nearest = nearest(base, refs.registry);
            let hint = nearest
                .map(|name| format!("; did you mean {name}?"))
                .unwrap_or_default();
            diagnostics.push(
                Diagnostic::warning(
                    "COMPONENT_UNKNOWN",
                    path,
                    format!("{type_name} is not a component this engine knows, so it is kept and does nothing{hint}"),
                )
                .on(id),
            );
            continue;
        }
        if let Err(ComponentRegistryError::InvalidPayload { source, .. }) =
            refs.registry.validate_payload(type_name, payload)
        {
            diagnostics.push(
                Diagnostic::error(
                    "COMPONENT_INVALID_PAYLOAD",
                    path,
                    format!("its {type_name} is not one the engine can read: {source}"),
                )
                .on(id),
            );
        }
        check_fields(path, id, type_name, payload, refs, diagnostics);
        if base == SCRIPT {
            check_script_reference(path, id, payload, refs, diagnostics);
        }
        if base == SPRITE {
            check_picture(path, id, payload, refs, diagnostics);
        }
    }
}

fn check_script_reference(
    path: &str,
    id: &str,
    payload: &Value,
    refs: &References<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (Some(source), Some(script)) = (
        payload.get("source").and_then(Value::as_str),
        payload.get("script").and_then(Value::as_str),
    ) else {
        return;
    };
    match refs.declared.get(source) {
        None => diagnostics.push(
            Diagnostic::error(
                "SCRIPT_MISSING_SOURCE",
                path,
                format!("its script source {source} is not in the project"),
            )
            .on(id),
        ),
        Some(declared) if !declared.iter().any(|name| name == script) => diagnostics.push(
            Diagnostic::error(
                "SCRIPT_UNDECLARED",
                path,
                format!(
                    "{source} declares no script {script}; it declares {}",
                    if declared.is_empty() {
                        "none".to_owned()
                    } else {
                        declared.join(", ")
                    }
                ),
            )
            .on(id),
        ),
        Some(_) => {}
    }
}

fn check_picture(
    path: &str,
    id: &str,
    payload: &Value,
    refs: &References<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(texture) = payload.get("texture").and_then(Value::as_str) else {
        return;
    };
    let file = texture.split('#').next().unwrap_or(texture);
    // Built in to the engine rather than shipped by the project.
    if file.is_empty() || file.starts_with("sindri:") || refs.images.contains(file) {
        return;
    }
    diagnostics.push(
        Diagnostic::error(
            "ASSET_MISSING_TEXTURE",
            path,
            format!("its sprite draws {file}, which is not in the project"),
        )
        .on(id),
    );
}

/// A field the component does not have: serde ignores it, so the scene
/// quietly does less than its author meant. A warning, as the engine runs the
/// scene regardless.
fn check_fields(
    path: &str,
    id: &str,
    type_name: &str,
    payload: &Value,
    refs: &References<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (Some(known), Some(written)) = (
        refs.fields.get(split_part(type_name).0),
        payload.as_object(),
    ) else {
        return;
    };
    for field in written.keys().filter(|field| !known.contains(*field)) {
        let hint = known
            .iter()
            .map(|name| (distance(field, name), name))
            .filter(|(distance, _)| *distance <= 3)
            .min()
            .map(|(_, name)| format!("; did you mean {name}?"))
            .unwrap_or_default();
        diagnostics.push(
            Diagnostic::warning(
                "COMPONENT_UNKNOWN_FIELD",
                path,
                format!("{type_name} has no field {field}, so it is ignored{hint}"),
            )
            .on(id),
        );
    }
}

/// Each component's own field names, read from the generated scene schema so
/// the check and an editor validating against the schema cannot disagree. A
/// component the schema leaves open in any of its forms is left out.
fn component_fields() -> BTreeMap<String, BTreeSet<String>> {
    let Ok(Some(schema)) = sindri_capabilities::reference("scene") else {
        return BTreeMap::new();
    };
    let Some(definitions) = schema.get("$defs").and_then(Value::as_object) else {
        return BTreeMap::new();
    };
    definitions
        .iter()
        .filter_map(|(name, definition)| {
            let forms: Vec<&Value> = ["anyOf", "oneOf"]
                .iter()
                .filter_map(|key| definition.get(*key).and_then(Value::as_array))
                .flatten()
                .chain(definition.get("properties").map(|_| definition))
                .collect();
            let mut names = BTreeSet::new();
            for form in &forms {
                if form.get("additionalProperties") != Some(&Value::Bool(false)) {
                    return None;
                }
                names.extend(form.get("properties")?.as_object()?.keys().cloned());
            }
            (!forms.is_empty()).then(|| (name.clone(), names))
        })
        .collect()
}

/// The registered component nearest `name`, if one is near enough to be a typo.
fn nearest<'a>(name: &str, registry: &'a ComponentSchemaRegistry) -> Option<&'a str> {
    registry
        .registered_components()
        .map(|metadata| {
            (
                distance(name, &metadata.type_name),
                metadata.type_name.as_str(),
            )
        })
        .filter(|(distance, _)| *distance <= 3)
        .min()
        .map(|(_, name)| name)
}

/// Edit distance, counted in characters.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, left) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, right) in b.iter().enumerate() {
            let replaced = previous + usize::from(left != *right);
            previous = row[j + 1];
            row[j + 1] = replaced.min(row[j] + 1).min(previous + 1);
        }
    }
    row[b.len()]
}

/// Each stylesheet parses, and composes with what it `@use`s.
fn check_stylesheets(
    sheets: &BTreeMap<String, String>,
    shown: &impl Fn(&str) -> String,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for id in sheets.keys() {
        if let Err(error) = weave::compose(id, sheets) {
            let mut found = Diagnostic::error(error.code(), &shown(id), error.to_string());
            if let weave::ComposeError::Parse { line, column, .. } = &error {
                found = found.at(*line, *column);
            }
            diagnostics.push(found);
        }
    }
}

#[cfg(test)]
mod tests;
