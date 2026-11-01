//! Where a script's nodes are on the canvas.
//!
//! Positions are the one thing a node graph has that a script does not, so
//! they live beside it, in `<script>.layout` (`wick.decay.layout`), and never
//! in it: dragging a node changes no byte of the script. A node nobody has
//! placed is laid out from the code, execution running left to right, nested
//! chains in lanes below, and the values a node takes in a column to its left.
//!
//! A stored position finds its node by key (its place in the code) and
//! fingerprint (its text). When the script was edited elsewhere and the node
//! moved in the code, its fingerprint still finds it; a node whose text
//! changed and whose place is taken is laid out afresh. Fields this version
//! does not know are kept as they were, so a newer editor's data survives an
//! older one.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::graph::{Graph, Node};

/// The canvas's measures, in points, which the app draws nodes to as well:
/// a node is [`WIDTH`] wide, a header and one [`ROW`] per pin row tall.
pub const WIDTH: f32 = 240.0;
pub const HEADER: f32 = 36.0;
pub const ROW: f32 = 32.0;
const ACROSS: f32 = WIDTH + 70.0;
const BETWEEN: f32 = 28.0;
const LANE: f32 = 48.0;
const BAND: f32 = 96.0;
/// How far left of what it feeds a value node stands.
const VALUE_ACROSS: f32 = WIDTH + 48.0;

/// The `.layout` file's version, which a reader older than the file refuses
/// to rewrite rather than lose what it cannot read.
pub const LAYOUT_VERSION: u32 = 1;

/// The suffix the positions file adds to the script's own name.
pub const LAYOUT_SUFFIX: &str = ".layout";

/// How tall `node` is drawn.
#[must_use]
pub fn height(node: &Node) -> f32 {
    #[allow(clippy::cast_precision_loss)] // A node has a handful of pins.
    let rows = node.inputs.len().max(node.outputs.len()).max(1) as f32;
    HEADER + rows * ROW + 8.0
}

/// Positions stored beside a script.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub format_version: u32,
    #[serde(default)]
    pub nodes: BTreeMap<String, Stored>,
    /// What a newer editor wrote that this one does not read, kept.
    #[serde(flatten)]
    pub unknown: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stored {
    pub at: [f32; 2],
    pub print: String,
    #[serde(flatten)]
    pub unknown: BTreeMap<String, Value>,
}

impl Layout {
    /// A layout file's text, or an empty layout for none.
    ///
    /// # Errors
    /// Text that is not a layout, or one of a newer version than this
    /// editor writes, which it must not overwrite.
    pub fn read(text: Option<&str>) -> Result<Self, String> {
        let Some(text) = text else {
            return Ok(Self {
                format_version: LAYOUT_VERSION,
                ..Self::default()
            });
        };
        let layout: Self = serde_json::from_str(text)
            .map_err(|error| format!("the layout file does not read: {error}"))?;
        if layout.format_version > LAYOUT_VERSION {
            return Err(format!(
                "the layout file is version {}, newer than this editor's {LAYOUT_VERSION}",
                layout.format_version
            ));
        }
        Ok(layout)
    }

    /// The file's text: stable, indented JSON, so it diffs well.
    ///
    /// # Errors
    /// Never in practice; JSON of maps and numbers always writes.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map(|mut text| {
                text.push('\n');
                text
            })
            .map_err(|error| error.to_string())
    }

    /// `node` was dragged to `at`.
    pub fn moved(&mut self, node: &Node, at: [f32; 2]) {
        let stored = self
            .nodes
            .entry(node.key.clone())
            .or_insert_with(|| Stored {
                at,
                print: node.print.clone(),
                unknown: BTreeMap::new(),
            });
        stored.at = at;
        stored.print.clone_from(&node.print);
    }

    /// Lays this layout's positions over `graph`'s laid-out ones: by key where
    /// the fingerprint agrees, else by a fingerprint no other node shares.
    pub fn place(&self, graph: &mut Graph) {
        let mut used = BTreeSet::new();
        for node in &mut graph.nodes {
            if let Some(stored) = self
                .nodes
                .get(&node.key)
                .filter(|stored| stored.print == node.print)
            {
                node.at = stored.at;
                node.placed = true;
                used.insert(node.key.clone());
            }
        }
        let mut by_print: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
        for (key, stored) in &self.nodes {
            if !used.contains(key) {
                by_print.entry(stored.print.as_str()).or_default().push(key);
            }
        }
        let unplaced_prints: Vec<String> = graph
            .nodes
            .iter()
            .filter(|node| !node.placed)
            .map(|node| node.print.clone())
            .collect();
        for node in graph.nodes.iter_mut().filter(|node| !node.placed) {
            let alone = unplaced_prints
                .iter()
                .filter(|print| **print == node.print)
                .count()
                == 1;
            if let Some([key]) = by_print.get(node.print.as_str()).map(Vec::as_slice)
                && alone
            {
                node.at = self.nodes[*key].at;
                node.placed = true;
            }
        }
    }
}

