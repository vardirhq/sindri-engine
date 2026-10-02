//! Running the scripts a world holds.
//!
//! Mirrors [`sindri_scene::SpriteAnimations`] on purpose: authored facts live
//! in the scene, and what a script has become halfway through a run lives here,
//! beside the world rather than in it. A script instance's fields drift as it
//! runs, and if they were the component, watching a scene play would rewrite
//! the file it was opened from.

mod authored;
mod environment;
mod frame;
mod person_surface;
mod project;

pub(crate) use authored::{blank, field_type, is_compound};
pub use project::Project;
pub(crate) use project::{ON, SharedField, board_key};
pub(crate) use run::{to_value, variant_name};
mod run;
mod sources;
mod timing;

use std::collections::{BTreeMap, BTreeSet};

use decay_ir::IrProgram;
use decay_runtime::{ScriptInstance, Value};
use sindri_core::{ComponentSchemaRegistry, EntityId, World};

use self::run::{TickWorld, ensure_compiled, tick};
use self::timing::timed;
use crate::{
    Blackboard, ScriptComponent, ScriptExport, ScriptFailure, ScriptMessage, ScriptReport,
    audio_host::{AudioCommand, AudioQueue},
    exports::exports_of,
    surface::{PREFAB, PROFILE},
};

pub use environment::{environment, referenced_sources};
pub use frame::ScriptFrame;
pub use sources::{LIFECYCLE, LifecycleFunction, ScriptSources};

/// How many times a pass will start what the previous round spawned.
///
/// A script started by a spawn may spawn in turn, and settling that in one
/// frame is what makes "a bullet moves on the frame it is fired" true for a
/// bullet fired by something that was itself just created. A cascade that does
/// not settle is a bug in the scripts, and it is reported with the round count
/// rather than being run until the frame is gone.
const SPAWN_ROUNDS: usize = 8;

/// How many rounds of messages sending messages a pass delivers before it
/// stops. Enough for a reply to a reply; a loop that never ends is a bug.
const MESSAGE_ROUNDS: usize = 8;

/// How many entities one pass of scripts may create.
///
/// Decay's operation budget already stops a loop that never ends, but it stops
/// it after a million instructions — long after a spawn loop with a mistaken
/// bound has put a hundred thousand entities in the world and taken the editor
/// with it. This is the same protection stated in the units the mistake is
/// made in.
pub(crate) const SPAWN_LIMIT_PER_PASS: usize = 4096;

struct Compiled {
    source: String,
    /// The project's declared shape it was compiled against: another script
    /// gaining or losing a field changes what this one may say about it.
    project: String,
    /// Shared so a call can hold its program while the cache is lent elsewhere.
    program: std::rc::Rc<IrProgram>,
}

/// Files one tick's outcome into the report.
fn collect(
    report: &mut ScriptReport,
    entity: EntityId,
    outcome: Result<Vec<String>, ScriptFailure>,
) {
    match outcome {
        Ok(printed) => report.printed.extend(
            printed
                .into_iter()
                .map(|message| ScriptMessage { entity, message }),
        ),
        Err(failure) => report.failures.push(failure),
    }
}

pub(crate) struct Running {
    elapsed_seconds: f32,
    source: String,
    pub(crate) script: String,
    pub(crate) instance: ScriptInstance,
}

/// Fields set on scripts that have not started, by entity and field.
pub(crate) type StartingValues = BTreeMap<EntityId, BTreeMap<String, decay_runtime::Value>>;

/// A call one script made on another, or an event one emitted, waiting to be
/// delivered.
pub(crate) struct Message {
    /// Who it is for: one entity, or — for an event — every running script
    /// with a handler for it.
    pub(crate) to: Option<EntityId>,
    pub(crate) name: String,
    pub(crate) args: Vec<decay_runtime::Value>,
}

#[derive(Default)]
pub struct Scripts {
    programs: BTreeMap<String, Compiled>,
    running: BTreeMap<EntityId, Running>,
    /// Fields another script set on one that has not started yet, applied
    /// when it does. Held here rather than written into the scene's component:
    /// they are the running game's, not the author's, and can name things only
    /// a running game has — an entity, most usefully.
    starting: StartingValues,
    blackboard: Blackboard,
    tweens: crate::tweens::Tweens,
    /// What scripts asked to play, for whoever owns an audio device to perform.
    audio: AudioQueue,
    /// The scene's input actions and what each is worth this step.
    actions: crate::actions::InputActions,
    /// Whether each script's tick is timed into the report.
    measuring: bool,
}

