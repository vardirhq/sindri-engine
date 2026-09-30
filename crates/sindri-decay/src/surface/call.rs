//! What a script can call, and what each call answers with.
//!
//! One list per host type. A new call is an entry in the matching list
//! and an arm where the host dispatches it; nothing else moves.

/// The name a script calls to say something into the host's log.
pub(crate) const PRINT: &str = "print";

/// A note left on the board shared by every script in the world.
///
/// The smallest thing that lets two scripts cooperate. Decay has no value that
/// can hold an entity, so a script cannot name another one — but it can leave a
/// number under a name and another can read it. That is enough for a player to
/// publish where it is, a collectible to notice, and a score to be counted by
/// nobody in particular.
///
/// Deliberately a stopgap with a shape that admits it: names are strings and
/// nothing checks them, which typed cross-entity access would fix. It is here
/// because it is small and it unblocks a game, and a game is what tells us
/// which of the bigger answers is worth building.
#[derive(Clone, Copy, Debug)]
pub(crate) enum GameCall {
    /// `Game.get(name, fallback)`.
    ///
    /// The fallback is not optional, because a note nobody has left yet is the
    /// ordinary case on the first frame — and a `get` that silently answered
    /// zero would be a typo that reads as a legitimate value.
    Get,
    /// `Game.set(name, value)`.
    Set,
}

pub(crate) const GAME_CALLS: &[(&str, GameCall)] =
    &[("get", GameCall::Get), ("set", GameCall::Set)];

/// A typed read from reusable authored data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProfileCall {
    Name,
    Kind,
    Number,
    Text,
    Flag,
    Count,
    NumberAt,
    TextAt,
    FlagAt,
}

pub(crate) const PROFILE_CALLS: &[(&str, ProfileCall)] = &[
    ("name", ProfileCall::Name),
    ("kind", ProfileCall::Kind),
    ("number", ProfileCall::Number),
    ("text", ProfileCall::Text),
    ("flag", ProfileCall::Flag),
    ("count", ProfileCall::Count),
    ("number_at", ProfileCall::NumberAt),
    ("text_at", ProfileCall::TextAt),
    ("flag_at", ProfileCall::FlagAt),
];

/// What a script can ask of the world it is in, as opposed to of one entity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorldCall {
    /// Finds an entity by the name the scene gave it, or `null`.
    ///
    /// By name rather than by scene ID because a name is what an author typed
    /// and can see in the hierarchy, and because a runtime-spawned entity has
    /// no scene ID at all.
    Find,
    /// Removes an entity and everything under it, through `WorldCommand` so it
    /// can be undone.
    Despawn,
    /// Whether a reference still names something. A reference outlives what it
    /// names — that is what generation checking is for — so a script that holds
    /// one across frames needs to be able to ask.
    Exists,
    /// Creates the entities an authored prefab describes and answers with its
    /// root.
    ///
    /// By asset ID rather than by a value, because a prefab is a file the
    /// project holds and Decay has no literal for one. A name the host has not
    /// loaded is refused at the call: a spawn that silently produced nothing is
    /// a bug report nobody can reproduce.
    Spawn,
    /// Creates a prefab and makes its root a local child before the spawned
    /// script is eligible to start.
    ///
    /// This is the common attachment operation in one typed call. The prefab's
    /// authored root transform stays local to the new parent, while any
    /// descendants the prefab already contains remain under that root.
    SpawnChild,
    /// Puts one entity under another, or at the root when given `null`.
    ///
    /// Separate from `spawn` rather than an argument to it, because reparenting
    /// is a thing a script wants to do to entities it did not create.
    SetParent,
    /// Replaces one vertex of the current script entity's world polygon.
    ///
    /// Points are local shape coordinates and the index is zero through seven.
    /// The bounded call is enough to author compact procedural silhouettes
    /// without giving Decay a general JSON escape hatch.
    SetShapePoint,
    /// Authors an `@export` property on an entity whose script has not started.
    ///
    /// The one thing a spawner cannot do with the paths it already has. A
    /// script's own fields are not on the surface — the analyzer cannot know
    /// which container another entity runs — so a per-instance starting value
    /// is set the way the scene sets one, by writing the authored property the
    /// instance is built from.
    ///
    /// Refused once the script is running. Properties are applied when the
    /// instance is created, so a later write would land in the payload and
    /// change nothing a script could see, which is the silent no-op this whole
    /// surface is arranged to avoid.
    SetProperty,
    /// Reads a numeric authored property from another entity's script.
    ///
    /// This is the other half of `set_property`: a projectile can carry the
    /// damage it was spawned with, and the target it reaches can read that
    /// value without routing a per-instance fact through the global board.
    /// The fallback is required so an optional property and a misspelling do
    /// not both silently become zero.
    PropertyNumber,
    /// Sends an accumulating numeric runtime signal to one entity.
    SendSignal,
    /// Takes this entity's accumulated signal, or zero when none is pending.
    TakeSignal,
    /// Every active entity carrying an authored tag, in world order.
    WithTag,
    /// The closest active tagged entity to a world-space point, or `null`.
    Nearest,
    /// Active tagged entities no farther than a world-space radius, nearest first.
    WithinRadius,
    /// Whether one active entity carries an authored tag.
    HasTag,
    SetActive,
    IsActive,
}