/// Lays `graph` out from the code: each handler a band, its chain left to
/// right, nested chains in lanes below, values to the left of what takes them.
pub(super) fn arrange(graph: &mut Graph) {
    let index: BTreeMap<String, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(at, node)| (node.key.clone(), at))
        .collect();
    let mut flow: BTreeMap<(usize, String), usize> = BTreeMap::new();
    let mut feeds: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for wire in &graph.wires {
        let (Some(&from), Some(&to)) = (index.get(&wire.from), index.get(&wire.to)) else {
            continue;
        };
        if wire.flow {
            flow.insert((from, wire.out.clone()), to);
        } else {
            feeds.entry(to).or_default().push(from);
        }
    }
    let mut arranger = Arranger {
        heights: graph.nodes.iter().map(height).collect(),
        at: vec![[0.0, 0.0]; graph.nodes.len()],
        outputs: graph
            .nodes
            .iter()
            .map(|node| node.outputs.clone())
            .collect(),
        flow,
        feeds,
    };
    let mut top = 0.0;
    for (at, node) in graph.nodes.iter().enumerate() {
        if node.kind == "values" {
            arranger.at[at] = [0.0, top];
            top += arranger.heights[at] + BAND;
        } else if matches!(node.kind, "event" | "function") {
            top += arranger.chain(at, 0.0, top) + BAND;
        }
    }
    for (node, at) in graph.nodes.iter_mut().zip(arranger.at) {
        node.at = at;
    }
}

struct Arranger {
    heights: Vec<f32>,
    at: Vec<[f32; 2]>,
    outputs: Vec<Vec<String>>,
    flow: BTreeMap<(usize, String), usize>,
    feeds: BTreeMap<usize, Vec<usize>>,
}

impl Arranger {
    /// Places the chain starting at `first` from `x`, `y`; answers how tall it
    /// came out, its lanes included. Each statement stands in a cell with the
    /// values it takes to its left, as a Blueprint reads: values flow right
    /// into their pins, and execution flows right from cell to cell.
    fn chain(&mut self, first: usize, x: f32, y: f32) -> f32 {
        let mut members = vec![first];
        let mut at = first;
        while let Some(&next) = self
            .flow
            .get(&(at, String::new()))
            .or_else(|| self.flow.get(&(at, "Next".to_owned())))
        {
            members.push(next);
            at = next;
        }
        let mut row = 0.0_f32;
        let mut cell = x;
        let mut columns = Vec::with_capacity(members.len());
        for &member in &members {
            #[allow(clippy::cast_precision_loss)] // Values nest a few deep.
            let left = self.depth(member) as f32 * VALUE_ACROSS;
            let column = cell + left;
            self.at[member] = [column, y];
            // A row down, so the execution wire into the cell passes above them.
            let values = self.values(member, column, y + HEADER + ROW);
            row = row.max(self.heights[member].max(values));
            columns.push(column);
            cell = column + ACROSS;
        }
        let mut lane = y + row + LANE;
        for (&member, &column) in members.iter().zip(&columns) {
            let nested: Vec<usize> = self.outputs[member]
                .iter()
                .filter(|output| !output.is_empty() && *output != "Next")
                .filter_map(|output| self.flow.get(&(member, output.clone())).copied())
                .collect();
            for start in nested {
                lane += self.chain(start, column + ACROSS / 2.0, lane) + LANE;
            }
        }
        lane - LANE - y
    }

    /// How many value nodes deep the values `node` takes go.
    fn depth(&self, node: usize) -> usize {
        self.feeds.get(&node).map_or(0, |feeding| {
            1 + feeding
                .iter()
                .map(|value| self.depth(*value))
                .max()
                .unwrap_or(0)
        })
    }

    /// Stacks the value nodes `node` (at `x`) takes in a column to its left,
    /// from `top`, each one's own values further left again; answers how far
    /// down from `top` they reach.
    fn values(&mut self, node: usize, x: f32, top: f32) -> f32 {
        let Some(feeding) = self.feeds.get(&node).cloned() else {
            return 0.0;
        };
        let column = x - VALUE_ACROSS;
        let mut bottom = top;
        for value in feeding {
            self.at[value] = [column, bottom];
            let own = self.values(value, column, bottom + HEADER);
            bottom += self.heights[value].max(own) + BETWEEN;
        }
        bottom - top
    }
}
