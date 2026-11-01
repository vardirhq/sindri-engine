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
use sindri_authoring::visual_decay::{self, Layout, ScriptEdit};

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
        let mut graph = visual_decay::graph(&visual_decay::blocks(&self.source), &self.source);
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

    /// Makes a block's edit to the open script and writes it to its file.
    /// `kind` is `number`, `text` or `bool`; `span` the slot's bytes.
    pub(super) fn edit_script(
        &mut self,
        kind: &str,
        span: [usize; 2],
        value: &Value,
    ) -> Result<(), String> {
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

    /// A node of the open script was dragged to `at`: kept in its layout file.
    pub(super) fn move_node(&mut self, key: &str, at: [f32; 2]) -> Result<(), String> {
        let script = self.script.as_mut().ok_or("no script is open")?;
        let graph = script.graph();
        let node = graph
            .nodes
            .iter()
            .find(|node| node.key == key)
            .ok_or_else(|| format!("the script has no node {key} now"))?;
        script.layout.moved(node, at);
        let path = script.layout_path();
        std::fs::write(&path, script.layout.to_json()?)
            .map_err(|error| format!("{}{LAYOUT_SUFFIX}: {error}", script.name))
    }

    /// The open script for the Behaviour sheet, as JSON: its `name`, its
    /// `source` for the Code view, its `blocks` and its node `graph`, placed;
    /// `null` with none open.
    pub(super) fn script(&self) -> String {
        self.script.as_ref().map_or_else(
            || "null".to_owned(),
            |script| {
                json!({
                    "name": script.name,
                    "source": script.source,
                    "blocks": visual_decay::blocks(&script.source),
                    "graph": script.graph(),
                })
                .to_string()
            },
        )
    }
}
