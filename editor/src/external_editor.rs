//! Opening source in a tool outside Sindri.
//!
//! Kept behind one small boundary: the project browser asks to edit a file,
//! while this module knows the command-line dialect of the chosen editor.
//! VS Code is the first supported tool, not an assumption spread through the
//! editor shell.

use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

const CODE_CANDIDATES: &[&str] = &["code", "code-insiders", "codium"];
const DECAY_EXTENSION_ID: &str = "vardir.sindri-decay";
const DECAY_VSIX_NAME: &str = "sindri-decay.vsix";
const DECAY_EXTENSION_DIR: &str = "editors/vscode-decay";

#[derive(Debug, Error)]
pub enum ExternalEditorError {
    #[error("VS Code was not found; install its `code` command or set SINDRI_VSCODE")]
    NotFound,
    #[error("Sindri Decay VS Code support could not be located or built")]
    DecayVsixNotFound,
    #[error("could not build Sindri Decay VS Code support; `{command}` exited with {code:?}")]
    ExtensionBuild {
        command: &'static str,
        code: Option<i32>,
    },
    #[error("VS Code rejected the Sindri Decay extension (exit code {0:?})")]
    ExtensionInstall(Option<i32>),
    #[error("could not open the external editor: {0}")]
    Launch(#[from] std::io::Error),
}

/// The configured external source editor.
pub struct ExternalEditor {
    program: PathBuf,
}

impl ExternalEditor {
    /// Finds VS Code without making it a requirement for running Sindri.
    pub fn detect() -> Result<Self, ExternalEditorError> {
        if let Some(configured) = std::env::var_os("SINDRI_VSCODE") {
            return Ok(Self {
                program: configured.into(),
            });
        }
        CODE_CANDIDATES
            .iter()
            .find_map(|candidate| find_on_path(candidate))
            .map(|program| Self { program })
            .ok_or(ExternalEditorError::NotFound)
    }

    /// Opens the project as the language-server root and focuses `source`.
    pub fn open(&self, project: &Path, source: &Path) -> Result<(), ExternalEditorError> {
        Command::new(&self.program)
            .args(arguments(project, source))
            .spawn()?;
        Ok(())
    }

    /// Installs the Decay extension that belongs to this Sindri checkout/build.
    ///
    /// Packaged builds use their bundled VSIX. Source builds bootstrap the same
    /// VSIX from `editors/vscode-decay` so a developer never has to know about
    /// `SINDRI_DECAY_VSIX` merely to open a script.
    pub fn install_decay_support(&self) -> Result<(), ExternalEditorError> {
        let vsix = ensure_decay_vsix()?;
        let status = Command::new(&self.program)
            .args([
                "--install-extension",
                vsix.to_string_lossy().as_ref(),
                "--force",
            ])
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(ExternalEditorError::ExtensionInstall(status.code()))
        }
    }

    /// Whether VS Code already reports the Sindri Decay extension as installed.
    pub fn has_decay_support(&self) -> Result<bool, ExternalEditorError> {
        let output = Command::new(&self.program)
            .arg("--list-extensions")
            .output()?;
        if !output.status.success() {
            return Ok(false);
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|extension| extension.trim().eq_ignore_ascii_case(DECAY_EXTENSION_ID)))
    }
}

fn arguments(project: &Path, source: &Path) -> [std::ffi::OsString; 3] {
    [
        "--reuse-window".into(),
        project.as_os_str().to_owned(),
        source.as_os_str().to_owned(),
    ]
}

fn ensure_decay_vsix() -> Result<PathBuf, ExternalEditorError> {
    if let Some(path) = decay_vsix_path() {
        return Ok(path);
    }
    build_decay_vsix_from_source()
}

