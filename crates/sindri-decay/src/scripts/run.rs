//! One tick of the scripts a world holds.

use std::collections::{BTreeMap, BTreeSet};

use decay_ir::{IrContainer, lower_with_environment};
use decay_runtime::{Runtime, ScriptInstance, Value};
use sindri_core::{EntityId, World};
use sindri_platform::InputState;

use crate::{
    Blackboard, Physics2d, PrefabSources, ProfileSources, ScriptComponent, ScriptContext,
    ScriptFailure, WorldHost, audio_host::AudioCommand, host::Peers, host::Spawning,
};

use super::sources::{START, ScriptSources, UPDATE};
use super::{Compiled, Running};

/// Everything a tick needs that is not about which entity it is for.
///
/// Grouped rather than passed one by one because the list had grown past what
/// a reader can hold, and every caller passes the same things.
pub(super) struct TickWorld<'a> {
    pub(super) programs: &'a mut BTreeMap<String, Compiled>,
    pub(super) running: &'a mut BTreeMap<EntityId, Running>,
    pub(super) starting: &'a mut super::StartingValues,
    pub(super) blackboard: &'a mut Blackboard,
    pub(super) audio: &'a mut Vec<AudioCommand>,
    pub(super) world: &'a mut World,
    pub(super) sources: &'a ScriptSources,
    pub(super) prefabs: &'a PrefabSources,
    /// Which scene is being played, and where a script asked to go.
    pub(super) scenes: Option<&'a mut crate::SceneChannel>,
    pub(super) profiles: &'a ProfileSources,
    pub(super) input: &'a InputState,
    /// Which entities have a script instance.
    ///
    /// Owned and updated as the pass runs rather than snapshotted before it.
    /// A snapshot was wrong for the entity being instantiated in the very tick
    /// that reads it: on its first frame a script could author its own
    /// property, change nothing, and be told nothing.
    pub(super) started: BTreeSet<EntityId>,
    /// What every call in this pass has created so far.
    pub(super) spawned: Vec<EntityId>,
    /// Calls scripts made on each other, waiting for the pass to finish.
    pub(super) messages: Vec<super::Message>,
    /// The physics a script may read and drive, when the host runs any.
    pub(super) physics: Option<Physics2d<'a>>,
    /// Where the screen elements are and what the pointer is doing to them.
    pub(super) screen_ui: Option<&'a sindri_scene::ScreenUi>,
    pub(super) aim: Option<sindri_scene::voxel::VolumeAim>,
    /// What the person just did, recognised across frames from the presses.
    pub(super) gestures: Option<&'a sindri_core::Gestures>,
    /// How far this frame's drag asks the camera to move.
    pub(super) camera_pan: Option<[f32; 3]>,
    /// The run's random stream, when the host is running one.
    pub(super) random: Option<&'a mut sindri_core::Rng>,
    /// What the game remembers, when the host is keeping a save.
    pub(super) saves: Option<&'a mut sindri_core::SaveStore>,
    /// The fleck pool, when the host is running one.
    pub(super) effects: Option<&'a mut sindri_scene::Effects2d>,
    /// Where each animated sprite has got to, when the host advances any.
    pub(super) animations: Option<&'a mut sindri_scene::SpriteAnimations>,
    /// What a stacked volume's cells mean, when the host has loaded any.
    pub(super) tile_sets: Option<&'a sindri_scene::TileSetBindings>,
}

