//! Where every playing sequence has got to, and moving it on.

use std::collections::BTreeMap;

use sindri_core::{ComponentRegistryError, ComponentSchemaRegistry, EntityId, World};

use super::property::resolve;
use super::{CueSound, Property, Sequence, SequenceComponent, SequenceError};

/// A cue reached this step, on the entity whose sequence holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct CueFired {
    pub entity: EntityId,
    pub name: String,
}

/// What one advance produced, for the host.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SequenceStep {
    pub cues: Vec<CueFired>,
    /// Sounds cues asked for, for the host's mixer to play.
    pub sounds: Vec<CueSound>,
    /// What could not be played, each said once per start of a sequence.
    pub problems: Vec<(EntityId, SequenceError)>,
}

#[derive(Clone, Debug)]
struct Cursor {
    sequence: String,
    time: f32,
    started: bool,
    finished: bool,
    /// Cues reached in the last advance.
    fired: Vec<String>,
    /// Whether a problem with this start has been said.
    reported: bool,
}

impl Cursor {
    fn new(sequence: &str) -> Self {
        Self {
            sequence: sequence.to_owned(),
            time: 0.0,
            started: false,
            finished: false,
            fired: Vec::new(),
            reported: false,
        }
    }
}

/// Every playing sequence's playhead, held beside the world.
#[derive(Clone, Debug, Default)]
pub struct Sequences {
    cursors: BTreeMap<EntityId, Cursor>,
}

impl Sequences {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves every playing sequence on by `delta` seconds, sets what its
    /// tracks move, and says which cues were reached.
    ///
    /// # Errors
    ///
    /// When the sequence component is not registered.
    pub fn advance(
        &mut self,
        world: &mut World,
        components: &ComponentSchemaRegistry,
        delta: f32,
    ) -> Result<SequenceStep, ComponentRegistryError> {
        let mut step = SequenceStep::default();
        let carriers = components.query::<SequenceComponent>(world)?;
        self.cursors
            .retain(|entity, _| carriers.iter().any(|(carrier, _)| carrier == entity));
        for (entity, component) in carriers {
            if !world.is_active(entity) {
                continue;
            }
            let Some(name) = component.playing.clone() else {
                self.cursors.remove(&entity);
                continue;
            };
            let cursor = self
                .cursors
                .entry(entity)
                .and_modify(|cursor| {
                    if cursor.sequence != name {
                        *cursor = Cursor::new(&name);
                    }
                })
                .or_insert_with(|| Cursor::new(&name));
            cursor.fired.clear();
            if cursor.finished {
                continue;
            }
            let checked = component
                .sequences
                .get(&name)
                .ok_or_else(|| SequenceError::Unknown(name.clone()))
                .and_then(|sequence| sequence.check(&name).map(|()| sequence))
                .and_then(|sequence| {
                    if component.speed.is_finite() && component.speed >= 0.0 {
                        Ok(sequence)
                    } else {
                        Err(SequenceError::Speed(component.speed))
                    }
                });
            let sequence = match checked {
                Ok(sequence) => sequence,
                Err(problem) => {
                    if !std::mem::replace(&mut cursor.reported, true) {
                        step.problems.push((entity, problem));
                    }
                    continue;
                }
            };
            let delta = if delta.is_finite() {
                delta.max(0.0)
            } else {
                0.0
            };
            let reached = move_on(cursor, sequence, delta * component.speed);
            for cue in reached {
                cursor.fired.push(cue.name.clone());
                step.cues.push(CueFired {
                    entity,
                    name: cue.name.clone(),
                });
                if let Some(sound) = &cue.sound {
                    step.sounds.push(sound.clone());
                }
            }
            let time = cursor.time;
            let problems = pose(world, entity, sequence, time);
            if let Some(problem) = problems.into_iter().next()
                && !std::mem::replace(&mut cursor.reported, true)
            {
                step.problems.push((entity, problem));
            }
        }
        Ok(step)
    }

    /// Where an entity's sequence has got to, in seconds, or `None` when it
    /// plays none.
    #[must_use]
    pub fn time(&self, entity: EntityId) -> Option<f32> {
        self.cursors.get(&entity).map(|cursor| cursor.time)
    }

    /// Whether an entity's sequence ran to its end and holds there.
    #[must_use]
    pub fn is_finished(&self, entity: EntityId) -> bool {
        self.cursors
            .get(&entity)
            .is_some_and(|cursor| cursor.finished)
    }

    /// Whether the cue was reached in the last advance.
    #[must_use]
    pub fn cued(&self, entity: EntityId, cue: &str) -> bool {
        self.cursors
            .get(&entity)
            .is_some_and(|cursor| cursor.fired.iter().any(|fired| fired == cue))
    }

    /// Plays an entity's sequence again from its start on the next advance.
    pub fn restart(&mut self, entity: EntityId) {
        if let Some(cursor) = self.cursors.get_mut(&entity) {
            *cursor = Cursor::new(&cursor.sequence);
        }
    }

    /// Forgets every playhead, as stopping a run does.
    pub fn clear(&mut self) {
        self.cursors.clear();
    }
}

/// Moves a cursor on, answering the cues passed in order.
///
/// A cue is reached when the playhead crosses or lands on its time, the
/// sequence's start included on the first step, and its end included when a
/// sequence that does not loop finishes there.
fn move_on<'a>(cursor: &mut Cursor, sequence: &'a Sequence, delta: f32) -> Vec<&'a super::Cue> {
    let duration = sequence.duration;
    let mut reached = Vec::new();
    let mut cues: Vec<&super::Cue> = sequence.cues.iter().collect();
    cues.sort_by(|a, b| a.time.total_cmp(&b.time));
    let mut from = cursor.time;
    let mut left = delta;
    let mut first = !cursor.started;
    cursor.started = true;
    // A loop crossing its end more than once in one step reaches each of its
    // cues once per crossing, but a step is never allowed to run for ever.
    for _ in 0..64 {
        let to = from + left;
        if to < duration {
            reached.extend(
                cues.iter()
                    .filter(|cue| {
                        (cue.time > from || (first && cue.time >= from)) && cue.time <= to
                    })
                    .copied(),
            );
            cursor.time = to;
            return reached;
        }
        reached.extend(
            cues.iter()
                .filter(|cue| {
                    (cue.time > from || (first && cue.time >= from)) && cue.time <= duration
                })
                .copied(),
        );
        if !sequence.looping {
            cursor.time = duration;
            cursor.finished = true;
            return reached;
        }
        left = to - duration;
        from = 0.0;
        first = true;
        if left <= 0.0 {
            cursor.time = 0.0;
            // Landed exactly on the end: the start's cues belong to the next
            // step, which begins there.
            cursor.started = false;
            return reached;
        }
    }
    cursor.time = 0.0;
    reached
}

/// Sets every number a sequence moves to what it is at `time`, on `entity`
/// and the children its tracks name. Answers what could not be set; the
/// rest is set regardless.
///
/// What the editor's Timeline uses to show a moment of a sequence without
/// playing it.
pub fn pose(
    world: &mut World,
    entity: EntityId,
    sequence: &Sequence,
    time: f32,
) -> Vec<SequenceError> {
    let mut problems = Vec::new();
    for track in &sequence.tracks {
        let Some(value) = track.sample(time) else {
            continue;
        };
        let written = Property::parse(&track.property).and_then(|property| {
            let target = resolve(world, entity, &track.target)?;
            let data = world
                .get_mut(target)
                .ok_or_else(|| SequenceError::Target(track.target.clone()))?;
            property.write(data, value)
        });
        if let Err(problem) = written {
            problems.push(problem);
        }
    }
    problems
}
