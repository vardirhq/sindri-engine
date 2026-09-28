//! The first question: does the project load and do its scripts compile?

use orbital_last_stand::Run;

#[test]
fn the_project_opens_and_every_script_compiles() {
    let mut run = Run::open().expect("the project opens");
    let failures = run
        .scripts
        .compile(&run.world, &run.components, &run.sources);
    let reported: Vec<String> = failures.iter().map(ToString::to_string).collect();
    assert!(reported.is_empty(), "{reported:#?}");
}

#[test]
fn a_first_frame_reports_nothing() {
    let mut run = Run::open().expect("the project opens");
    let notes = run.step(1.0 / 60.0);
    assert!(notes.is_empty(), "{notes:#?}");
}

/// Every label has to be a sensible share of the screen.
///
/// A screen element's position, scale and font size are all in the overlay's
/// units — two tall, centred on the origin. They were not always: a font size
/// used to be pixels, and a scene authored consistently in overlay units drew
/// its buttons and none of its words, because an eighth of a pixel is a
/// positive number. The unit is one now, and this is what keeps it one.
#[test]
fn every_label_is_a_sensible_share_of_the_screen() {
    let text =
        std::fs::read_to_string(orbital_last_stand::project().join("assets/orbital.scene.json"))
            .expect("the scene reads");
    let scene: serde_json::Value = serde_json::from_str(&text).expect("the scene parses");
    let entities = scene["entities"].as_array().expect("entities");

    let mut labels = 0;
    for entity in entities {
        let Some(label) = entity["components"].get("sindri.ui.text") else {
            continue;
        };
        labels += 1;
        let id = entity["id"].as_str().unwrap_or("?");
        for field in ["font_size", "line_height"] {
            let value = label[field].as_f64().unwrap_or(0.0);
            // Two is the whole height of the screen. Anything past it is
            // somebody typing a pixel count out of habit.
            assert!(
                value <= 2.0,
                "{id}: {field} is {value}, which is taller than the screen — \
                 this is in overlay units, not pixels"
            );
            // A hundredth of the screen is the smallest a person could read.
            assert!(
                value >= 0.02,
                "{id}: {field} is {value}, too small a share of the screen to read"
            );
        }
    }
    assert!(
        labels > 20,
        "only {labels} labels; the scene lost most of them"
    );
}

/// Every script in the project, not only the ones the scene names.
///
/// The test above compiles what the opening scene runs, and a script used only
/// by prefabs — `enemy-mark.decay`, on the marks spawned over enemies — failed
/// to compile for weeks without it noticing: it set `Shape.dash_duty`, which
/// scripts could not reach. The editor reported it; nothing here did.
#[test]
fn every_script_in_the_project_compiles() {
    let scripts = orbital_last_stand::project().join("assets/scripts");
    let mut sources = sindri_decay::ScriptSources::new();
    let mut files: Vec<_> = std::fs::read_dir(&scripts)
        .expect("the scripts directory reads")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "decay")
        })
        .collect();
    files.sort();
    for path in &files {
        let text = std::fs::read_to_string(path).expect("a script reads");
        sources.insert(
            format!("scripts/{}", path.file_name().unwrap().to_string_lossy()),
            text,
        );
    }
    let mut failures = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path).expect("a script reads");
        let check = sindri_decay::check_source_in(&text, sources.environment());
        for diagnostic in check.diagnostics {
            failures.push(format!(
                "{}:{}:{}: {}",
                path.file_name().unwrap().to_string_lossy(),
                diagnostic.line,
                diagnostic.column,
                diagnostic.message
            ));
        }
    }
    assert!(files.len() > 30, "only {} scripts found", files.len());
    assert!(failures.is_empty(), "{failures:#?}");
}
