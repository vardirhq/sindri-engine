//! The whole setup, end to end, with no network: the downloads come from local
//! files through the real downloader, the archive is unpacked by the real
//! `tar`, and the "runner" in it is a small stand-in that answers the way
//! llama.cpp's server does. Skipped where curl, tar or python3 is missing.

use std::fs;
use std::path::{Path, PathBuf};

use super::super::fetch::{Downloader, digest};
use super::super::managed::server_name;
use super::*;

fn scratch(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("sindri-install-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("a scratch directory");
    directory
}

fn have(program: &str, argument: &str) -> bool {
    std::process::Command::new(program)
        .arg(argument)
        .output()
        .is_ok_and(|output| output.status.success())
}

/// A runner that serves `/health` and fixes the two verification cases.
const STAND_IN: &str = r#"#!/usr/bin/env python3
import json, sys
from http.server import BaseHTTPRequestHandler, HTTPServer
port = int(sys.argv[sys.argv.index("--port") + 1])
FIXES = {
    "banner.decay": "script Banner {\n    var banner: Entity = null;\n\n    fn start() {\n        this.banner = World.find(\"Banner\");\n    }\n}\n",
    "lamp.decay": "script Lamp {\n    var level: f32 = 0.0;\n\n    fn update(dt: f32) {\n        this.level = this.level + dt;\n        glow(this.level);\n    }\n\n    fn glow(amount: f32) {\n        let seen = amount;\n    }\n}\n",
}
class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def reply(self, status, body):
        data = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
    def do_GET(self):
        self.reply(200, {"status": "ok"})
    def do_POST(self):
        asked = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        text = asked["messages"][-1]["content"]
        fix = next((f for name, f in FIXES.items() if name in text), "no idea")
        self.reply(200, {"choices": [{"message": {"role": "assistant", "content": "```decay\n" + fix + "```"}}]})
HTTPServer(("127.0.0.1", port), Handler).serve_forever()
"#;

fn asset_for(file: &Path, version: &str) -> Asset {
    Asset {
        id: "qwen2.5-coder:7b".to_owned(),
        version: version.to_owned(),
        platform: "any".to_owned(),
        architecture: "any".to_owned(),
        url: format!("file://{}", file.display()),
        sha256: digest(file).expect("hashed"),
        size: fs::metadata(file).expect("meta").len(),
        display_name: "Test".to_owned(),
        description: "A test file.".to_owned(),
        license: "MIT".to_owned(),
        source: "local".to_owned(),
    }
}

/// Builds a runner archive and a model file; `None` where the tools are absent.
fn published(root: &Path) -> Option<(Asset, Asset)> {
    if !cfg!(unix)
        || Downloader::found() != Some(Downloader::Curl)
        || !have("tar", "--version")
        || !have("python3", "--version")
    {
        return None;
    }
    let bin = root.join("source/build/bin");
    fs::create_dir_all(&bin).expect("made");
    fs::write(bin.join(server_name()), STAND_IN).expect("written");
    let archive = root.join("llama-test-bin.tar.gz");
    let packed = std::process::Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(root.join("source"))
        .arg("build")
        .status()
        .ok()?;
    packed.success().then_some(())?;
    let model = root.join("test-model.gguf");
    fs::write(&model, vec![1_u8; 2048]).expect("written");
    Some((asset_for(&archive, "b1"), asset_for(&model, "m1")))
}

#[test]
fn setup_runs_every_step_and_the_model_is_proved_and_remembered() {
    let root = scratch("full");
    let Some((runtime, model)) = published(&root) else {
        return;
    };
    let home = Home::at(root.join("home"));
    let shared = Shared::default();
    let finished = install(&home, &runtime, &model, &shared).expect("set up");
    assert_eq!(finished.verified, [Feature::DecayRepair]);
    assert_eq!(shared.progress().step, Some(Step::Check));
    assert!(home.server(&runtime).is_some());
    assert!(home.has_model(&model));
    assert!(!home.downloads().join("llama-test-bin.tar.gz").exists());
    let saved = home.load().expect("remembered");
    assert_eq!(
        saved.features_for(&model, &runtime),
        Some(vec![Feature::DecayRepair])
    );
    drop(finished);

    // A second start downloads nothing: the sources are made unreachable, and
    // it still starts.
    let gone = |asset: &Asset| Asset {
        url: format!(
            "file:///nowhere/{}",
            asset.url.rsplit('/').next().unwrap_or_default()
        ),
        ..asset.clone()
    };
    let again = start(&home, &gone(&runtime), &gone(&model), &Shared::default());
    assert!(again.is_ok(), "{:?}", again.err());
}

#[test]
fn a_download_that_does_not_match_is_refused_at_its_step() {
    let root = scratch("corrupt");
    let Some((runtime, model)) = published(&root) else {
        return;
    };
    let wrong = Asset {
        sha256: "0".repeat(64),
        ..model
    };
    let home = Home::at(root.join("home"));
    let error = install(&home, &runtime, &wrong, &Shared::default())
        .err()
        .expect("refused");
    assert_eq!(error.step(), Step::Model);
    assert!(error.friendly().contains("damaged"));
    assert!(!home.has_model(&wrong));
    // The runner that did arrive is kept, so trying again costs only the rest.
    assert!(home.server(&runtime).is_some());
}

#[test]
fn a_stopped_setup_says_it_was_stopped() {
    let root = scratch("stopped");
    let Some((runtime, model)) = published(&root) else {
        return;
    };
    let shared = Shared::default();
    shared.stop();
    let error = install(&Home::at(root.join("home")), &runtime, &model, &shared)
        .err()
        .expect("stopped");
    assert!(error.cancelled(), "{error:?}");
}
