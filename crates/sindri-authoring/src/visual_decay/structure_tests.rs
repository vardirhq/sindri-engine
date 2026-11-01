//! Changing a script's shape from its nodes.

use std::path::Path;

use super::*;

const SCRIPT: &str = "\
script Lamp {
    @export let glow: number = 1.5;
    var lit: bool = false;

    fn start() {
        let warmth = this.glow * 2.0;
        // Light it.
        this.lit = true;
        this.glow = warmth;
    }

    fn update(dt: number) {
        if this.lit {
        }
    }

    fn flare() {
        this.glow = 3.0;
    }
}
";

fn environment() -> decay_semantic::Environment {
    let mut sources = sindri_decay::ScriptSources::new();
    sources.insert("scripts/lamp.decay", SCRIPT);
    sources.environment().clone()
}

/// The span of the first statement whose text starts with `text`.
fn at(source: &str, text: &str) -> Span {
    let start = source.find(text).unwrap_or_else(|| panic!("no {text}"));
    let end = start
        + source[start..].find(';').map_or_else(
            || source[start..].find('}').expect("an end") + 1,
            |semi| semi + 1,
        );
    [start, end]
}

fn handler(source: &str, name: &str) -> Span {
    blocks(source).containers[0]
        .handlers
        .iter()
        .find(|handler| handler.name == name)
        .expect("the handler")
        .span
}

#[test]
fn deleting_a_statement_takes_its_lines_and_its_comment_and_nothing_else() {
    let changed = change(
        SCRIPT,
        &Shape::Delete(at(SCRIPT, "this.lit = true;")),
        &environment(),
    )
    .expect("deletes");
    assert_eq!(
        changed,
        SCRIPT.replace("        // Light it.\n        this.lit = true;\n", "")
    );
}

#[test]
fn moving_swaps_a_statement_with_its_neighbor_comment_and_all() {
    let changed = change(
        SCRIPT,
        &Shape::Move {
            statement: at(SCRIPT, "this.glow = warmth;"),
            up: true,
        },
        &environment(),
    )
    .expect("moves up");
    assert!(
        changed.contains("        let warmth = this.glow * 2.0;\n        this.glow = warmth;\n        // Light it.\n        this.lit = true;\n"),
        "{changed}"
    );
    let first = at(SCRIPT, "let warmth");
    assert!(
        change(
            SCRIPT,
            &Shape::Move {
                statement: first,
                up: true
            },
            &environment()
        )
        .is_err(),
        "already first"
    );
}

#[test]
fn adding_writes_the_template_at_the_indentation_around_it() {
    let script = blocks(SCRIPT);
    let set = Template::named("set:glow", &script, SCRIPT).expect("a set template");
    let changed = change(
        SCRIPT,
        &Shape::After {
            statement: at(SCRIPT, "this.glow = warmth;"),
            template: set,
        },
        &environment(),
    )
    .expect("adds after");
    assert!(
        changed.contains("        this.glow = warmth;\n        this.glow = 1.5;\n    }"),
        "{changed}"
    );

    let call = Template::named("call:flare", &script, SCRIPT).expect("a call template");
    let update = blocks(SCRIPT).containers[0].handlers[1].body.clone();
    let Block::If { span: branch, .. } = &update[0] else {
        panic!("an if")
    };
    let changed = change(
        SCRIPT,
        &Shape::Inside {
            node: *branch,
            branch: 0,
            template: call,
        },
        &environment(),
    )
    .expect("adds inside the empty branch");
    assert!(
        changed.contains("        if this.lit {\n            flare();\n        }"),
        "{changed}"
    );

    let comment = Template::named("comment", &script, SCRIPT).expect("a comment");
    let changed = change(
        SCRIPT,
        &Shape::Inside {
            node: handler(SCRIPT, "start"),
            branch: 0,
            template: comment,
        },
        &environment(),
    )
    .expect("adds first in a handler");
    assert!(
        changed.contains("    fn start() {\n        // Note\n        let warmth"),
        "{changed}"
    );
    assert!(
        palette(&script)
            .iter()
            .any(|(id, label)| id == "if:lit" && label == "If lit")
    );
}

#[test]
fn a_change_that_would_not_check_is_refused_with_the_reason() {
    let refused = change(
        SCRIPT,
        &Shape::Delete(at(SCRIPT, "let warmth")),
        &environment(),
    )
    .expect_err("warmth is still read below");
    assert!(refused.contains("warmth"), "{refused}");
    let missing = Template::Call {
        function: "nowhere".to_owned(),
    };
    assert!(
        change(
            SCRIPT,
            &Shape::After {
                statement: at(SCRIPT, "this.lit = true;"),
                template: missing
            },
            &environment()
        )
        .is_err()
    );
}

#[test]
fn a_real_script_takes_a_change_against_its_project() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter/assets/scripts");
    let mut sources = sindri_decay::ScriptSources::new();
    for entry in std::fs::read_dir(&root).expect("scripts").flatten() {
        let name = format!("scripts/{}", entry.file_name().to_string_lossy());
        sources.insert(name, std::fs::read_to_string(entry.path()).expect("reads"));
    }
    let source = std::fs::read_to_string(root.join("lantern.decay")).expect("lantern reads");
    let shake = at(&source, "Camera.add_trauma(0.18);");
    let changed = change(
        &source,
        &Shape::Move {
            statement: shake,
            up: true,
        },
        sources.environment(),
    )
    .expect("moves in the real project");
    assert_eq!(changed.len(), source.len(), "a move only reorders lines");
    assert!(
        changed.find("Camera.add_trauma").expect("still there")
            < changed.find("Effects.burst_at").expect("still there")
    );
}
