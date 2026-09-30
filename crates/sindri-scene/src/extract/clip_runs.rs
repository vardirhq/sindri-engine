//! Draws grouped by what they share, split only where their clip changes.
//!
//! A pass carries one scissor, so draws clipped differently cannot share one.
//! Grouping them by clip as well as by layer would fix that and break order:
//! every unclipped shape of a layer would draw before every clipped one,
//! wherever it was in the scene. So a group keeps its draws in the order they
//! came, as runs, and starts a new run only when the clip changes.

use std::collections::BTreeMap;

/// A pixel scissor: left, top, width, height.
pub(super) type Clip = Option<[u32; 4]>;

/// Draws of kind `T` grouped by `K`, in order, in runs of one clip each.
pub(super) struct ClipRuns<K, T> {
    groups: BTreeMap<K, Vec<(Clip, Vec<T>)>>,
}

impl<K: Ord, T> ClipRuns<K, T> {
    pub(super) const fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
        }
    }

    /// Adds a draw to the end of its group, joining the last run when the clip
    /// is the same and starting a new one when it is not.
    pub(super) fn push(&mut self, key: K, clip: Clip, draw: T) {
        let runs = self.groups.entry(key).or_default();
        match runs.last_mut() {
            Some((last, draws)) if *last == clip => draws.push(draw),
            _ => runs.push((clip, vec![draw])),
        }
    }

    /// Every run, groups in key order and runs in the order they began.
    pub(super) fn into_runs(self) -> impl Iterator<Item = (K, Clip, Vec<T>)>
    where
        K: Clone,
    {
        self.groups.into_iter().flat_map(|(key, runs)| {
            runs.into_iter()
                .map(move |(clip, draws)| (key.clone(), clip, draws))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ClipRuns;

    #[test]
    fn a_change_of_clip_splits_a_group_without_reordering_it() {
        let mut runs = ClipRuns::new();
        let clip = Some([0, 0, 10, 10]);
        runs.push(1, None, 'a');
        runs.push(1, clip, 'b');
        runs.push(1, clip, 'c');
        runs.push(1, None, 'd');
        runs.push(0, None, 'e');
        let order: Vec<_> = runs.into_runs().collect();
        assert_eq!(
            order,
            [
                (0, None, vec!['e']),
                (1, None, vec!['a']),
                (1, clip, vec!['b', 'c']),
                (1, None, vec!['d']),
            ]
        );
    }
}
