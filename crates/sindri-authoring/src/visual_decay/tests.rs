//! Decay, but visual, on every script in the repository and on small ones.

use std::path::{Path, PathBuf};

use super::*;

/// Every `.decay` file under the games and examples.
fn every_script() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    for top in ["games", "examples"] {
        walk(&root.join(top), &mut found);
    }
    found.sort();
    found
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "target") {
                walk(&path, found);
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension == "decay")
        {
            found.push(path);
        }
    }
}

/// Every block and slot of `script`, and what each spans.
fn slot_spans(slot: &Slot, out: &mut Vec<(String, Span)>) {
    out.push((format!("{slot:?}"), slot.span()));
    match slot {
        Slot::Call { args, .. } => args.iter().for_each(|arg| slot_spans(arg, out)),
        Slot::Op { left, right, .. } => {
            slot_spans(left, out);
            slot_spans(right, out);
        }
        Slot::Unary { operand, .. } => slot_spans(operand, out),
        _ => {}
    }
}
fn block_spans(blocks: &[Block], out: &mut Vec<(String, Span)>) {
    for block in blocks {
        match block {
            Block::Set {
                target,
                value,
                span,
            }
            | Block::Change {
                target,
                value,
                span,
                ..
            } => {
                out.push(("set".to_owned(), *span));
                slot_spans(target, out);
                slot_spans(value, out);
            }
            Block::Let { value, span, .. } | Block::Return { value, span } => {
                out.push(("let".to_owned(), *span));
                if let Some(value) = value {
                    slot_spans(value, out);
                }
            }
            Block::Do { call, span } => {
                out.push(("do".to_owned(), *span));
                slot_spans(call, out);
            }
            Block::If {
                condition,
                then,
                otherwise,
                span,
            } => {
                out.push(("if".to_owned(), *span));
                slot_spans(condition, out);
                block_spans(then, out);
                if let Some(otherwise) = otherwise {
                    block_spans(otherwise, out);
                }
            }
            Block::While {
                condition,
                body,
                span,
            } => {
                out.push(("while".to_owned(), *span));
                slot_spans(condition, out);
                block_spans(body, out);
            }
            Block::For {
                items, body, span, ..
            } => {
                out.push(("for".to_owned(), *span));
                slot_spans(items, out);
                block_spans(body, out);
            }
            Block::Match {
                subject,
                arms,
                span,
            } => {
                out.push(("match".to_owned(), *span));
                slot_spans(subject, out);
                for arm in arms {
                    block_spans(&arm.body, out);
                }
            }
            Block::Break { span } | Block::Continue { span } => {
                out.push(("jump".to_owned(), *span));
            }
            Block::Code { span, .. } => out.push(("code".to_owned(), *span)),
            Block::Comment { span, .. } => out.push(("comment".to_owned(), *span)),
        }
    }
}

fn spans(script: &VisualScript) -> Vec<(String, Span)> {
    let mut out = Vec::new();
    for container in &script.containers {
        for field in &container.fields {
            field
                .value
                .iter()
                .for_each(|value| slot_spans(value, &mut out));
        }
        for handler in &container.handlers {
            out.push(("handler".to_owned(), handler.span));
            block_spans(&handler.body, &mut out);
        }
    }
    out
}

#[test]
fn every_script_in_the_repository_reads_as_blocks_that_point_into_it() {
    let scripts = every_script();
    assert!(scripts.len() > 50, "found {} scripts", scripts.len());
    let (mut statements, mut code) = (0_usize, 0_usize);
    for path in &scripts {
        let source = std::fs::read_to_string(path).expect("the script reads");
        let script = blocks(&source);
        assert!(
            script.problems.is_empty(),
            "{}: {:?}",
            path.display(),
            script.problems
        );
        for (what, [start, end]) in spans(&script) {
            assert!(
                start <= end && source.get(start..end).is_some(),
                "{}: {what} spans {start}..{end}, outside the script",
                path.display()
            );
            if what != "handler" {
                statements += 1;
            }
            if what == "code" {
                code += 1;
            }
        }
    }
    // Most of what is written has a block of its own; the rest is shown as
    // its source, never dropped.
    assert!(
        code * 20 < statements,
        "{code} of {statements} statements have no block of their own"
    );
}

