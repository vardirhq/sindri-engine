//! Weave questions: which elements a selector reaches, what one element ends
//! up styled with and why, and which rules reach nothing.
//!
//! Answered by the cascade the game styles with: `sindri_weave::inspect`, the
//! function behind the editor's style panel, over each scene's own world, so
//! what these say is what is drawn. Elements are matched at rest: a rule for
//! `:hover` or `:focus` reaches an element only while a pointer or a key
//! puts it in that state, which no file can.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use sindri_core::{EntityId, SceneDocument, World};
use weave::{Stylesheet, Viewport};

use super::Project;
use crate::report::SCHEMA_VERSION;

/// One rule as `style show` lists it.
#[derive(Serialize)]
pub struct ShownRule {
    pub file: String,
    pub line: usize,
    pub selector: String,
    /// IDs, then classes and states, then element names, as CSS counts.
    pub specificity: [u16; 3],
    pub declarations: Vec<Declaration>,
}

#[derive(Serialize)]
pub struct Declaration {
    pub property: String,
    pub value: String,
    /// Whether this is the declaration that applies; `false` when a stronger
    /// rule, or a later stylesheet, sets the same property.
    pub applies: bool,
}

/// `style show`.
#[derive(Serialize)]
pub struct StyleShown {
    pub schema_version: u32,
    pub scene: String,
    pub id: String,
    /// Strongest first.
    pub rules: Vec<ShownRule>,
    /// Every property that applies, inherited ones included.
    pub computed: BTreeMap<String, String>,
    pub variables: BTreeMap<String, String>,
}

/// The project's stylesheets, composed as a run composes them.
///
/// # Errors
/// A manifest or stylesheet that does not read.
pub fn stylesheets(project: &Path) -> Result<Vec<Stylesheet>, String> {
    sindri_runtime::project::stylesheets(project)
}

/// A scene or prefab as a world. A prefab is written like a scene's entities
/// with its own format version, so it is loaded as the current scene format.
fn world(document: &SceneDocument) -> Option<World> {
    let mut document = document.clone();
    document.format_version = sindri_core::SCENE_FORMAT_VERSION;
    World::from_scene(&document).ok().map(|loaded| loaded.world)
}

fn entity(world: &World, id: &str) -> Option<EntityId> {
    world.entity_for_source_id(&sindri_core::SceneEntityId::new(id.to_owned()).ok()?)
}

/// How element `id` is styled, rule by rule.
///
/// # Errors
/// No such entity, or one that is not a styled element.
pub fn show(
    project: &Project,
    sheets: &[Stylesheet],
    id: &str,
    scene: Option<&str>,
    viewport: Viewport,
) -> Result<StyleShown, String> {
    let shown = super::show(project, id, scene)?;
    let (_, document) = project.document(&shown.scene)?;
    let world = world(document).ok_or("the scene does not load")?;
    let entity = entity(&world, id).ok_or("the entity does not load")?;
    let inspection = sindri_weave::inspect(&world, sheets, viewport, &BTreeMap::new(), entity)
        .ok_or_else(|| format!("{id} is not a styled element: it has no sindri.ui component"))?;
    Ok(StyleShown {
        schema_version: SCHEMA_VERSION,
        scene: shown.scene,
        id: id.to_owned(),
        rules: inspection
            .rules
            .into_iter()
            .map(|rule| ShownRule {
                file: rule.origin.file,
                line: rule.origin.line,
                selector: rule.origin.selector,
                specificity: [rule.specificity.0, rule.specificity.1, rule.specificity.2],
                declarations: rule
                    .declarations
                    .into_iter()
                    .map(|(property, value, applies)| Declaration {
                        property,
                        value,
                        applies,
                    })
                    .collect(),
            })
            .collect(),
        computed: inspection.computed.into_iter().collect(),
        variables: inspection.variables.into_iter().collect(),
    })
}

/// `style match`.
#[derive(Serialize)]
pub struct StyleMatches {
    pub schema_version: u32,
    pub selector: String,
    /// Each element it reaches, as `scene` and `id`.
    pub found: Vec<Reached>,
}

#[derive(Serialize)]
pub struct Reached {
    pub scene: String,
    pub id: String,
}

