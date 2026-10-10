//! What a Decay script can reach, and nothing else.
//!
//! Decay knows paths, not engine concepts: `this.transform.position.x` reaches
//! the runtime as four strings the IR never interprets. This is the only place
//! that gives those strings a meaning, which is what keeps the language
//! replaceable — swapping Decay for something else costs this directory and
//! the syntax, not the architecture.
//!
//! What the surface *is* lives in [`crate::surface`], and both this and the
//! analyzer's view of it are derived from there. Nothing here decides which
//! paths exist; it decides only how to reach one.

mod access;
mod animation;
mod blocks;
mod call;
mod convert;
mod dispatch;
mod effects;
mod flood;
mod frame;
mod gamepad;
mod geometry;
mod map;
mod peers;

pub(crate) use peers::Peers;
mod actions;
mod camera_3d;
mod person;
mod physics;
mod physics3d;
mod physics3d_layers;
mod physics3d_motion;
mod physics3d_query;
mod physics_character;
mod physics_contacts;
mod physics_controls;
mod physics_gravity;
mod physics_hinge_position;
mod physics_joint_creation;
mod physics_joint_endpoints;
mod physics_joint_state;
mod physics_joints;
mod physics_motion;
mod physics_slider_position;
mod print;
mod profile;
mod query;
mod random;
mod raycast;
mod save;
mod scene;
mod sequence;
mod services;
mod shape_query;
mod shared;
mod tiles;
mod transform;
mod tween;
mod ui;
mod widgets;

pub use services::WorldServices;

use std::collections::BTreeSet;

use sindri_core::{EntityId, World};
use sindri_platform::InputState;

use crate::{Blackboard, PrefabSources, ProfileSources};

/// What a script can know about the frame it is running in.
///
/// Passed per call rather than held, because none of it belongs to the script:
/// the input is the host's, and the clock is the host's.
#[derive(Clone, Copy)]
pub struct ScriptContext<'a> {
    pub input: &'a InputState,
    pub delta_seconds: f32,
    pub elapsed_seconds: f32,
}

/// What `World.spawn` needs, and where what it makes is recorded.
///
/// Grouped rather than three more parameters, because they are one thing: the
/// prefabs a spawn may name, the instances that already exist, and the list the
/// runner reads back to know what to start.
pub struct Spawning<'a> {
    /// What `World.spawn` can name.
    pub prefabs: &'a PrefabSources,
    /// Which entities already have a script instance.
    ///
    /// A snapshot taken before the pass rather than the live map, which is
    /// borrowed by the runner for the length of the call. Stale only in the
    /// safe direction: an entity spawned during the pass is genuinely not
    /// running yet, because instantiating one would mean executing Decay from
    /// inside a host call.
    pub started: &'a BTreeSet<EntityId>,
    /// What this pass has created, for the runner to start.
    pub spawned: &'a mut Vec<EntityId>,
}

/// The world, seen through one entity's script.
///
/// Borrowed for the length of a single script call rather than held, because a
/// script may write to the world and the world cannot be lent out twice.
pub struct WorldHost<'a> {
    world: &'a mut World,
    entity: EntityId,
    context: ScriptContext<'a>,
    /// The notes every script in the world shares.
    blackboard: &'a mut Blackboard,
    /// What spawning needs, and what it produced.
    spawning: Spawning<'a>,
    profiles: &'a ProfileSources,
    /// The physics a script may read and drive, when the host runs any.
    physics: Option<crate::Physics2d<'a>>,
    pub(crate) physics3d: Option<crate::Physics3d<'a>>,
    pub(crate) characters: Option<crate::Characters2d<'a>>,
    /// What the game remembers, when the host is keeping a save.
    saves: Option<&'a mut sindri_core::SaveStore>,
    /// The fleck pool, when the host is running one.
    effects: Option<&'a mut sindri_scene::Effects2d>,
    /// The run's random stream, when the host is running one.
    ///
    /// Mutable because drawing a number is what advances it: a stream a script
    /// could read without moving would hand out the same number for ever.
    random: Option<&'a mut sindri_core::Rng>,
    /// What a stacked volume's cells mean, when the host has loaded any.
    tile_sets: Option<&'a sindri_scene::TileSetBindings>,
    /// The walkable surface last derived, and the volume it came from.
    ///
    /// One update asks several times -- "can I step here" is asked once per
    /// axis, so a walker refused diagonally still slides along a wall -- and
    /// reducing a whole island's columns to their walkable tops per question
    /// would cost more than drawing it. Keyed by the grid and its revision, so
    /// a volume a script just wrote to is derived again rather than remembered.
    walkable: Option<(EntityId, u64, sindri_scene::TileSurfaces)>,
    /// Where the screen elements are and what the pointer is doing to them.
    ///
    /// Read-only: hover and click are answers about this frame, computed by the
    /// host before scripts ran. A script that could change them would be
    /// deciding what the person did.
    screen_ui: Option<&'a sindri_scene::ScreenUi>,
    /// Which block the person is pointing at, decided before scripts ran.
    ///
    /// Read-only and for the same reason `screen_ui` is: what the pointer is
    /// over is an answer about this frame, worked out from a camera and a
    /// pointer that no script owns. A script that could set it would be
    /// deciding what the person was looking at.
    aim: Option<sindri_scene::voxel::VolumeAim>,
    /// What the person just did, as an intention rather than as a button.
    gestures: Option<&'a sindri_core::Gestures>,
    /// How far this frame's drag asks the camera to move.
    camera_pan: Option<[f32; 3]>,
    /// Where each animated sprite has got to, when the host advances any.
    ///
    /// Mutable for one call: playing the clip already playing has to reset the
    /// cursor, because naming it again is not a change to the world and nothing
    /// else would notice. Everything else here is read.
    animations: Option<&'a mut sindri_scene::SpriteAnimations>,
    /// Where each playing sequence has got to, when the host advances any.
    /// Mutable for `Sequence.restart`; everything else here is read.
    pub(crate) sequences: Option<&'a mut sindri_scene::Sequences>,
    /// Which scene is being played, and where a script asked to go.
    ///
    /// Mutable because asking is a write: `Scene.go` records an intention the
    /// host performs between frames, since the change would rearrange the world
    /// the asking script is still running in.
    scenes: Option<&'a mut crate::SceneChannel>,
    /// What the script said, in order. Drained by the caller after the call.
    printed: Vec<String>,
    pointer_lock_request: Option<bool>,
    /// The other scripts in the pass, for a host that runs several.
    peers: Option<peers::Peers<'a>>,
    pub(crate) tweens: Option<&'a mut crate::tweens::Tweens>,
    pub(crate) actions: Option<&'a crate::actions::InputActions>,
}
