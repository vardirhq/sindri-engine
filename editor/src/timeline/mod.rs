//! Editing a sequence: what the Timeline panel changes, without the panel.
//!
//! Every edit here takes the sequence component as it is stored and gives
//! back the one to store, keeping each track's keys in time order so the
//! runtime never sees them otherwise. The panel draws; this decides.

use sindri_core::{EntityId, World};
use sindri_scene::{Cue, CueSound, Key, Property, Sequence, SequenceComponent, Track};

/// Keys and cues land on this grid when dragged, so two meant to coincide do.
pub const SNAP: f32 = 0.05;

/// What is picked in the panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Picked {
    Key { track: usize, key: usize },
    Cue(usize),
}

/// The panel's own state, which is the editor's and never the scene's.
#[derive(Clone, Debug, Default)]
pub struct TimelineState {
    /// The sequence being looked at, by name.
    pub sequence: Option<String>,
    /// Where the playhead is, in seconds.
    pub time: f32,
    /// Whether the playhead is moving on its own.
    pub playing: bool,
    /// Whether the Scene view shows the sequence posed at the playhead.
    pub preview: bool,
    pub picked: Option<Picked>,
    /// The track a new one is added for: a target path and a property.
    pub new_target: String,
    pub new_property: String,
    /// Which entity's sequence the panel showed last, and on which frame, so
    /// the Scene view poses it only while the panel is showing it.
    pub shown: Option<(EntityId, u64)>,
}

impl TimelineState {
    /// The sequence to look at in `component`: the one chosen, else the one
    /// playing, else the first.
    pub fn chosen<'a>(
        &mut self,
        component: &'a SequenceComponent,
    ) -> Option<(String, &'a Sequence)> {
        let name = self
            .sequence
            .clone()
            .filter(|name| component.sequences.contains_key(name))
            .or_else(|| {
                component
                    .playing
                    .clone()
                    .filter(|name| component.sequences.contains_key(name))
            })
            .or_else(|| component.sequences.keys().next().cloned())?;
        if self.sequence.as_ref() != Some(&name) {
            self.sequence = Some(name.clone());
            self.picked = None;
        }
        let sequence = component.sequences.get(&name)?;
        Some((name, sequence))
    }

    /// Moves the playhead on by `delta`, wrapping or stopping at the end.
    pub fn tick(&mut self, sequence: &Sequence, delta: f32) {
        if !self.playing {
            return;
        }
        self.time += delta;
        if self.time >= sequence.duration {
            if sequence.looping && sequence.duration > 0.0 {
                self.time %= sequence.duration;
            } else {
                self.time = sequence.duration;
                self.playing = false;
            }
        }
    }
}

/// Rounds a time to the snapping grid and keeps it inside the sequence.
#[must_use]
pub fn snapped(time: f32, duration: f32) -> f32 {
    ((time / SNAP).round() * SNAP).clamp(0.0, duration.max(0.0))
}

/// One change to a sequence, as the panel asks for it.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    AddSequence(String),
    Duration(f32),
    Looping(bool),
    /// Which sequence plays when the scene starts; `None` for none.
    Autoplay(Option<String>),
    AddTrack {
        target: String,
        property: String,
    },
    RemoveTrack(usize),
    /// Adds a key, or sets the one already at that time.
    Key {
        track: usize,
        time: f32,
        value: f32,
    },
    MoveKey {
        track: usize,
        key: usize,
        time: f32,
    },
    SetKey {
        track: usize,
        key: usize,
        value: f32,
        ease: String,
    },
    RemoveKey {
        track: usize,
        key: usize,
    },
    AddCue {
        time: f32,
    },
    SetCue {
        cue: usize,
        time: f32,
        name: String,
        sound: String,
    },
    RemoveCue(usize),
}

/// Applies `edit` to the sequence called `name`, answering what is picked
/// after it: a key that moved is still picked where it landed.
pub fn apply(component: &mut SequenceComponent, name: &str, edit: Edit) -> Option<Picked> {
    if let Edit::AddSequence(new) = &edit {
        component
            .sequences
            .entry(new.clone())
            .or_insert_with(|| Sequence {
                duration: 2.0,
                ..Sequence::default()
            });
        return None;
    }
    if let Edit::Autoplay(playing) = &edit {
        component.playing.clone_from(playing);
        return None;
    }
    let sequence = component.sequences.get_mut(name)?;
    match edit {
        Edit::AddSequence(_) | Edit::Autoplay(_) => None,
        Edit::Duration(duration) => {
            sequence.duration = duration.max(SNAP);
            None
        }
        Edit::Looping(looping) => {
            sequence.looping = looping;
            None
        }
        Edit::AddTrack { target, property } => {
            sequence.tracks.push(Track {
                target,
                property,
                keys: Vec::new(),
            });
            None
        }
        Edit::RemoveTrack(track) => {
            if track < sequence.tracks.len() {
                sequence.tracks.remove(track);
            }
            None
        }
        Edit::Key { .. } | Edit::MoveKey { .. } | Edit::SetKey { .. } | Edit::RemoveKey { .. } => {
            key_edit(sequence, edit)
        }
        Edit::AddCue { .. } | Edit::SetCue { .. } | Edit::RemoveCue(_) => cue_edit(sequence, edit),
    }
}

