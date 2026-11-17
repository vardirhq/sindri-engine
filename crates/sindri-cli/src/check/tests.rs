use std::fs;

use super::*;

/// A project in a temporary folder: a manifest naming `main.scene`, and
/// whatever files a test adds under `assets/`.
fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("sindri.toml"),
        "[project]\nname = \"probe\"\nmain_scene = \"assets/main.scene\"\n",
    )
    .unwrap();
    for (path, text) in files {
        let file = dir.path().join("assets").join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }
    dir
}

fn scene(entities: &str) -> String {
    format!(r#"{{"format_version": 10, "metadata": {{"name": "main"}}, "entities": [{entities}]}}"#)
}

fn codes(report: &CheckReport) -> Vec<&str> {
    report
        .diagnostics
        .iter()
        .map(|found| found.code.as_str())
        .collect()
}

#[test]
fn a_sound_project_checks_clean() {
    let dir = project(&[
        (
            "main.scene",
            &scene(
                r#"{"id": "hero", "name": "hero", "components": {"sindri.script": {"source": "hero.decay", "script": "Hero"}}}"#,
            ),
        ),
        ("hero.decay", "script Hero { fn update(dt: number) { } }"),
        ("ui.weave", ".title { color: #fff; }"),
    ]);
    let report = check(dir.path());
    assert!(report.ok, "{:?}", report.diagnostics);
    assert!(report.diagnostics.is_empty());
    assert_eq!(
        (
            report.checked.scenes,
            report.checked.scripts,
            report.checked.stylesheets
        ),
        (1, 1, 1)
    );
}

#[test]
fn every_problem_is_reported_with_its_code_not_only_the_first() {
    let dir = project(&[
        (
            "main.scene",
            &scene(
                r#"{"id": "hero", "name": "hero", "components": {
                    "sindri.script": {"source": "hero.decay", "script": "Villain"},
                    "sindri.sprite": {"texture": "textures/missing.png"},
                    "sindri.spirte": {}
                }},
                {"id": "orphan", "name": "orphan", "parent": "nobody", "components": {}}"#,
            ),
        ),
        (
            "hero.decay",
            "script Hero { fn update(dt: number) { missing(); } }",
        ),
        ("broken.scene", "{ not json"),
        ("ui.weave", ".title { color #fff; }"),
    ]);
    let report = check(dir.path());
    assert!(!report.ok);
    let found = codes(&report);
    for code in [
        "SCENE_JSON",
        "SCENE_MISSING_PARENT",
        "SCRIPT_UNDECLARED",
        "ASSET_MISSING_TEXTURE",
        "COMPONENT_UNKNOWN",
        "DECAY_UNKNOWN_NAME",
        "WEAVE_MISSING_COLON",
    ] {
        assert!(found.contains(&code), "{code} in {found:?}");
    }
    let typo = report
        .diagnostics
        .iter()
        .find(|found| found.code == "COMPONENT_UNKNOWN")
        .unwrap();
    assert_eq!(typo.severity, Severity::Warning);
    assert!(
        typo.message.contains("did you mean sindri.sprite"),
        "{}",
        typo.message
    );
    assert_eq!(typo.entity.as_deref(), Some("hero"));
    assert_eq!(typo.path, "assets/main.scene");
    let script = report
        .diagnostics
        .iter()
        .find(|found| found.code == "DECAY_UNKNOWN_NAME")
        .unwrap();
    assert_eq!(
        (script.line, script.path.as_str()),
        (Some(1), "assets/hero.decay")
    );
}

#[test]
fn a_missing_main_scene_and_manifest_entries_are_reported() {
    let dir = project(&[]);
    fs::write(
        dir.path().join("sindri.toml"),
        "[project]\nname = \"probe\"\nmain_scene = \"assets/main.scene\"\nscenes = [\"assets/gone.scene\"]\n[assets]\ninclude = [\"ui.weave\"]\n",
    )
    .unwrap();
    let found = check(dir.path());
    let found = codes(&found);
    for code in [
        "PROJECT_MISSING_MAIN_SCENE",
        "PROJECT_MISSING_SCENE",
        "PROJECT_MISSING_STYLESHEET",
    ] {
        assert!(found.contains(&code), "{code} in {found:?}");
    }
}

#[test]
fn the_json_carries_its_schema_version() {
    let dir = project(&[("main.scene", &scene(""))]);
    let json = serde_json::to_value(check(dir.path())).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["ok"], true);
    assert!(json["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn a_misspelled_field_is_named_with_the_one_meant() {
    let dir = project(&[(
        "main.scene",
        &scene(
            r#"{"id": "lamp", "components": {
                "sindri.audio.source#hum": {"clip": "hum.ogg", "volme": 0.5},
                "sindri.camera": {"projection": "orthographic", "vertical_size": 6.0, "near": 0.1, "far": 100.0}
            }}"#,
        ),
    )]);
    let report = check(dir.path());
    assert_eq!(
        codes(&report),
        ["COMPONENT_UNKNOWN_FIELD"],
        "{:?}",
        report.diagnostics
    );
    let found = &report.diagnostics[0];
    assert!(
        found.message.contains("volme") && found.message.contains("did you mean volume"),
        "{}",
        found.message
    );
    assert_eq!(found.entity.as_deref(), Some("lamp"));
    assert!(report.ok, "a warning: the scene still runs");
}

#[test]
fn a_payload_the_engine_cannot_read_says_why_and_on_which_entity() {
    let dir = project(&[(
        "main.scene",
        &scene(
            r#"{"id": "eye", "components": {"sindri.camera": {"projection": "orthographic", "vertical_size": 6.0}}},
               {"id": "ear", "components": {"sindri.audio.source": {"volume": "loud"}}}"#,
        ),
    )]);
    let report = check(dir.path());
    let invalid: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|found| found.code == "COMPONENT_INVALID_PAYLOAD")
        .collect();
    assert_eq!(invalid.len(), 2, "both, not only the first: {invalid:?}");
    let on = |id: &str| {
        invalid
            .iter()
            .find(|found| found.entity.as_deref() == Some(id))
            .unwrap_or_else(|| panic!("nothing on {id}"))
    };
    assert!(on("eye").message.contains("near"), "{}", on("eye").message);
    on("ear");
}