impl Scripts {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Times each script's tick into [`ScriptReport::timings`] from the next
    /// pass on. Off unless asked for: a shipped game has nobody to show them.
    pub const fn set_measuring(&mut self, measuring: bool) {
        self.measuring = measuring;
    }

    #[must_use]
    pub fn is_running(&self, entity: EntityId) -> bool {
        self.running.contains_key(&entity)
    }

    /// Takes what scripts asked to play since the last call.
    ///
    /// A caller with no audio device — the editor, a headless test — simply
    /// never calls this, and the requests are dropped with the runner rather
    /// than accumulating somewhere global.
    pub fn take_audio_commands(&mut self) -> Vec<AudioCommand> {
        self.audio.take()
    }

    /// A bus's volume as the scripts last set it, full for one never set.
    #[must_use]
    pub fn audio_volume(&self, bus: &str) -> f32 {
        self.audio.volume(bus)
    }

    #[must_use]
    pub fn field(&self, entity: EntityId, name: &str) -> Option<&Value> {
        self.running.get(&entity)?.instance.field(name)
    }

    pub fn compile(
        &mut self,
        world: &World,
        components: &ComponentSchemaRegistry,
        sources: &ScriptSources,
    ) -> Vec<ScriptFailure> {
        let scripted = match components.query::<ScriptComponent>(world) {
            Ok(scripted) => scripted,
            Err(error) => return vec![ScriptFailure::Registry(error.to_string())],
        };
        let mut failures = Vec::new();
        for (entity, component) in scripted {
            if let Err(failure) = ensure_compiled(&mut self.programs, sources, entity, &component) {
                failures.push(failure);
            }
        }
        failures
    }

    /// Runs one pass of every enabled script, then starts what that pass
    /// spawned.
    ///
    /// A spawned entity's script starts **in the same pass**, so a bullet
    /// created during an update moves during that update rather than standing
    /// still for a frame. It cannot start during the call that created it:
    /// building an instance runs the container's field initializers, which is
    /// Decay code, and the world is already lent to the call in progress.
    /// So spawning creates the entity now and starting it happens after —
    /// which is also why `World.set_property` is the way a spawner authors a
    /// starting value, and why it is refused once an instance exists.
    ///
    /// A script started this way may spawn in turn. Those rounds are bounded:
    /// a cascade that does not settle is stopped and reported rather than
    /// taking the frame with it.
    pub fn advance(
        &mut self,
        world: &mut World,
        components: &ComponentSchemaRegistry,
        frame: ScriptFrame<'_>,
    ) -> ScriptReport {
        let ScriptFrame {
            sources,
            prefabs,
            scenes,
            profiles,
            input,
            physics,
            screen_ui,
            aim,
            gestures,
            camera_pan,
            random,
            saves,
            effects,
            animations,
            tile_sets,
            delta_seconds,
        } = frame;
        let mut report = ScriptReport::default();
        if !delta_seconds.is_finite() || delta_seconds < 0.0 {
            report.failures.push(ScriptFailure::BadDelta(delta_seconds));
            return report;
        }

        let scripted = match components.query::<ScriptComponent>(world) {
            Ok(scripted) => scripted,
            Err(error) => {
                report
                    .failures
                    .push(ScriptFailure::Registry(error.to_string()));
                return report;
            }
        };

        if let Some(problem) = self.actions.update(world, input) {
            report.failures.push(ScriptFailure::Actions(problem));
        }
        let Self {
            programs,
            running,
            starting,
            blackboard,
            tweens,
            audio,
            actions,
            measuring,
        } = self;
        let measuring = *measuring;
        let mut at = TickWorld {
            programs,
            running,
            starting,
            blackboard,
            tweens,
            audio,
            actions,
            world,
            sources,
            prefabs,
            scenes,
            profiles,
            input,
            physics,
            screen_ui,
            aim,
            gestures,
            camera_pan,
            random,
            saves,
            effects,
            animations,
            tile_sets,
            started: BTreeSet::new(),
            spawned: Vec::new(),
            messages: Vec::new(),
        };
        at.started.extend(at.running.keys().copied());
        let mut live = BTreeSet::new();

        for (entity, component) in scripted {
            if !component.enabled {
                continue;
            }
            // The pass walks the entities that were scripted when it began, and
            // a script may despawn another before its turn comes. Running one
            // on an entity that has since gone would mean every write it made
            // reported a dead handle — a game clearing its enemies at the end
            // of a run would produce one failure per enemy, none of them a
            // mistake in the game. What a removed entity does is nothing.
            if Self::lost(&at, entity) {
                continue;
            }
            live.insert(entity);
            let outcome = timed(measuring.then_some(&mut report), &component, || {
                tick(&mut at, entity, &component, delta_seconds)
            });
            collect(&mut report, entity, outcome);
        }

        Self::start_spawned(&mut report, &mut live, &mut at, components, delta_seconds);
        Self::deliver_messages(&mut report, &mut live, &mut at, components, delta_seconds);

        at.tweens.retain(|entity| live.contains(&entity));
        at.running.retain(|entity, _| live.contains(entity));
        // What was waiting for something that never started goes with it.
        let world = &*at.world;
        at.starting.retain(|entity, _| world.get(*entity).is_some());
        let world = &*at.world;
        at.blackboard
            .retain_signals(|bits| world.get(EntityId::from_bits(bits)).is_some());
        report
    }

