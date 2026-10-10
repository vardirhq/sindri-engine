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
fn pointer_delta_completion_and_hover_are_read_only_in_both_namespaces() {
    for namespace in ["Pointer", "Input.Pointer"] {
        let source =
            format!("script Test {{ fn update(dt: f32) {{ let motion = {namespace}.delta; }} }}");
        let uri = "file:///tmp/pointer-delta.decay";
        let mut server = Server::new();
        server.documents.insert(
            uri.into(),
            Document {
                text: source.clone(),
                version: 1,
            },
        );
        let offset = source.find("delta;").unwrap();
        let params = json!({"textDocument":{"uri":uri}, "position":{"line":0,"character":offset}});
        let Value::Array(items) = server.completion(&params) else {
            panic!("completion array");
        };
        let delta = items.iter().find(|item| item["label"] == "delta").unwrap();
        assert_eq!(delta["detail"], "Vec2 (read-only)");
        let hover = server.hover(&params);
        let text = hover["contents"]["value"].as_str().unwrap();
        assert!(text.contains("Read-only"), "{text}");
        assert!(text.contains("Vec2"), "{text}");
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
