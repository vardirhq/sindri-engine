//! The `sindri` binary run as a person or an agent runs it. `project check`
//! over every project in this repository: each checks clean, and the JSON is
//! the contract `docs/cli-conventions.md` describes, so a project that breaks,
//! or a check that starts crying wolf, fails here. And `play run` over
//! Lamplighter, read back as an agent reads it.

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

/// `sindri play run` as an agent would use it: walk Lamplighter's Wick east,
/// build a fire, and read both results from the JSON.
#[test]
fn a_headless_run_reports_the_board_and_where_things_went() {
    let lamplighter = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let output = Command::new(env!("CARGO_BIN_EXE_sindri"))
        .args(["play", "run"])
        .arg(&lamplighter)
        .args([
            "--scene",
            "wilds.scene",
            "--track",
            "wick",
            "--format",
            "json",
        ])
        .args([
            "--steps",
            "wait:0.3 hold:D wait:1 release:D set:wood=4 set:stone=2 key:F wait:0.2",
        ])
        .output()
        .expect("sindri runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["scene"], "wilds.scene");
    assert_eq!(report["board"]["built"], 1.0, "a fire was built");
    assert_eq!(report["board"]["wood"], 0.0, "with the wood");
    let x = report["tracked"]["wick"][0].as_f64().unwrap();
    assert!(x > 13.0, "Wick walked east: {x}");
    assert!(report["failures"].as_array().unwrap().is_empty());
}

#[test]
fn a_step_that_does_not_read_is_a_usage_error() {
    let lamplighter = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let output = Command::new(env!("CARGO_BIN_EXE_sindri"))
        .args(["play", "run"])
        .arg(&lamplighter)
        .args(["--steps", "jump:high"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("a step is one of"));
}

/// Whether a run could not take its picture for want of a GPU, which a
/// machine without one may say; CI sets `SINDRI_REQUIRE_GPU` so it may not.
fn no_gpu(report: &serde_json::Value) -> bool {
    let wanting = report["stopped"]
        .as_str()
        .is_some_and(|why| why.contains("needs a GPU adapter"));
    if wanting {
        assert!(
            std::env::var_os("SINDRI_REQUIRE_GPU").is_none(),
            "SINDRI_REQUIRE_GPU is set but a shot found no adapter: {report}"
        );
        eprintln!("skipping the picture: no GPU adapter");
    }
    wanting
}

/// `render capture` writes the picture a player would see, at the size
/// asked, and says where.
#[test]
fn a_capture_is_a_picture_at_the_size_asked() {
    let flappy = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/flappy");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("title.png");
    let output = Command::new(env!("CARGO_BIN_EXE_sindri"))
        .args(["render", "capture"])
        .arg(&flappy)
        .arg("--out")
        .arg(&out)
        .args([
            "--size", "320x240", "--steps", "wait:0.2", "--format", "json",
        ])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    if no_gpu(&report) {
        return;
    }
    assert!(output.status.success(), "{report}");
    assert_eq!(report["shots"][0]["path"], out.display().to_string());
    let png = std::fs::read(&out).expect("the picture is written");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    // Width and height, big-endian, in the IHDR chunk.
    assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 320);
    assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 240);
}

/// The MCP `capture` tool answers with the picture itself, inline.
#[test]
fn the_mcp_capture_tool_answers_with_the_picture() {
    use std::io::Write as _;
    let flappy = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/flappy");
    let mut child = Command::new(env!("CARGO_BIN_EXE_sindri"))
        .arg("mcp")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let call = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "capture", "arguments": {"project": flappy, "width": 160, "height": 120}}});
    writeln!(child.stdin.take().unwrap(), "{call}").unwrap();
    let output = child.wait_with_output().unwrap();
    let reply: serde_json::Value =
        serde_json::from_slice(output.stdout.split(|b| *b == b'\n').next().unwrap()).unwrap();
    let result = &reply["result"];
    if no_gpu(&result["structuredContent"]) {
        return;
    }
    assert_eq!(result["isError"], false, "{result}");
    let image = result["content"]
        .as_array()
        .unwrap()
        .iter()
        .find(|part| part["type"] == "image")
        .expect("an image");
    assert_eq!(image["mimeType"], "image/png");
    assert!(
        image["data"].as_str().unwrap().starts_with("iVBORw0KGgo"),
        "base64 of a PNG"
    );
}
