//! Managed tween values, owned by a script runner rather than scene data.

use decay_runtime::Value;
use sindri_core::{Easing, EntityId};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const LIMIT: usize = 8192;

#[derive(Clone)]
pub(crate) struct Track {
    pub(crate) owner: EntityId,
    pub(crate) from: Value,
    pub(crate) to: Value,
    pub(crate) duration: f64,
    /// Time played, across every loop, not counting the delay.
    pub(crate) elapsed: f64,
    pub(crate) easing: Easing,
    pub(crate) paused: bool,
    pub(crate) cancelled: bool,
    /// Seconds to hold the starting value before playing.
    pub(crate) delay: f64,
    /// How much of the delay has passed.
    pub(crate) waited: f64,
    /// How many times it plays; zero plays for ever.
    pub(crate) loops: u32,
    /// Whether every other play runs back from `to` to `from`.
    pub(crate) yoyo: bool,
    /// The tween this one waits to finish before its delay starts: a sequence.
    pub(crate) after: Option<u64>,
}

impl Track {
    pub(crate) fn new(
        owner: EntityId,
        from: Value,
        to: Value,
        duration: f64,
        easing: Easing,
    ) -> Self {
        Self {
            owner,
            from,
            to,
            duration,
            elapsed: 0.0,
            easing,
            paused: false,
            cancelled: false,
            delay: 0.0,
            waited: 0.0,
            loops: 1,
            yoyo: false,
            after: None,
        }
    }

    /// How long every play together lasts: for ever for an endless loop.
    fn total(&self) -> f64 {
        if self.duration <= 0.0 {
            0.0
        } else if self.loops == 0 {
            f64::INFINITY
        } else {
            self.duration * f64::from(self.loops)
        }
    }

    pub(crate) fn done(&self) -> bool {
        !self.cancelled && self.elapsed >= self.total()
    }

    /// The linear fraction of the current play, running back down on a
    /// yoyo's return: where between `from` and `to` it is, before easing.
    pub(crate) fn progress(&self) -> f64 {
        let back = |play: f64| self.yoyo && play % 2.0 >= 1.0;
        if self.duration <= 0.0 || self.elapsed >= self.total() {
            // Finished: at `to`, unless a yoyo's last play ran back.
            return if back(f64::from(self.loops.max(1)) - 1.0) {
                0.0
            } else {
                1.0
            };
        }
        let cycle = self.elapsed / self.duration;
        let play = cycle.floor();
        let along = (cycle - play).clamp(0.0, 1.0);
        if back(play) { 1.0 - along } else { along }
    }

    /// Plays `seconds` forward: the delay first, then the plays, stopping at
    /// the end of the last one.
    fn play(&mut self, seconds: f64) {
        let waiting = (self.delay - self.waited).max(0.0).min(seconds);
        self.waited += waiting;
        self.elapsed = (self.elapsed + seconds - waiting).min(self.total());
    }

    pub(crate) fn value(&self) -> Value {
        // A normalized fraction fits f32; easing uses the shared UI math.
        #[allow(clippy::cast_possible_truncation)]
        let t = f64::from(self.easing.apply(self.progress() as f32));
        if t <= 0.0 {
            return self.from.clone();
        }
        if t >= 1.0 {
            return self.to.clone();
        }
        let mix = |a: f64, b: f64| a * (1.0 - t) + b * t;
        match (&self.from, &self.to) {
            (Value::Number(a), Value::Number(b)) => Value::Number(mix(*a, *b)),
            (a, b) => {
                let components: Vec<_> = a
                    .components()
                    .unwrap_or_default()
                    .iter()
                    .zip(b.components().unwrap_or_default())
                    .map(|(a, b)| mix(*a, *b))
                    .collect();
                Value::vector(&components).unwrap_or(Value::Unit)
            }
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct Tweens {
    tracks: BTreeMap<u64, Track>,
    owners: BTreeMap<EntityId, BTreeSet<u64>>,
    // Never reused, including after clear: stale handles cannot name new tracks.
    next: u64,
}

impl Tweens {
    pub(crate) fn insert(&mut self, track: Track) -> Result<u64, &'static str> {
        if self.tracks.len() >= LIMIT {
            return Err("exceeds the 8192 tween limit; dispose unused handles");
        }
        let id = self.next.checked_add(1).ok_or("exhausted tween handles")?;
        self.next = id;
        self.owners.entry(track.owner).or_default().insert(id);
        self.tracks.insert(id, track);
        Ok(id)
    }

    #[cfg(test)]
    pub(crate) fn get(&self, id: u64) -> Option<&Track> {
        self.tracks.get(&id)
    }
    pub(crate) fn get_mut(&mut self, id: u64) -> Option<&mut Track> {
        self.tracks.get_mut(&id)
    }
    pub(crate) fn dispose(&mut self, id: u64) {
        if let Some(track) = self.tracks.remove(&id) {
            if let Some(ids) = self.owners.get_mut(&track.owner) {
                ids.remove(&id);
            }
            if self
                .owners
                .get(&track.owner)
                .is_some_and(BTreeSet::is_empty)
            {
                self.owners.remove(&track.owner);
            }
        }
    }
    pub(crate) fn clear(&mut self) {
        self.tracks.clear();
        self.owners.clear();
    }
    pub(crate) fn remove_owner(&mut self, owner: EntityId) {
        for id in self.owners.remove(&owner).unwrap_or_default() {
            self.tracks.remove(&id);
        }
    }
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(EntityId) -> bool) {
        let removed: Vec<_> = self
            .owners
            .keys()
            .copied()
            .filter(|owner| !keep(*owner))
            .collect();
        for owner in removed {
            self.remove_owner(owner);
        }
    }
    pub(crate) fn advance(&mut self, owner: EntityId, seconds: f64) {
        // A tween after another holds still until that one has finished, as
        // it stood before this advance: the next one starts on the step after
        // the last ends, whichever order they were made in. One whose
        // predecessor was disposed has nothing left to wait for.
        let ready: Vec<u64> = self
            .owners
            .get(&owner)
            .into_iter()
            .flatten()
            .copied()
            .filter(|id| {
                self.tracks
                    .get(id)
                    .and_then(|track| track.after)
                    .is_none_or(|before| self.tracks.get(&before).is_none_or(Track::done))
            })
            .collect();
        for id in ready {
            if let Some(track) = self.tracks.get_mut(&id)
                && !track.paused
                && !track.cancelled
            {
                track.play(seconds);
            }
        }
    }
}

#[cfg(test)]
mod tests;
