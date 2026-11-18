//! `sindri scene outline`: a scene or prefab as a tree, one line an entity.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;
use sindri_core::{SceneDocument, SceneEntity, split_part};

use super::Project;
use crate::report::SCHEMA_VERSION;

#[derive(Serialize)]
pub struct Outline {
    pub schema_version: u32,
    pub scene: String,
    pub entities: usize,
    pub roots: Vec<Node>,
}

/// One entity, and the ones under it.
#[derive(Serialize)]
pub struct Node {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Its component keys; a script shows the container it runs, as
    /// `sindri.script(Hero)`.
    pub components: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefab: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Node>,
}

/// The scene or prefab called `name`, as a tree.
///
/// # Errors
/// No such document, or one that does not read.
pub fn outline(project: &Project, name: Option<&str>) -> Result<Outline, String> {
    let name = name
        .map(str::to_owned)
        .or_else(|| project.main_scene.clone())
        .ok_or("name a scene with --scene; this project has no main scene")?;
    let (id, document) = project.document(&name)?;
    Ok(Outline {
        schema_version: SCHEMA_VERSION,
        scene: id.to_owned(),
        entities: document.entities.len(),
        roots: tree(document),
    })
}

fn tree(document: &SceneDocument) -> Vec<Node> {
    let ids: std::collections::BTreeSet<&str> = document
        .entities
        .iter()
        .map(|entity| entity.id.as_str())
        .collect();
    let mut children: BTreeMap<&str, Vec<&SceneEntity>> = BTreeMap::new();
    let mut roots = Vec::new();
    for entity in &document.entities {
        match entity
            .parent
            .as_ref()
            .map(sindri_core::SceneEntityId::as_str)
        {
            // A parent that is not in the file is shown at the top, where
            // it can be seen, rather than lost.
            Some(parent) if ids.contains(parent) => {
                children.entry(parent).or_default().push(entity);
            }
            _ => roots.push(entity),
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    roots
        .into_iter()
        .map(|root| node(root, &children, &mut seen))
        .collect()
}

fn node<'a>(
    entity: &'a SceneEntity,
    children: &BTreeMap<&str, Vec<&'a SceneEntity>>,
    seen: &mut std::collections::BTreeSet<&'a str>,
) -> Node {
    let id = entity.id.as_str();
    // A cycle in a broken file is drawn once rather than forever.
    let under = if seen.insert(id) {
        children
            .get(id)
            .into_iter()
            .flatten()
            .map(|child| node(child, children, seen))
            .collect()
    } else {
        Vec::new()
    };
    Node {
        id: id.to_owned(),
        name: entity.name.clone(),
        components: entity
            .components
            .iter()
            .map(
                |(key, payload)| match payload.get("script").and_then(Value::as_str) {
                    Some(script) if split_part(key).0 == "sindri.script" => {
                        format!("{key}({script})")
                    }
                    _ => key.clone(),
                },
            )
            .collect(),
        prefab: entity
            .prefab
            .as_ref()
            .map(|instance| instance.source.clone()),
        disabled: entity.disabled,
        children: under,
    }
}

impl Outline {
    #[must_use]
    pub fn human(&self) -> String {
        let mut out = format!("{} ({} entities)\n", self.scene, self.entities);
        for root in &self.roots {
            draw(root, 0, &mut out);
        }
        out
    }
}

fn draw(node: &Node, depth: usize, out: &mut String) {
    use std::fmt::Write as _;
    let name = node
        .name
        .as_deref()
        .filter(|name| *name != node.id)
        .map(|name| format!(" \"{name}\""))
        .unwrap_or_default();
    let components = node
        .components
        .iter()
        .map(|key| key.strip_prefix("sindri.").unwrap_or(key))
        .collect::<Vec<_>>()
        .join(", ");
    let prefab = node
        .prefab
        .as_deref()
        .map(|prefab| format!(" <{prefab}>"))
        .unwrap_or_default();
    let off = if node.disabled { " (off)" } else { "" };
    let _ = writeln!(
        out,
        "{}{}{name}{prefab}{off}{}",
        "  ".repeat(depth),
        node.id,
        if components.is_empty() {
            String::new()
        } else {
            format!(": {components}")
        }
    );
    for child in &node.children {
        draw(child, depth + 1, out);
    }
}
