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

#[derive(Debug, Error)]
pub enum ExternalEditorError {
    #[error("VS Code was not found; install its `code` command or set SINDRI_VSCODE")]
    NotFound,
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
}

fn arguments(project: &Path, source: &Path) -> [std::ffi::OsString; 3] {
    [
        "--reuse-window".into(),
        project.as_os_str().to_owned(),
        source.as_os_str().to_owned(),
    ]
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
    use super::arguments;
    use std::path::Path;

    #[test]
    fn vscode_gets_the_project_before_the_script() {
        assert_eq!(
            arguments(Path::new("game"), Path::new("game/scripts/player.decay")),
            ["--reuse-window", "game", "game/scripts/player.decay"].map(std::ffi::OsString::from)
        );
    }
}
