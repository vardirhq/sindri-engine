//! Small composition tools share the existing score, mixer, and undo history.
mod ensemble;
mod harmony;
mod phrase;
mod snapshots;

pub(super) struct Tools {
    pub(super) first_bar: usize,
    pub(super) last_bar: usize,
    pub(super) range_visible: bool,
    pub(super) palette: [usize; 2],
    pub(super) snapshot_name: String,
    pub(super) snapshots: Vec<crate::lab::song::NamedSnapshot>,
}
impl Default for Tools {
    fn default() -> Self {
        Self {
            first_bar: 9,
            last_bar: 12,
            range_visible: false,
            palette: [0; 2],
            snapshot_name: "Version 1".into(),
            snapshots: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests;
