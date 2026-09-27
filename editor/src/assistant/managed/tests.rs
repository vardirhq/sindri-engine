//! What the managed install must guarantee, tested without a network.

use super::*;
use crate::assistant::catalogue;

fn scratch(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("sindri-managed-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("a scratch directory");
    directory
}

#[test]
fn the_runner_is_pinned_for_linux_and_apple_silicon() {
    for (platform, architecture) in [("linux", "x86_64"), ("macos", "arm64")] {
        let asset = runtime_for(platform, architecture).expect("a pinned runner");
        assert_eq!(asset.sha256.len(), 64);
        assert!(
            asset
                .url
                .starts_with("https://github.com/ggml-org/llama.cpp/")
        );
        assert!(asset.size > 0);
    }
    // Said, not guessed: nothing is published for these yet.
    assert!(runtime_for("windows", "x86_64").is_none());
}

/// The model Sindri sets up is one the catalogue grades, with a file whose URL
/// pins a commit rather than a branch, so what is served cannot change under
/// the digest.
#[test]
fn the_model_file_is_pinned_and_the_catalogue_knows_it() {
    let asset = model_file(MODEL).expect("a pinned model file");
    assert_eq!(asset.sha256.len(), 64);
    assert!(asset.url.contains(&format!("/resolve/{}/", asset.version)));
    assert!(asset.size > 1_000_000_000);
    assert!(catalogue::profile_for(MODEL).is_some());
}

#[test]
fn everything_lives_under_one_folder() {
    let home = Home::at("/data/assistant");
    let runtime = runtime_for("linux", "x86_64").expect("pinned");
    let model = model_file(MODEL).expect("pinned");
    for path in [
        home.downloads(),
        home.runtime_dir(&runtime),
        home.model_path(&model),
        home.log(),
    ] {
        assert!(path.starts_with("/data/assistant"), "{}", path.display());
    }
    assert!(home.model_path(&model).to_string_lossy().ends_with(".gguf"));
}

#[test]
fn what_was_proved_survives_a_restart_and_belongs_to_its_files() {
    let home = Home::at(scratch("saved"));
    let runtime = runtime_for("linux", "x86_64").expect("pinned");
    let model = model_file(MODEL).expect("pinned");
    assert_eq!(home.load(), None);
    let mut saved = Saved::answering(&model, &runtime);
    saved.record(&[Feature::DecayRepair], &[Feature::DecayRepair]);
    home.save(&saved).expect("saved");
    let saved = home.load().expect("loaded");
    let known = saved.known_for(&model, &runtime).expect("these files");
    assert_eq!(known.verified, [Feature::DecayRepair]);
    assert_eq!(known.checked, [Feature::DecayRepair]);
    // A different runner build proves nothing about this one.
    let newer = Asset {
        version: "b9999".to_owned(),
        ..runtime
    };
    assert_eq!(saved.known_for(&model, &newer), None);
}

/// A failed test is remembered as tested, so it is not re-run every time the
/// panel opens, and passing later replaces it.
#[test]
fn a_feature_that_did_not_pass_is_remembered_as_tested() {
    let runtime = runtime_for("linux", "x86_64").expect("pinned");
    let model = model_file(MODEL).expect("pinned");
    let mut saved = Saved::answering(&model, &runtime);
    assert_eq!(saved.known_for(&model, &runtime), Some(Known::default()));
    saved.record(&[Feature::DecayRepair], &[]);
    let known = saved.known_for(&model, &runtime).expect("these files");
    assert!(known.verified.is_empty());
    assert_eq!(known.checked, [Feature::DecayRepair]);
    saved.record(&[Feature::DecayRepair], &[Feature::DecayRepair]);
    let known = saved.known_for(&model, &runtime).expect("these files");
    assert_eq!(known.verified, [Feature::DecayRepair]);
    assert_eq!(known.checked, [Feature::DecayRepair]);
}

/// A record written before `checked` existed still reads: what passed was
/// tested.
#[test]
fn an_older_record_reads_its_passes_as_tested() {
    let runtime = runtime_for("linux", "x86_64").expect("pinned");
    let model = model_file(MODEL).expect("pinned");
    let text = format!(
        r#"{{"model_sha256":"{}","runtime_version":"{}","verified":["decay_repair"]}}"#,
        model.sha256, runtime.version
    );
    let saved: Saved = serde_json::from_str(&text).expect("an older record");
    let known = saved.known_for(&model, &runtime).expect("these files");
    assert_eq!(known.checked, [Feature::DecayRepair]);
}

#[test]
fn a_model_file_counts_only_at_its_full_size() {
    let root = scratch("model");
    let home = Home::at(&root);
    let model = Asset {
        size: 4,
        ..model_file(MODEL).expect("pinned")
    };
    assert!(!home.has_model(&model));
    fs::create_dir_all(home.model_path(&model).parent().expect("a parent")).expect("made");
    fs::write(home.model_path(&model), b"abc").expect("written");
    assert!(!home.has_model(&model), "a partial file is not a model");
    fs::write(home.model_path(&model), b"abcd").expect("written");
    assert!(home.has_model(&model));
    assert!(home.footprint() >= 4);
    home.remove().expect("removed");
    assert!(!root.exists());
    home.remove().expect("removing twice is fine");
}

/// A real archive through the real `tar`, shaped like a llama.cpp release: the
/// server nested in a folder with a library beside it.
#[test]
fn an_archive_is_unpacked_with_its_layout_and_the_server_is_found() {
    let root = scratch("unpack");
    let build = root.join("source/build/bin");
    fs::create_dir_all(&build).expect("made");
    fs::write(build.join(server_name()), b"#!/bin/sh\n").expect("written");
    fs::write(build.join("libggml.so"), b"lib").expect("written");
    let archive = root.join("runner.tar.gz");
    let packed = std::process::Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(root.join("source"))
        .arg("build")
        .status();
    if !packed.is_ok_and(|status| status.success()) {
        return; // No tar on this machine; nothing to test against.
    }
    let into = root.join("runtime/b1");
    let server = unpack(&archive, &into).expect("unpacked");
    assert!(server.starts_with(&into));
    assert!(server.with_file_name("libggml.so").is_file());
    assert!(!root.join("runtime/b1.unpacking").exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&server).expect("meta").permissions().mode();
        assert!(mode & 0o100 != 0, "the server is executable");
    }
}

#[test]
fn an_archive_without_a_server_is_refused_and_leaves_nothing() {
    let root = scratch("empty-archive");
    fs::create_dir_all(root.join("source/docs")).expect("made");
    fs::write(root.join("source/docs/readme"), b"hi").expect("written");
    let archive = root.join("runner.tar.gz");
    let packed = std::process::Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(root.join("source"))
        .arg("docs")
        .status();
    if !packed.is_ok_and(|status| status.success()) {
        return;
    }
    let into = root.join("runtime/b1");
    assert!(unpack(&archive, &into).is_err());
    assert!(!into.exists());
    assert!(!root.join("runtime/b1.unpacking").exists());
}
