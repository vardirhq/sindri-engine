//! Choreography: numbers that change over time, and moments that happen at
//! one.
//!
//! A sequence is what a cutscene, a title card sliding in, or a door that
//! opens in three movements is made of. Each **track** moves one number on the
//! entity or one of its children through keyframes; each **cue** is a named
//! moment, which a script can wait for and which may play a sound. A sequence
//! is authored data, so it lives in the scene beside the clips a sprite
//! animation keeps; where its playhead has got to is runtime state, and lives
//! beside the world in [`Sequences`], so watching one play never rewrites the
//! file it came from.
//!
//! Tracks are addressed rather than bound: a target is a path of names, and a
//! property is a word for a transform channel or a component and a path into
//! it. A relative path walks the children of the entity carrying the
//! sequence, so one on a prefab moves the right children wherever the prefab
//! is placed; a path starting with `/` starts from the scene's top level, so
//! a director can choreograph the camera, the player and the UI together.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sindri_core::{Easing, SceneComponent, World};
use thiserror::Error;

mod property;
mod runtime;

pub use property::{Axis, Property, Step, TRANSFORM_PROPERTIES, resolve, top_level};
pub use runtime::{CueFired, SequenceStep, Sequences, pose};

#[cfg(test)]
mod tests;

/// The named sequences an entity can play, and which one is playing.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SequenceComponent {
    pub sequences: BTreeMap<String, Sequence>,
    /// The sequence playing, or nothing.
    #[serde(default)]
    pub playing: Option<String>,
    /// A multiplier on time. Zero holds the playhead where it is.
    #[serde(default = "one")]
    pub speed: f32,
}

impl SceneComponent for SequenceComponent {
    const TYPE_NAME: &'static str = "sindri.sequence";
}

/// One piece of choreography.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Sequence {
    /// How long it runs, in seconds. Keys after it are never reached.
    pub duration: f32,
    /// Whether it starts again at the end rather than holding its last state.
    #[serde(default)]
    pub looping: bool,
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub cues: Vec<Cue>,
}

/// One number moved through keyframes.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Track {
    /// Whose number: empty for the entity carrying the sequence, a path of
    /// child names below it such as `Ship/Flame`, or, with a leading slash, a
    /// path from the scene's top level such as `/Camera`.
    #[serde(default)]
    pub target: String,
    /// Which number; see [`Property`].
    pub property: String,
    /// In time order.
    #[serde(default)]
    pub keys: Vec<Key>,
}

/// A value at a time, and how the track moves from here to the next key.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Key {
    pub time: f32,
    pub value: f32,
    /// A CSS timing curve name: `linear`, `ease`, `ease-in`, `ease-out` or
    /// `ease-in-out`.
    #[serde(default = "linear")]
    pub ease: String,
}

/// A named moment.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Cue {
    pub time: f32,
    pub name: String,
    /// A sound to play when it is reached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound: Option<CueSound>,
}

/// A sound a cue plays, routed like any other.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CueSound {
    pub clip: String,
    /// Empty means the effects bus.
    #[serde(default)]
    pub bus: String,
    #[serde(default = "one")]
    pub volume: f32,
}

impl CueSound {
    /// The bus it plays through.
    #[must_use]
    pub fn bus(&self) -> &str {
        if self.bus.is_empty() {
            "effects"
        } else {
            &self.bus
        }
    }
}

const fn one() -> f32 {
    1.0
}

fn linear() -> String {
    "linear".to_owned()
}

/// What is wrong with a sequence, or with applying it.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum SequenceError {
    #[error("there is no sequence called '{0}'")]
    Unknown(String),
    #[error("sequence '{sequence}' lasts {duration} seconds; it must last longer than nothing")]
    NoDuration { sequence: String, duration: f32 },
    #[error("'{0}' is not a property a sequence can move")]
    Property(String),
    #[error("'{0}' is not an easing; use linear, ease, ease-in, ease-out or ease-in-out")]
    Ease(String),
    #[error("the keys of '{property}' are not in time order")]
    Unordered { property: String },
    #[error("no child called '{0}' to move")]
    Target(String),
    #[error("'{0}' has no number to move there")]
    Field(String),
    #[error("speed {0} is not a speed; it must be zero or more")]
    Speed(f32),
}

impl Sequence {
    /// Everything that would stop it playing as written.
    pub fn check(&self, name: &str) -> Result<(), SequenceError> {
        if !self.duration.is_finite() || self.duration <= 0.0 {
            return Err(SequenceError::NoDuration {
                sequence: name.to_owned(),
                duration: self.duration,
            });
        }
        for track in &self.tracks {
            Property::parse(&track.property)?;
            if track
                .keys
                .windows(2)
                .any(|pair| pair[1].time < pair[0].time)
            {
                return Err(SequenceError::Unordered {
                    property: track.property.clone(),
                });
            }
            for key in &track.keys {
                easing(&key.ease)?;
            }
        }
        Ok(())
    }
}

impl Track {
    /// The value at `time`: the first key's before it, the last key's after
    /// it, and between two keys the earlier key's curve. `None` with no keys.
    #[must_use]
    pub fn sample(&self, time: f32) -> Option<f32> {
        let first = self.keys.first()?;
        if time <= first.time {
            return Some(first.value);
        }
        for pair in self.keys.windows(2) {
            let (from, to) = (&pair[0], &pair[1]);
            if time < to.time {
                let span = to.time - from.time;
                let t = if span > 0.0 {
                    (time - from.time) / span
                } else {
                    1.0
                };
                let eased = easing(&from.ease).unwrap_or(Easing::Linear).apply(t);
                return Some((to.value - from.value).mul_add(eased, from.value));
            }
        }
        self.keys.last().map(|key| key.value)
    }
}

/// Every sound a cue in `world` plays, so an export carries it.
///
/// Read from the stored payloads rather than through the registry: a sequence
/// that does not parse plays nothing, and names nothing to carry either.
#[must_use]
pub fn referenced_sounds(world: &World) -> BTreeSet<String> {
    world
        .entities()
        .filter_map(|(_, data)| data.components.get(SequenceComponent::TYPE_NAME))
        .filter_map(|payload| serde_json::from_value::<SequenceComponent>(payload.clone()).ok())
        .flat_map(|component| component.sequences.into_values())
        .flat_map(|sequence| sequence.cues)
        .filter_map(|cue| cue.sound)
        .map(|sound| sound.clip)
        .filter(|clip| !clip.is_empty())
        .collect()
}

/// Reads a curve name.
pub fn easing(name: &str) -> Result<Easing, SequenceError> {
    Easing::named(name).ok_or_else(|| SequenceError::Ease(name.to_owned()))
}

/// Every curve name, in the order an editor offers them.
pub const EASINGS: [&str; 5] = ["linear", "ease", "ease-in", "ease-out", "ease-in-out"];
