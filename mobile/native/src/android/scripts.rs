//! A script opened from the Behaviour sheet, as Decay nodes.
//!
//! The `.decay` file is the script; the node graph is how the phone shows it
//! (`sindri_authoring::visual_decay`). An edit replaces the text of the one
//! slot it changed and is written straight back to the file, so the script
//! reads on desktop exactly as it would had it been typed there. Where nodes
//! were dragged is kept beside it, in `<script>.layout`, never in it.

use std::path::PathBuf;

use serde_json::{Value, json};
use sindri_authoring::visual_decay::layout::LAYOUT_SUFFIX;
use sindri_authoring::visual_decay::{self, Flow, Layout, ScriptEdit, Shape, Template};

use super::Host;

/// The script the Behaviour sheet has open.
pub(super) struct OpenScript {
    /// As the script component names it, under the project's assets.
    name: String,
    path: PathBuf,
    source: String,
    /// Where its nodes were dragged, from beside it.
    layout: Layout,
}

impl OpenScript {
    fn layout_path(&self) -> PathBuf {
        let mut name = self.path.clone().into_os_string();
        name.push(LAYOUT_SUFFIX);
        PathBuf::from(name)
    }

    fn graph(&self) -> visual_decay::Graph {
        // Top to bottom, as code reads: a phone is held upright.
        let mut graph = visual_decay::graph_flowing(
            &visual_decay::blocks(&self.source),
            &self.source,
            Flow::Down,
        );
        self.layout.place(&mut graph);
        graph
    }
}

impl Host {
    /// Opens the script the selection's script component names, such as
    /// `scripts/wick.decay`.
    pub(super) fn open_script(&mut self, name: &str) -> Result<(), String> {
        let (project, _) = self.source.as_ref().ok_or("no project is open")?;
        if name.contains("..") {
            return Err(format!("{name} is not in the project"));
        }
        let path = sindri_runtime::project::assets_root(project).join(name);
        let source = std::fs::read_to_string(&path).map_err(|error| format!("{name}: {error}"))?;
        let mut script = OpenScript {
            name: name.to_owned(),
            path,
            source,
            layout: Layout::default(),
        };
        let stored = std::fs::read_to_string(script.layout_path()).ok();
        script.layout = Layout::read(stored.as_deref())?;
        self.script = Some(script);
        Ok(())
    }

    pub(super) fn close_script(&mut self) {
        self.script = None;
    }

    /// Makes a node's edit to the open script and writes it to its file.
    /// `kind` is `number`, `text` or `bool` for a slot at `span`, with its
    /// new `value`; or a change of shape at the node at `span`: `delete`,
    /// `moveUp`, `moveDown`, `insertAfter` (`value` a palette id) or
    /// `insertInside` (`value` `{ "template": id, "branch": n }`).
    pub(super) fn edit_script(
        &mut self,
        kind: &str,
        span: [usize; 2],
        value: &Value,
    ) -> Result<(), String> {
        if matches!(
            kind,
            "delete" | "moveUp" | "moveDown" | "insertAfter" | "insertInside"
        ) {
            return self.change_script(kind, span, value);
        }
        let script = self.script.as_mut().ok_or("no script is open")?;
        let edit = match kind {
            "number" => ScriptEdit::Number {
                span,
                value: value.as_f64().ok_or("a number slot takes a number")?,
            },
            "text" => ScriptEdit::Text {
                span,
                value: value.as_str().ok_or("a text slot takes text")?.to_owned(),
            },
            "bool" => ScriptEdit::Bool {
                span,
                value: value.as_bool().ok_or("a switch slot takes a switch")?,
            },
            other => return Err(format!("no slot of kind {other}")),
        };
        let edited = visual_decay::edit(&script.source, &edit)?;
        std::fs::write(&script.path, &edited)
            .map_err(|error| format!("{}: {error}", script.name))?;
        script.source = edited;
        Ok(())
    }

    fn change_script(&mut self, kind: &str, span: [usize; 2], value: &Value) -> Result<(), String> {
        let opened = self.opened.as_ref().ok_or("no project is open")?;
        let script = self.script.as_mut().ok_or("no script is open")?;
        let template = |id: Option<&str>| {
            let id = id.ok_or("no statement was chosen")?;
            Template::named(id, &visual_decay::blocks(&script.source), &script.source)
                .ok_or_else(|| format!("no statement {id} for this script"))
        };
        let shape = match kind {
            "delete" => Shape::Delete(span),
            "moveUp" | "moveDown" => Shape::Move {
                statement: span,
                up: kind == "moveUp",
            },
            "insertAfter" => Shape::After {
                statement: span,
                template: template(value.as_str())?,
            },
            _ => Shape::Inside {
                node: span,
                branch: value
                    .get("branch")
                    .and_then(Value::as_u64)
                    .and_then(|branch| usize::try_from(branch).ok())
                    .unwrap_or(0),
                template: template(value.get("template").and_then(Value::as_str))?,
            },
        };
        // Checked against the project's scripts, with this one as it is now.
        let mut sources = opened.assets.scripts.clone();
        sources.insert(script.name.clone(), script.source.clone());
        let changed = visual_decay::change(&script.source, &shape, sources.environment())?;
        std::fs::write(&script.path, &changed)
            .map_err(|error| format!("{}: {error}", script.name))?;
        script.source = changed;
        Ok(())
    }

    /// A node of the open script was dragged to `at`: kept in its layout file.
    pub(super) fn move_node(&mut self, key: &str, at: [f32; 2]) -> Result<(), String> {
        let script = self.script.as_mut().ok_or("no script is open")?;
        let graph = script.graph();
        let node = graph
            .nodes
            .iter()
            .find(|node| node.key == key)
            .ok_or_else(|| format!("the script has no node {key} now"))?;
        script.layout.moved(node, Flow::Down, at);
        let path = script.layout_path();
        std::fs::write(&path, script.layout.to_json()?)
            .map_err(|error| format!("{}{LAYOUT_SUFFIX}: {error}", script.name))
    }

    /// The open script for the Behaviour sheet, as JSON: its `name`, its
    /// `source` for the Code view, its `blocks`, its node `graph`, placed,
    /// and the `palette` of statements its node menu can add; `null` with none
    /// open.
    pub(super) fn script(&self) -> String {
        self.script.as_ref().map_or_else(
            || "null".to_owned(),
            |script| {
                json!({
                    "name": script.name,
                    "source": script.source,
                    "blocks": visual_decay::blocks(&script.source),
                    "graph": script.graph(),
                    "palette": visual_decay::palette(&visual_decay::blocks(&script.source))
                        .into_iter()
                        .map(|(id, label)| json!({ "id": id, "label": label }))
                        .collect::<Vec<_>>(),
                })
                .to_string()
            },
        )
    }
}