pub(super) fn tick(
    at: &mut TickWorld<'_>,
    entity: EntityId,
    component: &ScriptComponent,
    delta_seconds: f32,
) -> Result<Vec<String>, ScriptFailure> {
    ensure_compiled(at.programs, at.sources, entity, component)?;
    let program = std::rc::Rc::clone(&at.programs[&component.source].program);
    let container = program
        .containers
        .iter()
        .find(|container| container.name == component.script)
        .ok_or_else(|| ScriptFailure::UnknownScript {
            entity,
            asset: component.source.clone(),
            script: component.script.clone(),
        })?;

    let elapsed_seconds = at
        .running
        .get(&entity)
        .map_or(0.0, |current| current.elapsed_seconds)
        + delta_seconds;
    let fresh = !at.running.get(&entity).is_some_and(|current| {
        current.source == component.source && current.script == component.script
    });
    // Recorded before any of this script's code runs, because by then the
    // instance either exists or is being built, and either way its authored
    // properties have been decided.
    at.started.insert(entity);

    // Taken out of the map for the call, so the host can lend every *other*
    // running script to this one, and put back whatever happens.
    let mut current = if fresh {
        None
    } else {
        at.running.remove(&entity)
    };
    let mut starting_values = if fresh {
        at.starting.remove(&entity)
    } else {
        None
    };
    let context = ScriptContext {
        input: at.input,
        delta_seconds,
        elapsed_seconds,
    };
    let mut runtime = Runtime::new(&program, host_for(at, entity, context));
    let outcome = (|| {
        if fresh {
            let mut instance = runtime.instantiate(&component.script).map_err(|error| {
                ScriptFailure::runtime(entity, &component.script, START, &error)
            })?;
            apply_properties(&mut instance, container, entity, component)?;
            // Then what another script set on it before it started, which is
            // later than the scene and so wins.
            for (field, value) in starting_values.take().unwrap_or_default() {
                instance.set_field(&field, value).map_err(|error| {
                    ScriptFailure::runtime(entity, &component.script, START, &error)
                })?;
            }
            current = Some(Running {
                elapsed_seconds,
                source: component.source.clone(),
                script: component.script.clone(),
                instance,
            });
        }
        let Some(running) = current.as_mut() else {
            return Ok(());
        };
        running.elapsed_seconds = elapsed_seconds;
        if fresh && container.functions.iter().any(|f| f.name == START) {
            runtime
                .call_instance(&mut running.instance, START, vec![])
                .map_err(|error| {
                    ScriptFailure::runtime(entity, &component.script, START, &error)
                })?;
        }
        // Time passes for a script that was already running: every timer its
        // fields hold runs down by this frame, before it looks at any. One
        // just started has not lived through the frame, so its timers start
        // full.
        if !fresh {
            running.instance.advance_timers(f64::from(delta_seconds));
        }
        if container.functions.iter().any(|f| f.name == UPDATE) {
            runtime
                .call_instance(
                    &mut running.instance,
                    UPDATE,
                    vec![Value::Number(f64::from(delta_seconds))],
                )
                .map_err(|error| {
                    ScriptFailure::runtime(entity, &component.script, UPDATE, &error)
                })?;
        }
        Ok(())
    })();
    let printed = runtime.into_host().take_printed();
    if let Some(current) = current {
        at.running.insert(entity, current);
    }
    outcome.map(|()| printed)
}

/// Runs a message another script sent: `function` on `entity`'s script, with
/// the arguments it was sent with.
///
/// Nothing happens for an entity that has gone, or whose script is not
/// running: a message to something destroyed this frame is the ordinary end of
/// a projectile, not a mistake to report.
pub(super) fn deliver(
    at: &mut TickWorld<'_>,
    entity: EntityId,
    message: super::Message,
) -> Result<Vec<String>, ScriptFailure> {
    if at.world.get(entity).is_none() {
        return Ok(Vec::new());
    }
    let Some(mut current) = at.running.remove(&entity) else {
        return Ok(Vec::new());
    };
    let Some(compiled) = at.programs.get(&current.source) else {
        at.running.insert(entity, current);
        return Ok(Vec::new());
    };
    let program = std::rc::Rc::clone(&compiled.program);
    let script = current.script.clone();
    let context = ScriptContext {
        input: at.input,
        delta_seconds: 0.0,
        elapsed_seconds: current.elapsed_seconds,
    };
    let mut runtime = Runtime::new(&program, host_for(at, entity, context));
    let outcome = runtime
        .call_instance(&mut current.instance, &message.name, message.args)
        .map(|_| ())
        .map_err(|error| ScriptFailure::runtime(entity, &script, &message.name, &error));
    let printed = runtime.into_host().take_printed();
    at.running.insert(entity, current);
    outcome.map(|()| printed)
}

/// Every running script with a function of this name, in the order a pass
/// runs them: who an event is delivered to.
pub(super) fn handlers(at: &TickWorld<'_>, function: &str) -> Vec<EntityId> {
    at.running
        .iter()
        .filter(|(_, running)| {
            at.programs.get(&running.source).is_some_and(|compiled| {
                compiled.program.containers.iter().any(|container| {
                    container.name == running.script
                        && container.functions.iter().any(|f| f.name == function)
                })
            })
        })
        .map(|(entity, _)| *entity)
        .collect()
}

