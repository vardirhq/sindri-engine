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

#[derive(Debug, Error)]
pub enum ExternalEditorError {
    #[error("VS Code was not found; install its `code` command or set SINDRI_VSCODE")]
    NotFound,
    #[error("Sindri Decay VS Code support was not found; set SINDRI_DECAY_VSIX to its .vsix file")]
    DecayVsixNotFound,
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

    /// Installs the Decay extension shipped with this Sindri build into VS Code.
    pub fn install_decay_support(&self) -> Result<(), ExternalEditorError> {
        let vsix = decay_vsix_path().ok_or(ExternalEditorError::DecayVsixNotFound)?;
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

/// Finds the VSIX next to a packaged editor, with an explicit override for
/// source builds and unusual package layouts.
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
    use super::{DECAY_EXTENSION_ID, arguments};
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
}
