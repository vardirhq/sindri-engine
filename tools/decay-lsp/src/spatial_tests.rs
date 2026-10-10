use super::*;

#[test]
fn completion_hover_and_diagnostics_agree_on_read_only_directions() {
    let source =
        "script Test { fn update(dt: f32) { this.transform.forward = Vec3(1.0, 0.0, 0.0); } }";
    let uri = "file:///tmp/spatial.decay";
    let mut server = Server::new();
    server.documents.insert(
        uri.into(),
        Document {
            text: source.into(),
            version: 1,
        },
    );
    let offset = source.find("forward").unwrap();
    let params = json!({"textDocument":{"uri":uri}, "position":{"line":0,"character":offset}});
    let Value::Array(items) = server.completion(&params) else {
        panic!("completion array");
    };
    let forward = items
        .iter()
        .find(|item| item["label"] == "forward")
        .unwrap();
    assert_eq!(forward["detail"], "Vec3 (read-only)");
    assert!(items.iter().any(|item| item["label"] == "look_at"));
    assert!(
        server.hover(&params)["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("Read-only")
    );
    let checked = sindri_decay::check_source(source);
    assert_eq!(checked.diagnostics.len(), 1, "{checked:?}");
    assert_eq!(checked.diagnostics[0].code, "immutable");
}

#[test]
fn pointer_motion_and_capture_metadata_are_read_only_in_both_namespaces() {
    for (namespace, member, expected) in [
        ("Pointer", "delta", "Vec2"),
        ("Input.Pointer", "delta", "Vec2"),
        ("Pointer", "locked", "bool"),
        ("Input.Pointer", "locked", "bool"),
    ] {
        let source =
            format!("script Test {{ fn update(dt: f32) {{ let value = {namespace}.{member}; }} }}");
        let uri = "file:///tmp/pointer.decay";
        let mut server = Server::new();
        server.documents.insert(
            uri.into(),
            Document {
                text: source.clone(),
                version: 1,
            },
        );
        let offset = source.find(&format!("{member};")).unwrap();
        let params = json!({"textDocument":{"uri":uri}, "position":{"line":0,"character":offset}});
        let Value::Array(items) = server.completion(&params) else {
            panic!("completion array");
        };
        let entry = items.iter().find(|item| item["label"] == member).unwrap();
        assert_eq!(entry["detail"], format!("{expected} (read-only)"));
        let hover = server.hover(&params);
        let text = hover["contents"]["value"].as_str().unwrap();
        assert!(
            text.contains("Read-only") && text.contains(expected),
            "{text}"
        );
        assert!(items.iter().any(|item| item["label"] == "lock"));
        assert!(items.iter().any(|item| item["label"] == "unlock"));
    }
}

#[test]
fn gathered_input_namespaces_complete_their_own_members() {
    for (group, member) in [("Keyboard", "axis"), ("Touch", "count"), ("Action", "held")] {
        let source = format!("script Test {{ fn update(dt: f32) {{ Input.{group}. }} }}");
        let uri = "file:///tmp/gathered-input.decay";
        let mut server = Server::new();
        server.documents.insert(
            uri.into(),
            Document {
                text: source.clone(),
                version: 1,
            },
        );
        let offset = source.find(". }").unwrap() + 1;
        let params = json!({"textDocument":{"uri":uri}, "position":{"line":0,"character":offset}});
        let Value::Array(items) = server.completion(&params) else {
            panic!("completion array");
        };
        assert!(
            items.iter().any(|item| item["label"] == member),
            "{group}: {items:?}"
        );
    }
}

#[test]
fn character3d_calls_complete_hover_and_check_typed_results() {
    let source = "script Test { fn update(dt: f32) { Physics3d.move_character(this.entity, Vec3(1.0, 0.0, 0.0), false); } }";
    let uri = "file:///tmp/character3d.decay";
    let mut server = Server::new();
    server.documents.insert(
        uri.into(),
        Document {
            text: source.into(),
            version: 1,
        },
    );
    let offset = source.find("move_character").unwrap();
    let params = json!({"textDocument":{"uri":uri}, "position":{"line":0,"character":offset}});
    let Value::Array(items) = server.completion(&params) else {
        panic!("completion array")
    };
    assert!(items.iter().any(|item| item["label"] == "move_character"));
    assert!(items.iter().any(|item| item["label"] == "character_motion"));
    let hover = server.hover(&params);
    let text = hover["contents"]["value"].as_str().unwrap();
    assert!(
        text.contains("Entity, Vec3, bool") && text.contains("unit"),
        "{text}"
    );
    assert!(sindri_decay::check_source(source).diagnostics.is_empty());
    let wrong = source.replace("Vec3(1.0, 0.0, 0.0)", "Vec2(1.0, 0.0)");
    assert!(!sindri_decay::check_source(&wrong).diagnostics.is_empty());
    let observe =
        include_str!("../../../crates/sindri-decay/tests/character_controllers3d/observe.decay");
    let checked = sindri_decay::check_source(observe);
    assert!(checked.diagnostics.is_empty(), "{checked:?}");
    let wrong = observe.replace(
        "total = result.translation.x;",
        "total = result.movement.grounded;",
    );
    assert!(!sindri_decay::check_source(&wrong).diagnostics.is_empty());
}