/// The host one call on `entity` runs against, with every other running
/// script reachable through it.
fn host_for<'b>(
    at: &'b mut TickWorld<'_>,
    entity: EntityId,
    context: ScriptContext<'b>,
) -> WorldHost<'b> {
    WorldHost::new(
        &mut *at.world,
        entity,
        context,
        &mut *at.blackboard,
        crate::HostServices {
            spawning: Spawning {
                prefabs: at.prefabs,
                started: &at.started,
                spawned: &mut at.spawned,
            },
            profiles: at.profiles,
            // Reborrowed per tick rather than moved: every script in the
            // pass reads the same frame's events and drives the same world,
            // and one taking physics away from the rest would make which
            // script ran first decide what the others could do.
            physics: at.physics.as_mut().map(|physics| Physics2d {
                world: &mut *physics.world,
                events: physics.events,
            }),
            screen_ui: at.screen_ui,
            aim: at.aim,
            gestures: at.gestures,
            camera_pan: at.camera_pan,
            // Reborrowed per tick like physics: one stream, shared by every
            // script in the pass, so a run's numbers are the run's.
            random: at.random.as_deref_mut(),
            saves: at.saves.as_deref_mut(),
            effects: at.effects.as_deref_mut(),
            // Reborrowed per tick like the rest: every script in the pass
            // reads the same step's playback, and one taking the cursors
            // away from the others would make which script ran first decide
            // what the rest could see.
            animations: at.animations.as_deref_mut(),
            tile_sets: at.tile_sets,
            audio: &mut *at.audio,
            // Reborrowed per tick like the rest: one channel for the pass,
            // so which script ran first does not decide who may ask.
            scenes: at.scenes.as_deref_mut(),
        },
    )
    .with_peers(Peers {
        running: &mut *at.running,
        starting: &mut *at.starting,
        project: at.sources.project_ref(),
        messages: &mut at.messages,
    })
}

pub(super) fn ensure_compiled(
    programs: &mut BTreeMap<String, Compiled>,
    sources: &ScriptSources,
    entity: EntityId,
    component: &ScriptComponent,
) -> Result<(), ScriptFailure> {
    let Some(source) = sources.get(&component.source) else {
        return Err(ScriptFailure::MissingSource {
            entity,
            asset: component.source.clone(),
        });
    };
    if programs.get(&component.source).is_some_and(|compiled| {
        compiled.source == source && compiled.project == sources.project_key()
    }) {
        return Ok(());
    }

    let lowered = lower_with_environment(source, sources.environment());
    let program = lowered.program.ok_or_else(|| ScriptFailure::Compile {
        asset: component.source.clone(),
        diagnostics: lowered
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{}:{}: {}",
                    diagnostic.line, diagnostic.column, diagnostic.message
                )
            })
            .collect(),
    })?;
    programs.insert(
        component.source.clone(),
        Compiled {
            source: source.to_owned(),
            project: sources.project_key().to_owned(),
            program: std::rc::Rc::new(program),
        },
    );
    Ok(())
}

pub(super) fn apply_properties(
    instance: &mut ScriptInstance,
    container: &IrContainer,
    entity: EntityId,
    component: &ScriptComponent,
) -> Result<(), ScriptFailure> {
    let refuse = |property: &str, reason: &str| ScriptFailure::Property {
        entity,
        script: component.script.clone(),
        property: property.to_owned(),
        reason: reason.to_owned(),
    };

    for (name, value) in &component.properties {
        let Some(field) = container.fields.iter().find(|field| field.name == *name) else {
            return Err(refuse(name, "the script declares no such field"));
        };
        if !field.exported {
            return Err(refuse(name, "the field is not @export"));
        }
        let value = to_value(value).ok_or_else(|| {
            refuse(
                name,
                &format!("{value} is not a number, string, boolean or vector"),
            )
        })?;
        instance
            .set_field(name, value)
            .map_err(|error| refuse(name, &format!("{error:?}")))?;
    }
    Ok(())
}

pub(crate) fn to_value(value: &serde_json::Value) -> Option<Value> {
    Some(match value {
        serde_json::Value::Number(number) => Value::Number(number.as_f64()?),
        serde_json::Value::Bool(value) => Value::Bool(*value),
        serde_json::Value::String(value) => Value::String(value.clone()),
        serde_json::Value::Null => Value::Null,
        // A vector is stored as its components, the way a transform stores a
        // position: `[x, y]` or `[x, y, z]`.
        serde_json::Value::Array(items) => Value::vector(
            &items
                .iter()
                .map(serde_json::Value::as_f64)
                .collect::<Option<Vec<f64>>>()?,
        )?,
        serde_json::Value::Object(_) => return None,
    })
}