/// Finds an explicitly supplied or packaged VSIX. Source checkout discovery is
/// intentionally separate so a missing packaged artifact falls through to the
/// automatic development bootstrap rather than becoming user-facing plumbing.
fn decay_vsix_path() -> Option<PathBuf> {
    if let Some(configured) = std::env::var_os("SINDRI_DECAY_VSIX") {
        let path = PathBuf::from(configured);
        return path.is_file().then_some(path);
    }
    let executable = std::env::current_exe().ok()?;
    let directory = executable.parent()?;
    [
        directory.join(DECAY_VSIX_NAME),
        directory.join("share").join("sindri").join(DECAY_VSIX_NAME),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

fn build_decay_vsix_from_source() -> Result<PathBuf, ExternalEditorError> {
    let root = source_repository_root().ok_or(ExternalEditorError::DecayVsixNotFound)?;
    let extension = root.join(DECAY_EXTENSION_DIR);
    let bin = extension.join("bin");
    std::fs::create_dir_all(&bin)?;

    run_in(
        &root,
        "cargo",
        &["build", "--release", "--package", "decay-lsp"],
    )?;
    let server_name = if cfg!(windows) {
        "decay-lsp.exe"
    } else {
        "decay-lsp"
    };
    let built_server = root.join("target").join("release").join(server_name);
    let bundled_server = bin.join(server_name);
    std::fs::copy(&built_server, &bundled_server)?;

    // `npm ci` is deterministic and makes a clean checkout sufficient. The
    // package script uses the repository-pinned vsce rather than downloading a
    // mystery version at runtime.
    run_in(&extension, "npm", &["ci"])?;
    run_in(&extension, "npm", &["run", "check"])?;
    run_in(
        &extension,
        "npx",
        &[
            "--no-install",
            "@vscode/vsce",
            "package",
            "--allow-unused-files-pattern",
            "--out",
            DECAY_VSIX_NAME,
        ],
    )?;
    let vsix = extension.join(DECAY_VSIX_NAME);
    vsix.is_file()
        .then_some(vsix)
        .ok_or(ExternalEditorError::DecayVsixNotFound)
}

fn source_repository_root() -> Option<PathBuf> {
    // `CARGO_MANIFEST_DIR` is embedded at compile time, so it still identifies
    // the checkout when the editor is launched from another working directory.
    let editor = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = editor.parent()?.to_path_buf();
    root.join(DECAY_EXTENSION_DIR)
        .join("package.json")
        .is_file()
        .then_some(root)
}

fn run_in(
    directory: &Path,
    command: &'static str,
    args: &[&str],
) -> Result<(), ExternalEditorError> {
    let status = Command::new(command)
        .args(args)
        .current_dir(directory)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(ExternalEditorError::ExtensionBuild {
            command,
            code: status.code(),
        })
    }
}

fn find_on_path(program: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).find_map(|directory| {
        executable_names(program)
            .map(|name| directory.join(name))
            .find(|path| path.is_file())
    })
}

fn executable_names(program: &str) -> impl Iterator<Item = String> {
    let mut names = vec![program.to_owned()];
    if cfg!(windows) {
        names.extend([format!("{program}.cmd"), format!("{program}.exe")]);
    }
    names.into_iter()
}

#[cfg(test)]
mod tests {
    use super::{DECAY_EXTENSION_DIR, DECAY_EXTENSION_ID, arguments, source_repository_root};
    use std::path::Path;

    #[test]
    fn vscode_gets_the_project_before_the_script() {
        assert_eq!(
            arguments(Path::new("game"), Path::new("game/scripts/player.decay")),
            ["--reuse-window", "game", "game/scripts/player.decay"].map(std::ffi::OsString::from)
        );
    }

    #[test]
    fn decay_extension_id_matches_the_manifest_publisher_and_name() {
        assert_eq!(DECAY_EXTENSION_ID, "vardir.sindri-decay");
    }

    #[test]
    fn source_checkout_contains_the_decay_extension() {
        let root =
            source_repository_root().expect("editor should be compiled from the Sindri checkout");
        assert!(
            root.join(DECAY_EXTENSION_DIR)
                .join("package.json")
                .is_file()
        );
    }
}
