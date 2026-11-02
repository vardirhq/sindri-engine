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
use sindri_authoring::visual_decay::{self, Flow, Layout, Member, ScriptEdit, Shape, Template};

use super::Host;

/// The script the Behaviour sheet has open.
pub(super) struct OpenScript {
    /// As the script component names it, under the project's assets.
    name: String,
    path: PathBuf,
    source: String,
    /// Where its nodes were dragged, from beside it.
    layout: Layout,
    /// Its text before each edit made here, latest last, for Undo.
    earlier: Vec<String>,
    /// Its text before each Undo, latest last, for Redo.
    undone: Vec<String>,
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

    /// Writes `source` as the script's text, after `edit` changed it; Undo
    /// and Redo move it between the two histories, any other edit forgets
    /// what was undone. If the file was changed elsewhere since it was read,
    /// both are forgotten, so going back never overwrites that work.
    fn write(&mut self, source: String, edit: Step) -> Result<(), String> {
        let on_disk = std::fs::read_to_string(&self.path).ok();
        if on_disk.as_deref() != Some(self.source.as_str()) {
            self.earlier.clear();
            self.undone.clear();
            if edit != Step::Edit {
                return Err(format!(
                    "{} was changed elsewhere, so there is nothing to go back to",
                    self.name
                ));
            }
        }
        std::fs::write(&self.path, &source).map_err(|error| format!("{}: {error}", self.name))?;
        let before = std::mem::replace(&mut self.source, source);
        match edit {
            Step::Edit => {
                self.earlier.push(before);
                self.undone.clear();
            }
            Step::Undo => self.undone.push(before),
            Step::Redo => self.earlier.push(before),
        }
        Ok(())
    }

    /// Goes back one edit, or forward again after going back.
    fn step(&mut self, edit: Step) -> Result<(), String> {
        let source = match edit {
            Step::Undo => self.earlier.pop().ok_or("there is nothing to undo")?,
            _ => self.undone.pop().ok_or("there is nothing to redo")?,
        };
        self.write(source, edit)
    }
}

/// What made a script's new text.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Edit,
    Undo,
    Redo,
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
            earlier: Vec::new(),
            undone: Vec::new(),
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
    /// `insertInside` (`value` `{ "template": id, "branch": n }`) or
    /// `useField` (the slot at `span` reads the field `value`) or `add`
    /// (`value` `{ "member": variable|function|event, "name", "type",
    /// "inputs": [{ "name", "type" }] }`), `remove` (`value` a name) or
    /// `rename` (`value` `{ "from", "to" }`), which take no `span`; or `undo`
    /// or `redo`, which take no `span` or `value`.
    pub(super) fn edit_script(
        &mut self,
        kind: &str,
        span: [usize; 2],
        value: &Value,
    ) -> Result<(), String> {
        if let Some(step) = match kind {
            "undo" => Some(Step::Undo),
            "redo" => Some(Step::Redo),
            _ => None,
        } {
            return self.script.as_mut().ok_or("no script is open")?.step(step);
        }
        if matches!(
            kind,
            "delete"
                | "moveUp"
                | "moveDown"
                | "insertAfter"
                | "insertInside"
                | "useField"
                | "add"
                | "remove"
                | "rename"
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
        script.write(edited, Step::Edit)
    }

    fn change_script(&mut self, kind: &str, span: [usize; 2], value: &Value) -> Result<(), String> {
        let opened = self.opened.as_ref().ok_or("no project is open")?;
        let script = self.script.as_mut().ok_or("no script is open")?;
        // Checked against the project's scripts, with this one as it is now.
        let mut sources = opened.assets.scripts.clone();
        sources.insert(script.name.clone(), script.source.clone());
        let environment = sources.environment();
        let template = |id: Option<&str>| {
            let id = id.ok_or("no statement was chosen")?;
            if id.starts_with("engine:") {
                Template::engine(id, environment)
            } else {
                Template::named(id, &visual_decay::blocks(&script.source), &script.source)
            }
            .ok_or_else(|| format!("no statement {id} for this script"))
        };
        let shape = match kind {
            "delete" => Shape::Delete(span),
            "moveUp" | "moveDown" => Shape::Move {
                statement: span,
                up: kind == "moveUp",
            },
            "add" => {
                let text = |key: &str| {
                    value
                        .get(key)
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .ok_or_else(|| format!("no {key} was given"))
                };
                let name = text("name")?;
                Shape::Add(match value.get("member").and_then(Value::as_str) {
                    Some("variable") => Member::Variable {
                        name,
                        ty: text("type")?,
                    },
                    Some("function") => Member::Function {
                        name,
                        inputs: value
                            .get("inputs")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                            .map(|input| {
                                let part = |key: &str| {
                                    input
                                        .get(key)
                                        .and_then(Value::as_str)
                                        .unwrap_or_default()
                                        .to_owned()
                                };
                                (part("name"), part("type"))
                            })
                            .collect(),
                    },
                    Some("event") => Member::Event { name },
                    _ => return Err("nothing to add was chosen".to_owned()),
                })
            }
            "remove" => Shape::Remove {
                name: value
                    .as_str()
                    .ok_or("nothing to remove was chosen")?
                    .to_owned(),
            },
            "rename" => Shape::Rename {
                from: value
                    .get("from")
                    .and_then(Value::as_str)
                    .ok_or("nothing to rename was chosen")?
                    .to_owned(),
                to: value
                    .get("to")
                    .and_then(Value::as_str)
                    .ok_or("no new name was given")?
                    .to_owned(),
            },
            "useField" => Shape::Field {
                slot: span,
                field: value.as_str().ok_or("no field was chosen")?.to_owned(),
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
        let changed = visual_decay::change(&script.source, &shape, environment)?;
        script.write(changed, Step::Edit)
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

    /// The engine calls a step can add, for the finder: `{ id, call }`.
    fn engine_calls(&self) -> Vec<Value> {
        let Some(opened) = self.opened.as_ref() else {
            return Vec::new();
        };
        visual_decay::engine_calls(opened.assets.scripts.environment())
            .into_iter()
            .map(|(id, call)| json!({ "id": id, "call": call }))
            .collect()
    }

    /// The open script for the Behaviour sheet, as JSON: its `name`, its
    /// `source` for the Code view, its `blocks`, its node `graph`, placed,
    /// the `palette` of statements its node menu can add, and the `engine`
    /// calls its finder offers; `null` with none
    /// open.
    pub(super) fn script(&self) -> String {
        self.script.as_ref().map_or_else(
            || "null".to_owned(),
            |script| {
                json!({
                    "name": script.name,
                    "source": script.source,
                    "canUndo": !script.earlier.is_empty(),
                    "canRedo": !script.undone.is_empty(),
                    "blocks": visual_decay::blocks(&script.source),
                    "graph": script.graph(),
                    "palette": visual_decay::palette(&visual_decay::blocks(&script.source))
                        .into_iter()
                        .map(|(id, label)| json!({ "id": id, "label": label }))
                        .collect::<Vec<_>>(),
                    "engine": self.engine_calls(),
                })
                .to_string()
            },
        )
    }
}
