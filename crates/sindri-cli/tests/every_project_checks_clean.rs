//! The `sindri` binary run as a person or an agent runs it, over every
//! project in this repository: each checks clean, and the JSON is the
//! contract `docs/cli-conventions.md` describes. A project that breaks, or a
//! check that starts crying wolf, fails here.

use std::path::{Path, PathBuf};
use std::process::Command;

fn projects() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    for parent in ["game", "games", "examples"] {
        let parent = root.join(parent);
        if parent.join("sindri.toml").is_file() {
            found.push(parent.clone());
        }
        for entry in std::fs::read_dir(&parent).unwrap().flatten() {
            if entry.path().join("sindri.toml").is_file() {
                found.push(entry.path());
            }
        }
    }
    found.sort();
    found
}

fn check(project: &Path) -> (bool, serde_json::Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_sindri"))
        .args(["project", "check"])
        .arg(project)
        .args(["--format", "json"])
        .output()
        .expect("sindri runs");
    let report = serde_json::from_slice(&output.stdout).expect("the output is JSON");
    (output.status.success(), report)
}

#[test]
fn every_project_in_the_repository_checks_clean() {
    let projects = projects();
    assert!(projects.len() >= 20, "found {projects:?}");
    for project in projects {
        let (passed, report) = check(&project);
        assert_eq!(report["schema_version"], 1);
        assert!(
            passed && report["diagnostics"].as_array().is_some_and(Vec::is_empty),
            "{} does not check clean: {:#}",
            project.display(),
            report["diagnostics"]
        );
    }
}

#[test]
fn a_broken_project_fails_with_exit_code_one_and_says_why() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("sindri.toml"),
        "[project]\nname = \"broken\"\nmain_scene = \"assets/main.scene\"\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_sindri"))
        .args(["project", "check"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("error[PROJECT_MISSING_MAIN_SCENE]"), "{text}");
    assert!(text.trim_end().ends_with("1 errors, 0 warnings"), "{text}");
}

#[test]
fn a_folder_that_is_not_a_project_is_a_usage_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_sindri"))
        .args(["project", "check", "/"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}