/// The edits that change a track's keys.
fn key_edit(sequence: &mut Sequence, edit: Edit) -> Option<Picked> {
    match edit {
        Edit::Key { track, time, value } => {
            let keys = &mut sequence.tracks.get_mut(track)?.keys;
            let at = if let Some(at) = keys.iter().position(|key| (key.time - time).abs() < 1e-4) {
                keys[at].value = value;
                at
            } else {
                keys.push(Key {
                    time,
                    value,
                    ease: "linear".to_owned(),
                });
                sort(keys, keys.len() - 1)
            };
            Some(Picked::Key { track, key: at })
        }
        Edit::MoveKey { track, key, time } => {
            let keys = &mut sequence.tracks.get_mut(track)?.keys;
            keys.get_mut(key)?.time = time;
            Some(Picked::Key {
                track,
                key: sort(keys, key),
            })
        }
        Edit::SetKey {
            track,
            key,
            value,
            ease,
        } => {
            let found = sequence.tracks.get_mut(track)?.keys.get_mut(key)?;
            found.value = value;
            found.ease = ease;
            Some(Picked::Key { track, key })
        }
        Edit::RemoveKey { track, key } => {
            let keys = &mut sequence.tracks.get_mut(track)?.keys;
            if key < keys.len() {
                keys.remove(key);
            }
            None
        }
        _ => None,
    }
}

/// The edits that change the cues.
fn cue_edit(sequence: &mut Sequence, edit: Edit) -> Option<Picked> {
    match edit {
        Edit::AddCue { time } => {
            let number = sequence.cues.len() + 1;
            sequence.cues.push(Cue {
                time,
                name: format!("cue {number}"),
                sound: None,
            });
            Some(Picked::Cue(sequence.cues.len() - 1))
        }
        Edit::SetCue {
            cue,
            time,
            name,
            sound,
        } => {
            let found = sequence.cues.get_mut(cue)?;
            found.time = time;
            found.name = name;
            found.sound = if sound.trim().is_empty() {
                None
            } else {
                Some(found.sound.take().map_or_else(
                    || CueSound {
                        clip: sound.trim().to_owned(),
                        bus: String::new(),
                        volume: 1.0,
                    },
                    |mut kept| {
                        sound.trim().clone_into(&mut kept.clip);
                        kept
                    },
                ))
            };
            Some(Picked::Cue(cue))
        }
        Edit::RemoveCue(cue) => {
            if cue < sequence.cues.len() {
                sequence.cues.remove(cue);
            }
            None
        }
        _ => None,
    }
}

/// Puts the key at `moved` back in time order, answering where it went.
fn sort(keys: &mut Vec<Key>, moved: usize) -> usize {
    let key = keys.remove(moved);
    let at = keys
        .iter()
        .position(|other| other.time > key.time)
        .unwrap_or(keys.len());
    keys.insert(at, key);
    at
}

/// What a track would key now: the number as the scene holds it.
#[must_use]
pub fn current_value(world: &World, carrier: EntityId, track: &Track) -> Option<f32> {
    let property = Property::parse(&track.property).ok()?;
    let target = sindri_scene::resolve(world, carrier, &track.target).ok()?;
    property.read(world.get(target)?)
}

/// Every target a track on `carrier` can name: itself, then each descendant
/// by its path of names, then every other entity in the scene by its path
/// from the top. Unnamed ones cannot be addressed and are left out.
#[must_use]
pub fn targets(world: &World, carrier: EntityId) -> Vec<String> {
    fn walk(world: &World, entity: EntityId, path: &str, into: &mut Vec<(String, EntityId)>) {
        let Some(data) = world.get(entity) else {
            return;
        };
        for child in &data.children {
            let Some(name) = world.get(*child).and_then(|child| child.name.clone()) else {
                continue;
            };
            let path = format!("{path}/{name}");
            into.push((path.clone(), *child));
            walk(world, *child, &path, into);
        }
    }
    let mut below = Vec::new();
    walk(world, carrier, "", &mut below);
    let mut targets = vec![String::new()];
    targets.extend(below.iter().map(|(path, _)| path[1..].to_owned()));
    let mut everywhere = Vec::new();
    let level = sindri_scene::top_level(world, carrier);
    for (entity, data) in world.entities() {
        if data.parent != level {
            continue;
        }
        let Some(name) = data.name.clone() else {
            continue;
        };
        let path = format!("/{name}");
        everywhere.push((path.clone(), entity));
        walk(world, entity, &path, &mut everywhere);
    }
    // The carrier and what is under it are already named relatively, which
    // keeps a prefab's sequence working wherever it is placed.
    targets.extend(
        everywhere
            .into_iter()
            .filter(|(_, entity)| *entity != carrier && !below.iter().any(|(_, b)| b == entity))
            .map(|(path, _)| path),
    );
    targets
}

#[cfg(test)]
mod tests;
