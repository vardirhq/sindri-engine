//! Calls over entities in the world.

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
    ///
    /// The answer to "a game cannot hold a reference to each of three hundred
    /// enemies". By tag rather than by name, because `find` matches the name a
    /// scene gave *one* entity and a game whose enemies are "Scout 41" through
    /// "Scout 300" has three hundred authored names and no way to say "the
    /// enemies". By tag rather than by component type, because spelling
    /// `sindri.sprite` in a script puts engine internals in gameplay code and
    /// makes every enemy that happens to have a sprite an enemy.
    ///
    /// Bounded, ordered, and a snapshot. See `docs/scripting.md` for what each
    /// of those costs and buys.
    WithTag,
    /// Closest active tagged spatial entity to a world-space `Vec3`, or `null`.
    Nearest,
    /// Active tagged spatial entities inside an inclusive world-space radius.
    /// Sorted by distance, with equal distances retaining world order.
    WithinRadius,
    /// Whether one active entity carries an authored tag.
    HasTag,
    /// Switches an entity — and everything under it — on or off.
    ///
    /// `docs/scripting.md` says a screen is an entity with children and that
    /// showing one is switching it on. That was true of the engine and not of
    /// Decay: a script could make and destroy entities but not hide one, so a
    /// title screen could be authored and never dismissed, and the only way to
    /// remove a menu was to despawn it and lose the ability to show it again.
    SetActive,
    /// Whether an entity takes part in the scene.
    ///
    /// The question `set_active` answers, which a script needs to toggle a
    /// pause overlay rather than track a truth that the world already holds
    /// and that something else may have changed.
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