    /// Delivers the calls scripts made on each other this pass, and the events
    /// they emitted, in the order they were made.
    ///
    /// An event goes to every running script with a handler for it, in the
    /// order the pass runs them — the emitter's own handler included, since an
    /// event is about what happened, not about who noticed.
    ///
    /// After everything else, so a message sees the world its sender left, and
    /// so a message to something spawned this pass reaches a script that has
    /// started. A message may send another; those are delivered in a following
    /// round, up to a bound, and a conversation that never ends is stopped and
    /// reported rather than taking the frame with it.
    fn deliver_messages(
        report: &mut ScriptReport,
        live: &mut BTreeSet<EntityId>,
        at: &mut TickWorld<'_>,
        components: &ComponentSchemaRegistry,
        delta_seconds: f32,
    ) {
        for _ in 0..MESSAGE_ROUNDS {
            let pending = std::mem::take(&mut at.messages);
            if pending.is_empty() {
                return;
            }
            for message in pending {
                match message.to {
                    Some(entity) => collect(report, entity, run::deliver(at, entity, message)),
                    None => {
                        for entity in run::handlers(at, &message.name) {
                            let copy = Message {
                                to: Some(entity),
                                name: message.name.clone(),
                                args: message.args.clone(),
                            };
                            collect(report, entity, run::deliver(at, entity, copy));
                        }
                    }
                }
            }
            // A message may spawn, and what it spawns starts now like anything
            // else spawned this pass.
            Self::start_spawned(report, live, at, components, delta_seconds);
        }
        if !at.messages.is_empty() {
            report.failures.push(ScriptFailure::MessagesDidNotSettle {
                rounds: MESSAGE_ROUNDS,
                waiting: at.messages.len(),
            });
            at.messages.clear();
        }
    }

    /// Whether the world has stopped holding this entity part way through a
    /// pass.
    fn lost(at: &TickWorld<'_>, entity: EntityId) -> bool {
        at.world.get(entity).is_none()
    }

    /// Starts the scripts on entities this pass created, and on entities those
    /// created, until nothing new appears.
    fn start_spawned(
        report: &mut ScriptReport,
        live: &mut BTreeSet<EntityId>,
        at: &mut TickWorld<'_>,
        components: &ComponentSchemaRegistry,
        delta_seconds: f32,
    ) {
        let mut pending: Vec<EntityId> = std::mem::take(&mut at.spawned);
        for round in 0..SPAWN_ROUNDS {
            if pending.is_empty() {
                return;
            }
            for entity in std::mem::take(&mut pending) {
                // Read through the registry rather than the payload, so a
                // spawned script is validated exactly as an authored one is.
                let component = match components.get::<ScriptComponent>(&*at.world, entity) {
                    Ok(Some(component)) => component,
                    Ok(None) => continue,
                    Err(error) => {
                        report
                            .failures
                            .push(ScriptFailure::Registry(error.to_string()));
                        continue;
                    }
                };
                if !component.enabled {
                    continue;
                }
                live.insert(entity);
                collect(report, entity, tick(at, entity, &component, delta_seconds));
            }
            pending = std::mem::take(&mut at.spawned);
            if !pending.is_empty() && round + 1 == SPAWN_ROUNDS {
                report.failures.push(ScriptFailure::SpawnCascade {
                    rounds: SPAWN_ROUNDS,
                    pending: pending.len(),
                });
            }
        }
    }

