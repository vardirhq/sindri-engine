//! Native-shell bridge to external source editors.

use std::path::Path;

use crate::external_editor::ExternalEditor;
use crate::project::manifest;

use super::EditorApp;

impl EditorApp {
    pub(super) fn open_external_editor(&mut self, source: &Path) {
        let project = self
            .open_project_root
            .clone()
            .or_else(|| manifest::root_for(source))
            .or_else(|| source.parent().map(Path::to_path_buf));
        let Some(project) = project else {
            self.report(format!("{} has no parent directory", source.display()));
            return;
        };
        match ExternalEditor::detect().and_then(|editor| editor.open(&project, source)) {
            Ok(()) => self
                .console
                .info(format!("Opened {} in VS Code", source.display())),
            Err(error) => self.report(error.to_string()),
        }
    }
}
