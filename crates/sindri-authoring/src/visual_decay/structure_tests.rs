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
        arguments: Vec::new(),
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

#[test]
fn a_pin_takes_a_field_in_place_of_its_value_and_an_unknown_one_is_refused() {
    let start = SCRIPT.find("3.0").expect("the literal");
    let slot = [start, start + 3];
    let changed = change(
        SCRIPT,
        &Shape::Field {
            slot,
            field: "glow".to_owned(),
        },
        &environment(),
    )
    .expect("glow is a field");
    assert_eq!(
        changed,
        SCRIPT.replace("this.glow = 3.0;", "this.glow = this.glow;")
    );

    for field in ["warmth", "glow; this.lit = true"] {
        assert!(
            change(
                SCRIPT,
                &Shape::Field {
                    slot,
                    field: field.to_owned(),
                },
                &environment(),
            )
            .is_err(),
            "{field} is not a field"
        );
    }
}

#[test]
fn a_step_can_call_the_engine_from_the_finder_and_still_reads() {
    let environment = environment();
    let calls = engine_calls(&environment);
    let (id, call) = calls
        .iter()
        .find(|(id, _)| id == "engine:Audio.play")
        .expect("Audio.play takes plain arguments");
    assert_eq!(call, "Audio.play(\"\", 0)");
    let template = Template::engine(id, &environment).expect("named by its id");
    let changed = change(
        SCRIPT,
        &Shape::After {
            statement: at(SCRIPT, "this.glow = 3.0"),
            template,
        },
        &environment,
    )
    .expect("an engine call with placeholders still checks");
    assert!(changed.contains("this.glow = 3.0;\n        Audio.play(\"\", 0);\n"));
    let despawn = calls
        .iter()
        .find(|(id, _)| id == "engine:World.despawn")
        .expect("entity calls are available");
    assert_eq!(despawn.1, "World.despawn(this.entity)");
    let template = Template::engine(&despawn.0, &environment).expect("despawn template");
    let changed = change(
        SCRIPT,
        &Shape::After {
            statement: at(SCRIPT, "this.glow = 3.0"),
            template,
        },
        &environment,
    )
    .expect("despawn with its entity checks");
    assert!(changed.contains("World.despawn(this.entity);"));
    for (id, expected) in [
        (
            "engine:Physics.apply_force",
            "Physics.apply_force(this.entity, Vec2(0, 0));",
        ),
        (
            "engine:World.nearest",
            "World.nearest(\"\", Vec3(0, 0, 0));",
        ),
    ] {
        let template = Template::engine(id, &environment).expect("vector call is available");
        let changed = change(
            SCRIPT,
            &Shape::After {
                statement: at(SCRIPT, "this.glow = 3.0"),
                template,
            },
            &environment,
        )
        .expect("vector placeholder checks");
        assert!(changed.contains(expected));
    }
}

#[test]
fn a_script_gains_variables_functions_and_events_where_its_code_keeps_them() {
    let add = |member: Member| change(SCRIPT, &Shape::Add(member), &environment());
    let variable = add(Member::Variable {
        name: "speed".to_owned(),
        ty: "number".to_owned(),
    })
    .expect("a new number");
    assert!(
        variable.starts_with("script Lamp {\n    var speed: number = 0;\n    @export let glow")
    );

    let function = add(Member::Function {
        name: "dim".to_owned(),
        inputs: Vec::new(),
    })
    .expect("a new function");
    assert!(function.ends_with("        this.glow = 3.0;\n    }\n\n    fn dim() {\n    }\n}\n"));

    // The lamp already has both events, so a copy of one is refused, as is a bad name.
    for member in [
        Member::Event {
            name: "update".to_owned(),
        },
        Member::Function {
            name: "2fast".to_owned(),
            inputs: Vec::new(),
        },
        Member::Variable {
            name: "glow".to_owned(),
            ty: "number".to_owned(),
        },
    ] {
        assert!(add(member.clone()).is_err(), "{member:?} is refused");
    }
    let bare = "script Bare {\n}\n";
    let mut sources = sindri_decay::ScriptSources::new();
    sources.insert("scripts/bare.decay", bare);
    let event = change(
        bare,
        &Shape::Add(Member::Event {
            name: "update".to_owned(),
        }),
        sources.environment(),
    )
    .expect("a script with no events gains one");
    assert_eq!(
        event,
        "script Bare {\n    fn update(dt: number) {\n    }\n}\n"
    );
}

#[test]
fn functions_take_inputs_steps_remember_values_and_members_rename_or_go() {
    let environment = environment();
    let with_inputs = change(
        SCRIPT,
        &Shape::Add(Member::Function {
            name: "dim".to_owned(),
            inputs: vec![
                ("by".to_owned(), "number".to_owned()),
                ("why".to_owned(), "String".to_owned()),
            ],
        }),
        &environment,
    )
    .expect("a function with inputs");
    assert!(with_inputs.contains("    fn dim(by: number, why: String) {\n    }\n"));
    let call = Template::named("call:dim", &blocks(&with_inputs), &with_inputs).expect("callable");
    assert_eq!(
        call,
        Template::Call {
            function: "dim".to_owned(),
            arguments: vec!["0".to_owned(), "\"\"".to_owned()],
        }
    );

    let remembered = change(
        SCRIPT,
        &Shape::After {
            statement: at(SCRIPT, "this.glow = 3.0"),
            template: Template::named("let:boost", &blocks(SCRIPT), SCRIPT).expect("a let"),
        },
        &environment,
    )
    .expect("a remembered value checks");
    assert!(remembered.contains("this.glow = 3.0;\n        let boost = 0;\n"));

    let renamed = change(
        SCRIPT,
        &Shape::Rename {
            from: "lit".to_owned(),
            to: "burning".to_owned(),
        },
        &environment,
    )
    .expect("lit renames");
    assert_eq!(renamed, SCRIPT.replace("lit", "burning"));
    assert!(
        change(
            SCRIPT,
            &Shape::Rename {
                from: "glow".to_owned(),
                to: "shine".to_owned(),
            },
            &environment,
        )
        .is_err(),
        "an inspector field keeps its name"
    );

    let removed = change(
        SCRIPT,
        &Shape::Remove {
            name: "flare".to_owned(),
        },
        &environment,
    )
    .expect("flare is used nowhere");
    assert!(!removed.contains("flare"));
    assert!(
        change(
            SCRIPT,
            &Shape::Remove {
                name: "lit".to_owned(),
            },
            &environment,
        )
        .is_err(),
        "lit is still read"
    );
}
