//! A project's brand on its browser loading screen: written beside the page
//! when it is described well, and refused with a reason when it is not.

use std::path::{Path, PathBuf};

use sindri_export::{ExportError, ProjectExport};

/// A copy of the Shapes Lab with `splash` appended to its `sindri.toml`.
fn project_with(splash: &str, name: &str) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/shapes-lab");
    let root = std::env::temp_dir().join(format!("sindri-splash-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    copy(&source, &root);
    std::fs::write(root.join("brand.png"), [137, 80, 78, 71]).expect("the image writes");
    let toml = std::fs::read_to_string(root.join("sindri.toml")).expect("the project reads");
    std::fs::write(root.join("sindri.toml"), format!("{toml}\n{splash}\n")).expect("it writes");
    root
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a directory");
    for entry in std::fs::read_dir(from).expect("the project lists") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("a file copies");
        }
    }
}

fn refused(splash: &str, name: &str) -> String {
    match ProjectExport::gather(&project_with(splash, name)) {
        Err(ExportError::Project(message)) => message,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_described_brand_is_written_beside_the_page() {
    let project = project_with(
        "[web.splash]\nimage = \"brand.png\"\ntitle = \"Vardir\"\nbackground = \"#101820\"",
        "good",
    );
    let export = ProjectExport::gather(&project).expect("the project gathers");
    let out = std::env::temp_dir().join(format!("sindri-splash-out-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    export.write(&out, "/").expect("the export writes");
    let file = export
        .splash
        .as_ref()
        .and_then(sindri_export::Splash::image_file)
        .expect("a file");
    assert_eq!(
        std::fs::read(out.join(&file)).expect("the image is there"),
        [137, 80, 78, 71]
    );
    let page = std::fs::read_to_string(out.join("index.html")).expect("the page is there");
    assert!(page.contains(&format!(r#"<img src="{file}""#)), "{page}");
    assert!(
        page.contains(r#"data-brand-seconds="1.5""#),
        "the default time: {page}"
    );
}

#[test]
fn every_mistake_in_a_brand_is_refused_with_its_reason() {
    assert!(
        refused(
            "[web.splash]\ntitle = \"x\"\nbackground = \"teal\"",
            "colour"
        )
        .contains("\"teal\" is not a `#rrggbb` colour")
    );
    assert!(
        refused("[web.splash]\ntitle = \"x\"\nseconds = 60.0", "long")
            .contains("a splash shows for 0 to 10 seconds")
    );
    assert!(
        refused("[web.splash]\nimage = \"sindri.toml\"", "kind")
            .contains("is not a png, jpg, webp, svg or gif")
    );
    assert!(
        refused("[web.splash]\ncaption = \"x\"", "empty")
            .contains("needs an image, a title, or both")
    );
    assert!(refused("[web.splash]\ntitel = \"x\"", "typo").contains("titel"));
}
