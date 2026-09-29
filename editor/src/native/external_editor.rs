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
        let result = ExternalEditor::detect().and_then(|editor| {
            // A Sindri user opening Decay in VS Code asked for the external
            // authoring experience, not for a scavenger hunt through extension
            // artifacts. Keep the companion tooling matched to the editor by
            // installing the VSIX shipped with this Sindri build when needed.
            if source
                .extension()
                .is_some_and(|extension| extension == "decay")
                && !editor.has_decay_support()?
            {
                editor.install_decay_support()?;
            }
            editor.open(&project, source)
        });
        match result {
            Ok(()) => self
                .console
                .info(format!("Opened {} in VS Code", source.display())),
            Err(error) => self.report(error.to_string()),
        }
    }
}