pub(crate) const WORLD_CALLS: &[(&str, WorldCall)] = &[
    ("find", WorldCall::Find),
    ("despawn", WorldCall::Despawn),
    ("exists", WorldCall::Exists),
    ("spawn", WorldCall::Spawn),
    ("spawn_child", WorldCall::SpawnChild),
    ("set_parent", WorldCall::SetParent),
    ("set_shape_point", WorldCall::SetShapePoint),
    ("set_property", WorldCall::SetProperty),
    ("property_number", WorldCall::PropertyNumber),
    ("send_signal", WorldCall::SendSignal),
    ("take_signal", WorldCall::TakeSignal),
    ("with_tag", WorldCall::WithTag),
    ("nearest", WorldCall::Nearest),
    ("within_radius", WorldCall::WithinRadius),
    ("has_tag", WorldCall::HasTag),
    ("set_active", WorldCall::SetActive),
    ("is_active", WorldCall::IsActive),
];

/// A conversion between an entity's world position and a tilemap's logical
/// coordinate space.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GridCall {
    PositionX, PositionY, Place, CanReach, StepToward, Tile, SetTile, Columns, Rows,
    Block, SetBlock, Walkable, Tagged,
}
impl GridCall { pub(crate) const fn is_about_a_cell(self) -> bool { matches!(self, Self::Tile | Self::SetTile | Self::Columns | Self::Rows | Self::Block | Self::SetBlock | Self::Walkable | Self::Tagged) } }
#[derive(Clone, Copy, Debug, Eq, PartialEq)] pub(crate) enum SceneCall { Go, Current }
pub(crate) const SCENE_CALLS: &[(&str, SceneCall)] = &[("go", SceneCall::Go), ("current", SceneCall::Current)];
pub(crate) const GRID_CALLS: &[(&str, GridCall)] = &[("position_x",GridCall::PositionX),("position_y",GridCall::PositionY),("place",GridCall::Place),("can_reach",GridCall::CanReach),("step_toward",GridCall::StepToward),("tile",GridCall::Tile),("set_tile",GridCall::SetTile),("columns",GridCall::Columns),("rows",GridCall::Rows),("block",GridCall::Block),("set_block",GridCall::SetBlock),("walkable",GridCall::Walkable),("tagged",GridCall::Tagged)];
#[derive(Clone, Copy, Debug, Eq, PartialEq)] pub(crate) enum PhysicsCall { VelocityX, VelocityY, SetVelocity, ApplyImpulse, ConnectDistance, CollisionStarted, CollisionStopped, SensorEntered, SensorExited }
pub(crate) const PHYSICS_CALLS: &[(&str, PhysicsCall)] = &[("velocity_x",PhysicsCall::VelocityX),("velocity_y",PhysicsCall::VelocityY),("set_velocity",PhysicsCall::SetVelocity),("apply_impulse",PhysicsCall::ApplyImpulse),("connect_distance",PhysicsCall::ConnectDistance),("collision_started",PhysicsCall::CollisionStarted),("collision_stopped",PhysicsCall::CollisionStopped),("sensor_entered",PhysicsCall::SensorEntered),("sensor_exited",PhysicsCall::SensorExited)];
impl PhysicsCall { pub(crate) const fn is_event(self)->bool { matches!(self,Self::CollisionStarted|Self::CollisionStopped|Self::SensorEntered|Self::SensorExited) } }
#[derive(Clone, Copy, Debug, Eq, PartialEq)] pub(crate) enum UiCall { Text, Number, Numbers, Fill, SliderValue, SliderSetValue, SliderChanged, Hovered, Pressed, Held }
impl UiCall { pub(crate) const fn is_query(self)->bool { matches!(self,Self::Hovered|Self::Pressed|Self::Held|Self::SliderChanged) } }
pub(crate) const UI_CALLS:&[(&str,UiCall)]=&[("set_text",UiCall::Text),("set_number",UiCall::Number),("set_numbers",UiCall::Numbers),("set_fill",UiCall::Fill),("slider_value",UiCall::SliderValue),("set_slider_value",UiCall::SliderSetValue),("slider_changed",UiCall::SliderChanged),("is_hovered",UiCall::Hovered),("is_pressed",UiCall::Pressed),("is_held",UiCall::Held)];
#[derive(Clone, Copy, Debug, Eq, PartialEq)] pub(crate) enum AnimationCall { Play, Restart, Stop, Finished, Frame, Clip, Speed }
pub(crate) const ANIMATION_CALLS:&[(&str,AnimationCall)]=&[("play",AnimationCall::Play),("restart",AnimationCall::Restart),("stop",AnimationCall::Stop),("is_finished",AnimationCall::Finished),("frame",AnimationCall::Frame),("clip",AnimationCall::Clip),("set_speed",AnimationCall::Speed)];
#[derive(Clone, Copy, Debug, Eq, PartialEq)] pub(crate) enum RandomCall { Value, Range, Int, Pick, Seed }
pub(crate) const RANDOM_CALLS:&[(&str,RandomCall)]=&[("value",RandomCall::Value),("range",RandomCall::Range),("int",RandomCall::Int),("pick",RandomCall::Pick),("seed",RandomCall::Seed)];
#[derive(Clone, Copy, Debug, Eq, PartialEq)] pub(crate) enum SaveCall { Number, SetNumber, Flag, SetFlag, Has, Clear, IsNew, IsDamaged, IsFromNewer }
pub(crate) const SAVE_CALLS:&[(&str,SaveCall)]=&[("number",SaveCall::Number),("set_number",SaveCall::SetNumber),("flag",SaveCall::Flag),("set_flag",SaveCall::SetFlag),("has",SaveCall::Has),("clear",SaveCall::Clear),("is_new",SaveCall::IsNew),("is_damaged",SaveCall::IsDamaged),("is_from_newer",SaveCall::IsFromNewer)];
#[derive(Clone, Copy, Debug, Eq, PartialEq)] pub(crate) enum EffectsCall { Burst, BurstAt, Live }
pub(crate) const EFFECTS_CALLS:&[(&str,EffectsCall)]=&[("burst",EffectsCall::Burst),("burst_at",EffectsCall::BurstAt),("live",EffectsCall::Live)];
#[derive(Clone, Copy, Debug)] pub(crate) enum TimeValue { Delta, Elapsed }
pub(crate) const TIME_VALUES:&[(&str,TimeValue)]=&[("delta",TimeValue::Delta),("elapsed",TimeValue::Elapsed)];