/// Every element across the scenes and prefabs that `selector` reaches.
///
/// # Errors
/// A selector Weave does not read.
pub fn matching(
    project: &Project,
    selector: &str,
    viewport: Viewport,
) -> Result<StyleMatches, String> {
    let probe = weave::parse(&format!("{selector} {{ --sindri-probe: 1; }}"))
        .map_err(|error| format!("{selector}: {error}"))?;
    let mut found = Vec::new();
    for (scene, world) in worlds(project) {
        for (entity, data) in world.entities() {
            let reached = sindri_weave::inspect(
                &world,
                std::slice::from_ref(&probe),
                viewport,
                &BTreeMap::new(),
                entity,
            )
            .is_some_and(|inspection| !inspection.rules.is_empty());
            if reached && let Some(id) = &data.source_id {
                found.push(Reached {
                    scene: scene.clone(),
                    id: id.as_str().to_owned(),
                });
            }
        }
    }
    Ok(StyleMatches {
        schema_version: SCHEMA_VERSION,
        selector: selector.to_owned(),
        found,
    })
}

/// `style unused`.
#[derive(Serialize)]
pub struct Unused {
    pub schema_version: u32,
    pub rules: Vec<Unreached>,
}

impl Unused {
    #[must_use]
    pub fn of(project: &Project, sheets: &[Stylesheet], viewport: Viewport) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            rules: unreached(project, sheets, viewport),
        }
    }

    #[must_use]
    pub fn human(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        for rule in &self.rules {
            let _ = writeln!(
                out,
                "{}:{}: {} reaches nothing",
                rule.file, rule.line, rule.selector
            );
        }
        let _ = writeln!(out, "{} rules reach nothing at rest", self.rules.len());
        out
    }
}

/// A rule nothing in the project reaches.
#[derive(Clone, Debug, Serialize)]
pub struct Unreached {
    pub file: String,
    pub line: usize,
    pub selector: String,
}

/// The rules no element of any scene or prefab reaches at rest.
///
/// Rules that wait on a state (`:hover`, `:focus`, …) or on a screen size
/// (`@media`) are left out: no file can show them unreached. So are
/// `:root` and `*`, which reach whatever there is.
#[must_use]
pub fn unreached(project: &Project, sheets: &[Stylesheet], viewport: Viewport) -> Vec<Unreached> {
    let mut reached: std::collections::BTreeSet<(String, usize, String)> =
        std::collections::BTreeSet::default();
    for (_, world) in worlds(project) {
        for (entity, _) in world.entities() {
            let Some(inspection) =
                sindri_weave::inspect(&world, sheets, viewport, &BTreeMap::new(), entity)
            else {
                continue;
            };
            for rule in inspection.rules {
                reached.insert((rule.origin.file, rule.origin.line, rule.origin.selector));
            }
        }
    }
    let mut found = Vec::new();
    for sheet in sheets {
        for rule in &sheet.rules {
            let origin = &rule.origin;
            let waits = !rule.conditions.is_empty()
                || rule
                    .selector
                    .compounds
                    .iter()
                    .any(|compound| compound.states != weave::States::default())
                || origin.selector.contains(':');
            let anything = matches!(origin.selector.trim(), "*" | ":root");
            if waits || anything {
                continue;
            }
            let key = (origin.file.clone(), origin.line, origin.selector.clone());
            if !reached.contains(&key) {
                found.push(Unreached {
                    file: origin.file.clone(),
                    line: origin.line,
                    selector: origin.selector.clone(),
                });
            }
        }
    }
    found.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    found.dedup_by(|a, b| a.file == b.file && a.line == b.line && a.selector == b.selector);
    found
}

/// Every scene and prefab that loads, as a world.
fn worlds(project: &Project) -> Vec<(String, World)> {
    project
        .documents
        .iter()
        .filter_map(|(id, read)| {
            let document = read.as_ref().ok()?;
            Some((id.clone(), world(document)?))
        })
        .collect()
}

impl StyleShown {
    #[must_use]
    pub fn human(&self) -> String {
        use std::fmt::Write as _;
        let mut out = format!("{} in {}\n", self.id, self.scene);
        for rule in &self.rules {
            let [a, b, c] = rule.specificity;
            let _ = writeln!(
                out,
                "  {} ({}:{}, specificity {a},{b},{c})",
                rule.selector, rule.file, rule.line
            );
            for declaration in &rule.declarations {
                let mark = if declaration.applies { " " } else { "x" };
                let _ = writeln!(
                    out,
                    "   {mark} {}: {}",
                    declaration.property, declaration.value
                );
            }
        }
        let _ = writeln!(out, "computed:");
        for (property, value) in &self.computed {
            let _ = writeln!(out, "  {property}: {value}");
        }
        out
    }
}

impl StyleMatches {
    #[must_use]
    pub fn human(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        for reached in &self.found {
            let _ = writeln!(out, "{}: {}", reached.scene, reached.id);
        }
        let _ = writeln!(
            out,
            "{} reaches {} elements",
            self.selector,
            self.found.len()
        );
        out
    }
}
