//! The app's design screen for Decay nodes draws a real script's graph, read
//! by the same code the phone runs, from a fixture this test keeps current.
//!
//! Regenerate it after changing how scripts read as nodes:
//!
//! ```bash
//! SINDRI_UPDATE_FIXTURES=1 cargo test -p sindri-mobile --test decay_fixture
//! ```

use std::path::Path;

use sindri_authoring::visual_decay;

#[test]
fn the_lamp_script_fixture_is_what_the_phone_reads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source =
        std::fs::read_to_string(root.join("games/lamplighter/assets/scripts/lantern.decay"))
            .expect("the lamp post's script reads");
    let blocks = visual_decay::blocks(&source);
    let graph = visual_decay::graph_flowing(&blocks, &source, visual_decay::Flow::Down);
    // As `script()` answers it on the phone, with no layout stored.
    let mut json = serde_json::to_string_pretty(&serde_json::json!({
        "name": "scripts/lantern.decay",
        "source": source,
        "blocks": blocks,
        "graph": graph,
        "palette": visual_decay::palette(&blocks)
            .into_iter()
            .map(|(id, label)| serde_json::json!({ "id": id, "label": label }))
            .collect::<Vec<_>>(),
    }))
    .expect("writes");
    json.push('\n');
    let fixture = root.join("mobile/android/app/src/test/resources/decay/lantern.json");
    if std::env::var_os("SINDRI_UPDATE_FIXTURES").is_some() {
        std::fs::write(&fixture, &json).expect("the fixture writes");
    }
    let kept = std::fs::read_to_string(&fixture).unwrap_or_default();
    assert!(
        kept == json,
        "{} is out of date; regenerate it with SINDRI_UPDATE_FIXTURES=1",
        fixture.display()
    );
}
