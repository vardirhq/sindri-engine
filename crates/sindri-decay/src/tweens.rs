//! Managed tween values, owned by a script runner rather than scene data.

use std::collections::{BTreeMap, BTreeSet};
use decay_runtime::Value;
use sindri_core::{Easing, EntityId};

pub(crate) const LIMIT: usize = 8192;

pub(crate) struct Track {
    pub(crate) owner: EntityId,
    pub(crate) from: Value,
    pub(crate) to: Value,
    pub(crate) duration: f64,
    pub(crate) elapsed: f64,
    pub(crate) easing: Easing,
    pub(crate) paused: bool,
    pub(crate) cancelled: bool,
}

impl Track {
    pub(crate) fn done(&self) -> bool {
        !self.cancelled && self.elapsed >= self.duration
    }

    pub(crate) fn progress(&self) -> f64 {
        if self.duration <= 0.0 { 1.0 }
        else { (self.elapsed / self.duration).clamp(0.0, 1.0) }
    }

    pub(crate) fn value(&self) -> Value {
        // A normalized fraction fits f32; easing uses the shared UI math.
        #[allow(clippy::cast_possible_truncation)]
        let t = f64::from(self.easing.apply(self.progress() as f32));
        if t <= 0.0 { return self.from.clone(); }
        if t >= 1.0 { return self.to.clone(); }
        let mix = |a: f64, b: f64| a * (1.0 - t) + b * t;
        match (&self.from, &self.to) {
            (Value::Number(a), Value::Number(b)) => Value::Number(mix(*a, *b)),
            (a, b) => {
                let components: Vec<_> = a.components().unwrap_or_default().iter()
                    .zip(b.components().unwrap_or_default())
                    .map(|(a, b)| mix(*a, *b)).collect();
                Value::vector(&components).unwrap_or(Value::Unit)
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct Tweens {
    tracks: BTreeMap<u64, Track>,
    owners: BTreeMap<EntityId, BTreeSet<u64>>,
    // Never reused, including after clear: stale handles cannot name new tracks.
    next: u64,
}

impl Tweens {
    pub(crate) fn insert(&mut self, track: Track) -> Result<u64, &'static str> {
        if self.tracks.len() >= LIMIT { return Err("exceeds the 8192 tween limit; dispose unused handles"); }
        let id = self.next.checked_add(1).ok_or("exhausted tween handles")?;
        self.next = id;
        self.owners.entry(track.owner).or_default().insert(id);
        self.tracks.insert(id, track);
        Ok(id)
    }

    #[cfg(test)]
    pub(crate) fn get(&self, id: u64) -> Option<&Track> { self.tracks.get(&id) }
    pub(crate) fn get_mut(&mut self, id: u64) -> Option<&mut Track> { self.tracks.get_mut(&id) }
    pub(crate) fn dispose(&mut self, id: u64) {
        if let Some(track) = self.tracks.remove(&id) {
            if let Some(ids) = self.owners.get_mut(&track.owner) { ids.remove(&id); }
            if self.owners.get(&track.owner).is_some_and(BTreeSet::is_empty) {
                self.owners.remove(&track.owner);
            }
        }
    }
    pub(crate) fn clear(&mut self) { self.tracks.clear(); self.owners.clear(); }
    pub(crate) fn remove_owner(&mut self, owner: EntityId) {
        for id in self.owners.remove(&owner).unwrap_or_default() { self.tracks.remove(&id); }
    }
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(EntityId) -> bool) {
        let removed: Vec<_> = self.owners.keys().copied().filter(|owner| !keep(*owner)).collect();
        for owner in removed { self.remove_owner(owner); }
    }
    pub(crate) fn advance(&mut self, owner: EntityId, seconds: f64) {
        for id in self.owners.get(&owner).into_iter().flatten() {
            if let Some(track) = self.tracks.get_mut(id) && !track.paused && !track.cancelled {
                track.elapsed = (track.elapsed + seconds).min(track.duration);
            }
        }
    }
}

#[cfg(test)]
mod tests;
