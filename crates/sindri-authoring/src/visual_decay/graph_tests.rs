//! The node graph and its layout, on a small script and every real one.

use std::collections::BTreeSet;
use std::path::Path;

use super::layout::{self, Layout, height};
use super::*;

const LAMP: &str = "\
script Lamp {
    @export let glow: number = 1.5;

    fn update(dt: number) {
        // Light up when Wick is near.
        if !this.lit && World.near(this, \"wick\", 1.2) {
            this.lit = true;
            Camera.shake(0.3);
        }
        this.age += dt;
    }
}
";

fn lamp() -> Graph {
    graph(&blocks(LAMP), LAMP)
}

fn node<'a>(graph: &'a Graph, key: &str) -> &'a Node {
    graph
        .nodes
        .iter()
        .find(|node| node.key == key)
        .unwrap_or_else(|| {
            panic!(
                "no node {key}: {:?}",
                graph.nodes.iter().map(|node| &node.key).collect::<Vec<_>>()
            )
        })
}

fn wired(graph: &Graph, from: &str, out: &str, to: &str, into: &str) -> bool {
    graph
        .wires
        .iter()
        .any(|wire| wire.from == from && wire.out == out && wire.to == to && wire.into == into)
}

#[test]
fn a_script_is_event_nodes_chains_of_statements_and_values_wired_in() {
    let graph = lamp();
    assert_eq!(node(&graph, "Lamp.values").inputs[0].name, "glow");
    let event = node(&graph, "Lamp.update");
    assert_eq!(
        (event.kind, event.title.as_str()),
        ("event", "Every frame (dt)")
    );

    let branch = node(&graph, "Lamp.update/0");
    assert_eq!(branch.title, "Branch");
    assert_eq!(branch.note.as_deref(), Some("Light up when Wick is near."));
    assert_eq!(branch.outputs, ["True", "False", "Next"]);
    assert!(wired(&graph, "Lamp.update", "", "Lamp.update/0", ""));
    // The condition is a value node: `&&` of `not` and the call.
    assert!(wired(&graph, "Lamp.update/0:if", "", "Lamp.update/0", "if"));
    let and = node(&graph, "Lamp.update/0:if");
    assert_eq!((and.kind, and.title.as_str()), ("value", "&&"));
    let near = node(&graph, "Lamp.update/0:if:b");
    assert_eq!(near.title, "World.near");
    assert!(matches!(&near.inputs[1].value, Some(Slot::Text { value, .. }) if value == "wick"));
    assert!(
        matches!(&near.inputs[2].value, Some(Slot::Number { .. })),
        "a literal stays on its pin"
    );

    // True runs the nested chain; Next continues after the branch.
    assert_eq!(node(&graph, "Lamp.update/0.0/0").title, "Set this.lit");
    assert!(wired(
        &graph,
        "Lamp.update/0",
        "True",
        "Lamp.update/0.0/0",
        ""
    ));
    assert_eq!(node(&graph, "Lamp.update/0.0/1").title, "Camera.shake");
    assert!(wired(
        &graph,
        "Lamp.update/0.0/0",
        "",
        "Lamp.update/0.0/1",
        ""
    ));
    assert_eq!(node(&graph, "Lamp.update/1").title, "Add to this.age");
    assert!(wired(&graph, "Lamp.update/0", "Next", "Lamp.update/1", ""));
}

#[test]
fn every_script_makes_a_graph_with_unique_keys_and_no_loose_wires() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games");
    let mut seen = 0;
    for game in std::fs::read_dir(root).expect("games").flatten() {
        let scripts = game.path().join("assets/scripts");
        let Ok(entries) = std::fs::read_dir(scripts) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_none_or(|extension| extension != "decay")
            {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("reads");
            let graph = graph(&blocks(&source), &source);
            let keys: BTreeSet<&str> = graph.nodes.iter().map(|node| node.key.as_str()).collect();
            assert_eq!(
                keys.len(),
                graph.nodes.len(),
                "{}: duplicate keys",
                path.display()
            );
            for wire in &graph.wires {
                assert!(
                    keys.contains(wire.from.as_str()) && keys.contains(wire.to.as_str()),
                    "{}: loose wire {wire:?}",
                    path.display()
                );
            }
            let corners: BTreeSet<(u32, u32)> = graph
                .nodes
                .iter()
                .map(|node| (node.at[0].to_bits(), node.at[1].to_bits()))
                .collect();
            assert_eq!(
                corners.len(),
                graph.nodes.len(),
                "{}: two nodes laid out in one place",
                path.display()
            );
            seen += 1;
        }
    }
    assert!(seen > 30, "{seen} scripts");
}