    /// Every prefab asset the world's scripts could spawn.
    ///
    /// Found through the *declared* type of each `@export` field rather than by
    /// looking for strings that resemble asset IDs: a field declared `Prefab`
    /// is a prefab reference, and one declared `String` is text however much it
    /// looks like a path. That distinction is the whole reason `World.spawn`
    /// refuses text — it is what lets a host load a scene's prefabs before the
    /// first frame instead of discovering them when a script spawns.
    ///
    /// Empty for a source that has not compiled yet, because the declared types
    /// are not known until it has. A host asks again once it has.
    pub fn referenced_prefabs(
        &self,
        world: &World,
        components: &ComponentSchemaRegistry,
    ) -> BTreeSet<String> {
        let mut referenced = BTreeSet::new();
        for (_, component) in components
            .query::<ScriptComponent>(world)
            .unwrap_or_default()
        {
            let Some(exports) = self.exports(&component.source, &component.script) else {
                continue;
            };
            for export in exports {
                if export.type_name.as_deref() != Some(PREFAB) {
                    continue;
                }
                if let Some(serde_json::Value::String(id)) = component.properties.get(&export.name)
                {
                    referenced.insert(id.clone());
                }
            }
        }
        referenced
    }

    /// Every reusable profile named by a typed `@export` field.
    pub fn referenced_profiles(
        &self,
        world: &World,
        components: &ComponentSchemaRegistry,
    ) -> BTreeSet<String> {
        let mut referenced = BTreeSet::new();
        for (_, component) in components
            .query::<ScriptComponent>(world)
            .unwrap_or_default()
        {
            let Some(exports) = self.exports(&component.source, &component.script) else {
                continue;
            };
            for export in exports {
                if export.type_name.as_deref() != Some(PROFILE) {
                    continue;
                }
                if let Some(serde_json::Value::String(id)) = component.properties.get(&export.name)
                {
                    referenced.insert(id.clone());
                }
            }
        }
        referenced
    }

    #[must_use]
    pub fn exports(&self, source: &str, script: &str) -> Option<Vec<ScriptExport>> {
        exports_of(&self.programs.get(source)?.program, script)
    }

    /// The scripts one compiled source declares.
    ///
    /// What an editor offers when asking which script of a file an entity runs.
    /// Empty for a source that has not compiled, which is not the same as a
    /// source declaring nothing — but both leave a panel with no names to
    /// offer, so both are the same answer here.
    #[must_use]
    pub fn declared(&self, source: &str) -> Vec<String> {
        self.programs
            .get(source)
            .map(|compiled| {
                compiled
                    .program
                    .containers
                    .iter()
                    .map(|container| container.name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn clear(&mut self) {
        self.programs.clear();
        self.running.clear();
        self.blackboard.clear();
        self.tweens.clear();
        // Requests from the run being cleared belong to it. Carrying them over
        // would play the previous session's sounds into the next one.
        self.audio.clear();
    }

    #[must_use]
    /// The board, to write on.
    ///
    /// For a host or a test standing in for a script — putting a run into a
    /// state it would otherwise have to be played into, which is the
    /// difference between a test that checks an ending and one that has to
    /// reach it by luck.
    pub const fn blackboard_mut(&mut self) -> &mut Blackboard {
        &mut self.blackboard
    }

    pub const fn blackboard(&self) -> &Blackboard {
        &self.blackboard
    }
}

#[cfg(test)]
mod tests {
    use decay_ir::lower_with_environment;

    use super::environment;

    #[test]
    fn audio_calls_are_type_checked() {
        let source = r#"
            script Sound {
                fn start() {
                    Audio.play("audio/pickup.wav", 0.8);
                    Audio.loop("audio/music.ogg", 0.4);
                    Audio.play_on("voice", "audio/line.wav", 1.0);
                    Audio.loop_on("ambience", "audio/wind.ogg", 0.5);
                    Audio.set_volume("master", Audio.volume("music") * 0.5);
                    Audio.pause_all();
                    Audio.resume_all();
                    Audio.stop_all();
                }
            }
        "#;
        let lowered = lower_with_environment(source, &environment());
        assert!(
            lowered.program.is_some(),
            "{:?}",
            lowered.analysis.diagnostics
        );
    }
}