#[test]
fn a_script_reads_as_handlers_fields_comments_and_blocks() {
    let source = "\
script Lamp {
    // How bright it gets.
    @export let brightness: number = 1.5;
    var lit: bool = false;

    fn start() {
        // Dark until touched.
        this.lit = false;
    }

    fn update(dt: number) {
        if this.lit && Game.night {
            this.brightness += dt * 0.5;
        } else if Game.day {
            this.lit = false;
        } else {
            World.emit(\"dim\");
        }
        let fade = -1.0;
    }
}
";
    let script = blocks(source);
    assert!(script.problems.is_empty(), "{:?}", script.problems);
    let lamp = &script.containers[0];
    assert_eq!(lamp.name, "Lamp");
    assert_eq!(lamp.fields[0].note.as_deref(), Some("How bright it gets."));
    assert!(lamp.fields[0].exported && !lamp.fields[1].exported);
    assert!(
        matches!(lamp.fields[0].value, Some(Slot::Number { value, .. }) if (value - 1.5).abs() < 1e-9)
    );
    assert_eq!(lamp.handlers[0].label, "When it starts");
    assert_eq!(lamp.handlers[1].label, "Every frame");
    assert!(
        matches!(&lamp.handlers[0].body[0], Block::Comment { text, .. } if text == "Dark until touched.")
    );
    assert!(
        matches!(&lamp.handlers[0].body[1], Block::Set { target: Slot::Name { path, .. }, value: Slot::Bool { value: false, .. }, .. } if path == "this.lit")
    );
    let Block::If {
        condition,
        then,
        otherwise,
        ..
    } = &lamp.handlers[1].body[0]
    else {
        panic!("an if block: {:?}", lamp.handlers[1].body[0]);
    };
    assert!(matches!(condition, Slot::Op { op: "&&", .. }));
    assert!(matches!(&then[0], Block::Change { op: "+=", .. }));
    let otherwise = otherwise.as_ref().expect("an else");
    assert_eq!(otherwise.len(), 1, "else if is one if block: {otherwise:?}");
    assert!(
        matches!(&otherwise[0], Block::If { otherwise: Some(last), .. } if matches!(&last[0], Block::Do { .. }))
    );
    assert!(
        matches!(&lamp.handlers[1].body[1], Block::Let { value: Some(Slot::Number { value, .. }), .. } if (value + 1.0).abs() < 1e-9)
    );
}

#[test]
fn a_block_edit_changes_only_its_slot_and_the_rest_stays_byte_for_byte() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../games/lamplighter/assets/scripts/wick.decay");
    let source = std::fs::read_to_string(path).expect("wick reads");
    let script = blocks(&source);
    let speed = script.containers[0]
        .fields
        .iter()
        .find(|field| field.name == "speed")
        .expect("Wick has a speed");
    let Some(Slot::Number { span, .. }) = speed.value else {
        panic!("speed is a number: {:?}", speed.value);
    };
    let edited = edit(&source, &ScriptEdit::Number { span, value: 6.5 }).expect("the edit applies");
    assert_eq!(
        &edited[..span[0]],
        &source[..span[0]],
        "everything before is untouched"
    );
    assert_eq!(
        &edited[span[0] + 3..],
        &source[span[1]..],
        "everything after is untouched"
    );
    assert_eq!(&edited[span[0]..span[0] + 3], "6.5");
    let reread = blocks(&edited);
    assert!(reread.problems.is_empty());
    assert!(matches!(
        reread.containers[0].fields.iter().find(|field| field.name == "speed").and_then(|field| field.value.clone()),
        Some(Slot::Number { value, .. }) if (value - 6.5).abs() < 1e-9
    ));
    // The same value again writes the same bytes: 5.0 keeps its decimal.
    assert_eq!(
        edit(&source, &ScriptEdit::Number { span, value: 5.0 }).expect("a no-op applies"),
        source
    );
}

#[test]
fn an_edit_is_refused_where_the_slot_is_not_what_it_was_or_would_not_read() {
    let source =
        "script S { let a: number = 2; let t: String = \"hi\"; fn start() { this.a = -3; } }";
    let at = |needle: &str| {
        let start = source.find(needle).expect("in the script");
        [start, start + needle.len()]
    };
    let number = at("2");
    assert!(
        edit(
            source,
            &ScriptEdit::Text {
                span: number,
                value: "x".to_owned()
            }
        )
        .is_err(),
        "not text"
    );
    assert!(
        edit(
            source,
            &ScriptEdit::Number {
                span: [0, 6],
                value: 1.0
            }
        )
        .is_err(),
        "not a number"
    );
    assert!(
        edit(
            source,
            &ScriptEdit::Number {
                span: number,
                value: f64::NAN
            }
        )
        .is_err()
    );
    assert!(
        edit(
            source,
            &ScriptEdit::Number {
                span: [500, 501],
                value: 1.0
            }
        )
        .is_err()
    );
    assert!(
        edit(
            "script S { fn start() { this.a = ; } }",
            &ScriptEdit::Bool {
                span: [0, 1],
                value: true
            }
        )
        .is_err(),
        "a broken script is fixed as text first"
    );

    let text = at("\"hi\"");
    let quoted = edit(
        source,
        &ScriptEdit::Text {
            span: text,
            value: "say \"hi\"".to_owned(),
        },
    )
    .expect("text with quotes applies");
    assert!(quoted.contains(r#""say \"hi\"""#), "{quoted}");
    let Some(Slot::Text { value, .. }) = blocks(&quoted).containers[0].fields[1].value.clone()
    else {
        panic!("still text");
    };
    assert_eq!(value, "say \"hi\"");

    let negative = blocks(source).containers[0].handlers[0].body[0].clone();
    let Block::Set {
        value: Slot::Number { span, value },
        ..
    } = negative
    else {
        panic!("a negative number slot: {negative:?}");
    };
    assert!((value + 3.0).abs() < 1e-9);
    let flipped = edit(source, &ScriptEdit::Number { span, value: 4.0 })
        .expect("a negative becomes positive");
    assert!(flipped.contains("this.a = 4;"), "{flipped}");
}