#[test]
fn a_chain_runs_left_to_right_and_values_stand_left_of_what_takes_them() {
    let graph = lamp();
    let branch = node(&graph, "Lamp.update/0");
    let after = node(&graph, "Lamp.update/1");
    assert!(after.at[0] > branch.at[0] && (after.at[1] - branch.at[1]).abs() < f32::EPSILON);
    let condition = node(&graph, "Lamp.update/0:if");
    assert!(
        condition.at[0] + layout::WIDTH < branch.at[0],
        "{:?} left of {:?}",
        condition.at,
        branch.at
    );
    let near = node(&graph, "Lamp.update/0:if:b");
    assert!(
        near.at[0] + layout::WIDTH < condition.at[0],
        "a value's own values stand further left"
    );
    let event = node(&graph, "Lamp.update");
    assert!(
        near.at[0] > event.at[0] + layout::WIDTH,
        "values do not cover the event before them"
    );
    let nested = node(&graph, "Lamp.update/0.0/0");
    assert!(
        nested.at[1] > branch.at[1] + height(branch),
        "the True chain is in a lane below"
    );
}

#[test]
fn a_stored_position_finds_its_node_by_place_or_by_text_and_is_kept_out_of_the_script() {
    let mut layout = Layout::read(None).expect("an empty layout");
    let mut graph = lamp();
    let shake = node(&graph, "Lamp.update/0.0/1").clone();
    layout.moved(&shake, [900.0, 40.0]);
    layout.place(&mut graph);
    assert_eq!(
        node(&graph, "Lamp.update/0.0/1").at.map(f32::to_bits),
        [900.0_f32, 40.0].map(f32::to_bits)
    );
    assert!(node(&graph, "Lamp.update/0.0/1").placed);

    // On desktop, a statement is added before the shake: its key moves on by
    // one, and its text finds it.
    let edited = LAMP.replace(
        "            Camera.shake(0.3);",
        "            Game.lit += 1.0;\n            Camera.shake(0.3);",
    );
    let mut moved = graph_of(&edited);
    layout.place(&mut moved);
    let shake = node(&moved, "Lamp.update/0.0/2");
    assert_eq!(shake.title, "Camera.shake");
    assert_eq!(
        shake.at.map(f32::to_bits),
        [900.0_f32, 40.0].map(f32::to_bits),
        "found by its text"
    );
    assert!(
        !node(&moved, "Lamp.update/0.0/1").placed,
        "the new statement is laid out"
    );

    let text = layout.to_json().expect("writes");
    assert!(!text.contains("script Lamp"));
    assert_eq!(Layout::read(Some(&text)).expect("reads back"), layout);
}

fn graph_of(source: &str) -> Graph {
    graph(&blocks(source), source)
}

#[test]
fn what_a_newer_editor_wrote_is_kept_and_a_newer_file_is_not_overwritten() {
    let text = r#"{ "format_version": 1, "groups": [{"name": "Lights"}],
        "nodes": { "Lamp.update": { "at": [1.0, 2.0], "print": "x", "color": "amber" } } }"#;
    let layout = Layout::read(Some(text)).expect("reads");
    let written = layout.to_json().expect("writes");
    assert!(
        written.contains("\"groups\"") && written.contains("\"color\": \"amber\""),
        "{written}"
    );
    assert!(Layout::read(Some(r#"{ "format_version": 2, "nodes": {} }"#)).is_err());
}

#[test]
fn simple_math_stays_on_its_pin_as_a_formula_and_calls_stay_nodes() {
    let source = "script S {\n    var grace: number = 0.0;\n    fn update(dt: number) {\n        this.grace = max(0.0, this.grace - dt * 2.0);\n    }\n}\n";
    let graph = graph(&blocks(source), source);
    let set = node(&graph, "S.update/0");
    assert!(set.inputs[0].value.is_none(), "the call is wired in");
    let max = node(&graph, "S.update/0:to");
    assert_eq!(max.title, "max");
    assert!(
        matches!(&max.inputs[1].value, Some(Slot::Code { source, .. }) if source == "this.grace - dt * 2.0"),
        "{:?}",
        max.inputs[1]
    );
    assert_eq!(
        graph.nodes.len(),
        4,
        "values, event, set, max: no node per operator"
    );
}
