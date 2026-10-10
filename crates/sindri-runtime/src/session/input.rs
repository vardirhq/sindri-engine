//! Host input requests, drained after gameplay has stepped.

use super::Session;

impl Session {
    /// Text copied or cut in a field since this was last asked, for the host
    /// to put on the system clipboard.
    pub fn take_copied(&mut self) -> Option<String> {
        self.screen_ui.take_copied()
    }

    /// Last successful script's cursor capture request since the host drained it.
    /// This is an intent, not actual capture state, and is not replayed by restore.
    pub fn take_pointer_lock_request(&mut self) -> Option<bool> {
        self.pending_pointer_lock.take()
    }

    pub(super) fn collect_pointer_lock(&mut self, report: &sindri_decay::ScriptReport) {
        if report.pointer_lock_request.is_some() {
            self.pending_pointer_lock = report.pointer_lock_request;
        }
    }
}
