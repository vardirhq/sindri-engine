//! Selection actions from the mobile toolbar.

use super::Host;

impl Host {
    pub(super) fn selection_command(&mut self, command: &str) -> Result<(), String> {
        let (opened, editing) = self.editing()?;
        match command {
            "selectMore" => {
                editing.select_more();
                Ok(())
            }
            "duplicate" => editing.duplicate_selection(&mut opened.world),
            "delete" => editing.delete_selection(&mut opened.world),
            _ => Err(format!("unknown selection command {command}")),
        }
    }
}
