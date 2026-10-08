//! What a played scene remembers between runs.
//!
//! Kept in memory for as long as the editor is open, and never written to
//! disk. A script's `Save.*` calls work and round-trip inside a session, so
//! persistence can be play-tested; putting a file into someone's project
//! directory because they pressed Play would be a side effect they did not
//! ask for. Where a real save belongs is the shipped host's decision, and
//! `docs/scripting.md` says so.
//!
//! Each run's session writes back through a handle to the one store, so the
//! next run reads what the last one kept.

use std::cell::RefCell;
use std::rc::Rc;

use sindri_core::{SaveDocument, SaveReadError};
use sindri_platform::{SaveBackend, SaveWriteError};

/// The editor's in-memory save, shared by every run in one sitting.
#[derive(Clone, Debug, Default)]
pub(in crate::native) struct EditorSaves(Rc<RefCell<Option<SaveDocument>>>);

impl EditorSaves {
    /// A backend for one run, writing into this store.
    pub(in crate::native) fn backend(&self) -> Box<dyn SaveBackend> {
        Box::new(self.clone())
    }
}

impl SaveBackend for EditorSaves {
    fn read(&mut self) -> Result<Option<SaveDocument>, SaveReadError> {
        Ok(self.0.borrow().clone())
    }

    fn write(&mut self, document: &SaveDocument) -> Result<(), SaveWriteError> {
        *self.0.borrow_mut() = Some(document.clone());
        Ok(())
    }
}
