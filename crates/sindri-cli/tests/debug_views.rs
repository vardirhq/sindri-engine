//! `debug:` overlays and `probe:` steps, run as an agent runs them: the
//! colliders physics has and the laid-out UI, drawn over pictures and asked
//! about one pixel at a time.

use std::path::Path;
use std::process::Command;

mod common;

use common::no_gpu;

#[test]
fn a_probe_says_what_is_at_a_pixel_and_needs_no_gpu() {
    let flappy = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/flappy");
    let output = Command::new(env!("CARGO_BIN_EXE_sindri"))
        .args(["play", "run"])
        .arg(&flappy)
        .args(["--size", "640x360", "--format", "json"])
        .args(["--steps", "wait:0.5 probe:275,158 probe:500,30"])
        .output()
        .expect("sindri runs");
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["probes"][0]["colliders"],
        serde_json::json!(["bird"]),
        "{report}"
    );
    assert_eq!(report["probes"][0]["element"], "title");
    assert_eq!(
        report["probes"][1]["colliders"],
        serde_json::json!(["pipe-a-top"])
    );
    let world = &report["probes"][1]["world"];
    assert!((world[0].as_f64().unwrap() - 6.0).abs() < 0.01, "{world}");
}

#[test]
fn a_debug_shot_draws_the_colliders_physics_has() {
    let flappy = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/flappy");
    let out = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_sindri"))
        .args(["play", "run"])
        .arg(&flappy)
        .args(["--size", "320x180", "--format", "json"])
        .args([
            "--steps",
            "wait:0.5 shot:plain debug:colliders shot:outlined",
        ])
        .arg("--shots")
        .arg(out.path())
        .output()
        .expect("sindri runs");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
    if no_gpu(&report) {
        return;
    }
    assert!(output.status.success(), "{report}");
    // The pipes are sensors, outlined in yellow; the plain shot has none.
    let sensor = |file: &str| {
        let picture = image::open(out.path().join(file)).unwrap().to_rgba8();
        picture
            .pixels()
            .filter(|pixel| pixel.0 == [255, 220, 48, 255])
            .count()
    };
    assert_eq!(sensor("plain.png"), 0);
    assert!(sensor("outlined.png") > 100, "{}", sensor("outlined.png"));
}

#[test]
fn a_tilemap_s_ground_is_a_collider_a_probe_finds() {
    let platformer = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/platformer");
    let output = Command::new(env!("CARGO_BIN_EXE_sindri"))
        .args(["play", "run"])
        .arg(&platformer)
        .args([
            "--scene",
            "level-1.scene",
            "--size",
            "640x360",
            "--format",
            "json",
        ])
        .args(["--steps", "wait:0.3 probe:320,330"])
        .output()
        .expect("sindri runs");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["probes"][0]["colliders"],
        serde_json::json!(["level"]),
        "{report}"
    );
}

#[test]
fn a_debug_shot_draws_3d_colliders_as_wireframes() {
    let physics = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/physics");
    let out = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_sindri"))
        .args(["play", "run"])
        .arg(&physics)
        .args([
            "--scene",
            "quarry.scene",
            "--size",
            "320x180",
            "--format",
            "json",
        ])
        .args(["--steps", "shot:plain debug:colliders shot:wired"])
        .arg("--shots")
        .arg(out.path())
        .output()
        .expect("sindri runs");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
    if no_gpu(&report) {
        return;
    }
    let green = |file: &str| {
        let picture = image::open(out.path().join(file)).unwrap().to_rgba8();
        picture
            .pixels()
            .filter(|pixel| pixel.0 == [64, 255, 96, 255])
            .count()
    };
    assert_eq!(green("plain.png"), 0);
    assert!(green("wired.png") > 200, "{}", green("wired.png"));
}
