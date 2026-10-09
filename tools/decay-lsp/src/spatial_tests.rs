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
