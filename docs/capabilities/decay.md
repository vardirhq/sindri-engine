<!-- Generated from docs/capabilities/decay.toml by `cargo run -p sindri-capabilities -- --write`. Edit the source, not this file. -->

# Decay

The gameplay language, in `decay/` — a **separate Cargo workspace**, not a
member of this one. Nothing under `decay/` depends on a `sindri-*` crate and no
engine crate depends on Decay. `sindri-decay` binds it to a world. The whole
host table is `docs/scripting.md`; the language is `decay/LANGUAGE.md`; the
generated host surface is `docs/generated/decay-api.md`.

Host calls that belong to one subsystem — physics, audio, UI, grids, effects,
saves, randomness — are listed with that subsystem's capability, not here.

Each entry links its tests and the projects that exercise it. Surface marks match the row in [parity](../parity.md) where one is named: ✅ works and is exercised · 🟡 a useful slice, with the gap named · ❌ absent · — not applicable.

| Capability | Engine | Editor | Decay | Proof |
| --- | :-: | :-: | :-: | :-: |
| **Scripts and the host** | | | | |
| [Typed host, compile / run / reload](#typed-host) | ✅ | 🟡 | ✅ | ✅ |
| [Safe entity, prefab, profile references](#references) | ✅ | ✅ | ✅ | ✅ |
| [Scripts reach each other by type](#scripts-by-type) | ✅ | ✅ | ✅ | ✅ |
| [Events (emit, handle anywhere)](#events) | ✅ | ✅ | ✅ | ✅ |
| [Shared game state, declared and typed](#shared-state) | 🟡 | ✅ | 🟡 | 🟡 |
| [Tags and queries](#tag-queries) | ✅ | ✅ | ✅ | ✅ |
| [Deterministic gameplay nearest / radius queries](#spatial-queries) | ✅ | — | ✅ | ✅ |
| [Camera impacts are gameplay](#camera-trauma) | ✅ | — | ✅ | ✅ |
| [Tweening / easing](#tweening) | ✅ | 🟡 | ✅ | ✅ |
| **Language** | | | | |
| [Vectors as values (`Vec2`, `Vec3`)](#vectors) | ✅ | 🟡 | ✅ | ✅ |
| [Timers as values](#timers) | ✅ | 🟡 | ✅ | ✅ |
| [Shared helper functions across files](#shared-functions) | ✅ | ✅ | ✅ | ✅ |
| [Constants](#constants) | ✅ | ✅ | ✅ | ✅ |
| [Structs](#structs) | ✅ | ✅ | ✅ | ✅ |
| [Lists and ranges](#lists) | ✅ | ✅ | ✅ | ✅ |
| [Maps](#maps) | ✅ | ✅ | ✅ | ✅ |
| [Text: joining and methods](#decay-text) | ✅ | ✅ | ✅ | ✅ |
| [Enums and exhaustive `match`](#enums) | ✅ | ✅ | ✅ | ✅ |
| [Optional values](#optionals) | ✅ | ✅ | ✅ | ✅ |
| [Colours](#colours) | ✅ | ✅ | ✅ | ✅ |
| [Parser, analyzer, IR and runtime](#language-pipeline) | ✅ | — | ✅ | ✅ |
| **Tooling** | | | | |
| [Language server: highlight, complete, hover, definitions, diagnostics](#language-server) | — | 🟡 | ✅ | ✅ |
| **Not yet** | | | | |
| [Formatter](#formatter) | ❌ | ❌ | ❌ | — |
| [Debugger (breakpoints, stepping)](#debugger) | ❌ | ❌ | ❌ | — |
| [Coroutines / sequencing](#coroutines) | ❌ | — | ❌ | — |
| [Script unit tests](#script-unit-tests) | ❌ | ❌ | ❌ | ❌ |
| [Sets, closures and first-class functions](#closures) | — | — | ❌ | — |
| [TypeScript embedding SDK](#typescript-sdk) | ❌ | — | — | — |
| [Fractional component writes from integer-authored numbers](#generic-fractional-component-writes) | ❌ | — | ❌ | ❌ |
| [Quaternion composition and interpolation](#quaternion-operations) | ❌ | — | ❌ | ❌ |
| **3D transform values** | | | | |
| [Rotations as values](#rotations) | 🟡 | — | 🟡 | 🟡 |

## Scripts and the host

<a id="typed-host"></a>

### Typed host, compile / run / reload

`sindri.script` binds a source to an entity; paths are typed against one description shared by analyzer and host, and sources hot-reload.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Typed host, compile / run / reload*

**Tested by:** [`crates/sindri-decay/src/surface/tests.rs`](../../crates/sindri-decay/src/surface/tests.rs), [`crates/sindri-decay/tests/documented_surface.rs`](../../crates/sindri-decay/tests/documented_surface.rs), [`crates/sindri-runtime/tests/a_session_steps_a_scene.rs`](../../crates/sindri-runtime/tests/a_session_steps_a_scene.rs), [`crates/sindri-decay/tests/spatial_transforms.rs`](../../crates/sindri-decay/tests/spatial_transforms.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked), [`games/scorchball`](../../games/scorchball), [`games/platformer`](../../games/platformer), [`examples/transform`](../../examples/transform)

**See:** [`docs/scripting.md`](../../docs/scripting.md), [`docs/generated/decay-api.md`](../../docs/generated/decay-api.md)

**Limits:**

- No script state migration across a reload: a changed file recompiles, and the running instance keeps whatever fields it had.
- There is no general dynamic component API; purpose-built typed paths cover each subsystem.
- Despawning is not undoable — no script write is; play mode restores from a snapshot.

<details markdown="1">
<summary>Evidence</summary>

`sindri-decay` binds the language to a world: a `sindri.script` component names
a source and a container, `WorldHost` gives Decay's symbolic paths a meaning in
terms of one entity's transform, and the editor runs every script once a frame
with whatever the transport says a frame is worth. Authored `@export`
properties reach the script before its first line. Sources load through
`sindri-assets` and hot-reload from the same `AssetWatch` the textures use.

A script reaches its own transform's position, scale and Z rotation, its
sprite's tint and layer, the keyboard, the frame's delta and its own elapsed
time, logical grid position through an explicit tilemap entity, maths
functions (`abs`, `sqrt`, `sin`, `cos`, `atan2`, `exp`, `min`, `max`, `floor`,
`ceil`, `round`, `sign`, `clamp`, `lerp`, `PI` and `TAU`), and `print`.
**Those paths are typed**, so `this.transfrom.position.x` is a compile error
with a line number rather than a first-frame failure, and reaching for a method
on a container says what to write instead. The analyzer's view and the host's
accessors are derived from one description, and a test walks every path the
analyzer accepts to assert the host answers it.

A failing script reports itself and does not stop the others.


The 3D transform surface additionally offers read-only world-space unit
`forward`, `right`, and `up`, and checked `look_at(target)` and
`rotate_around(pivot, axis, radians)` methods on own and referenced entities.
The script regressions cover rotated/scaled parents, normalized directions,
copy-versus-write semantics, and atomic errors including the Z lock. The
[Transform Lab](../../examples/transform/README.md) opens a perspective scene
and runs to its goal.
This is slice 6 of `docs/3d-update.md`, a foundation for Explorer rather than
completed game proof: no in-tree game uses these new methods yet. Quaternion
composition/interpolation remains pending; character and orbit-camera feature
proof is documented below.

`transform.rotation` is a copied parent-relative `Quat` x/y/z/w value, using
Decay's existing struct constructor and copy semantics. Whole writes normalize
finite nonzero values atomically; zero/non-finite writes retain the transform.
Stored components are readable; edit a copied local value and assign it back.
Script regressions cover huge/tiny magnitudes, invalid input, parented rotation,
copy independence and runtime rejection of direct component writes. Transform
Lab saves its rotation, transfers its facing to the goal and restores its original
on arrival through Session and exported Chromium. Generated host metadata exposes
the type to the checker and LSP. This is a general transform capability added to
finish the planned 3D scripting surface; Explorer game proof remains pending.

</details>

<a id="references"></a>

### Safe entity, prefab, profile references

Opaque references a script holds, passes and reaches through, checked at compile time and reported when stale.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Safe entity, prefab, profile references*

**Tested by:** [`crates/sindri-decay/tests/finding_an_entity_by_name.rs`](../../crates/sindri-decay/tests/finding_an_entity_by_name.rs), [`crates/sindri-decay/tests/one_script_reaches_another_entity.rs`](../../crates/sindri-decay/tests/one_script_reaches_another_entity.rs), [`crates/sindri-decay/tests/a_script_makes_an_entity.rs`](../../crates/sindri-decay/tests/a_script_makes_an_entity.rs), [`crates/sindri-decay/tests/a_script_reads_a_profile.rs`](../../crates/sindri-decay/tests/a_script_reads_a_profile.rs), [`crates/sindri-decay/tests/a_struct_holds_an_entity.rs`](../../crates/sindri-decay/tests/a_struct_holds_an_entity.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

<details markdown="1">
<summary>Evidence</summary>

`Value::Reference` is a value Decay can hold, pass, compare and store but
cannot construct or look inside; the engine packs a runtime handle into it.
`World.find` looks one up by the name a scene gave it, `World.exists` asks
whether it still names anything, `World.despawn` removes it, and reaching
through one gets the same transform and sprite paths a script reaches on
itself — checked at compile time. Reaching through a stale or null reference is
reported rather than silently ignored.

`World.spawn` takes a typed `Prefab` — an asset reference the scene authored
into an `@export` field, not a string in the source, which is what lets the
editor resolve it and load the document before the frame that needs it — and
answers with a generation-checked reference to the new root. Overrides are the
ordinary writes through that reference; `World.set_parent` moves it, and
`World.set_property` authors a per-instance starting value that reaches the
spawned script before its first callback. A second entity can read a numeric
authored value through `World.property_number`, and a running entity receives
addressed numeric events through `World.send_signal` and `World.take_signal`.
A spawned script starts within the same pass, so a bullet fired during an
update moves during that update. Both the cascade that allows and the number of
entities one pass may create are bounded and reported rather than run.

An opaque exported `Profile` is resolved and loaded from the project in the
same pass. `Profiles.number`/`text`/`flag` read scalar values;
`Profiles.count` and the typed `*_at` calls read object records from top-level
lists, always with explicit fallbacks. Orbital Last Stand drives its 160-entry
module registry, weighted pools, requirements, display copy, and generic
effects from one profile rather than ID switchboards.

`World.set_shape_point(index, x, y)` writes one of at most eight authored
points on the current entity's `sindri.shape`; non-integer and out-of-range
indices are refused. Orbital Last Stand uses it for the exact six-point Strider
hull.

</details>

<a id="scripts-by-type"></a>

### Scripts reach each other by type

Every `script` is a type: `Bolt.on(entity)` finds one, its fields read and write live, and its functions are messages.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Scripts reach each other by type*

**Tested by:** [`crates/sindri-decay/tests/scripts_reach_each_other_by_type.rs`](../../crates/sindri-decay/tests/scripts_reach_each_other_by_type.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

<details markdown="1">
<summary>Evidence</summary>

Every `script` in a project is a type any other script can name:
`Bolt.on(entity)` finds one (or `null`), its fields read and write live — or
set what a just-spawned one starts with — and calling one of its functions
sends a message delivered after the pass, in order, in bounded rounds. A script
type is also an `Entity`. An unknown script, a misspelt field or message, or a
wrong argument is a compile error. The editor and the exporter load every
project script rather than only referenced ones, and the editor's preview and
`decay-lsp` check against the whole project. Orbital Last Stand's player sets
up each bullet it fires this way.

</details>

<a id="events"></a>

### Events (emit, handle anywhere)

A project declares events any script emits and any script handles, delivered after the pass and checked across files.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Events (emit, handle anywhere)*

**Tested by:** [`crates/sindri-decay/tests/scripts_hear_events.rs`](../../crates/sindri-decay/tests/scripts_hear_events.rs), [`decay/crates/decay-semantic/src/tests/events.rs`](../../decay/crates/decay-semantic/src/tests/events.rs), [`decay/crates/decay-runtime/src/tests/events.rs`](../../decay/crates/decay-runtime/src/tests/events.rs)

**Exercised in:** [`games/scorchball`](../../games/scorchball)

<details markdown="1">
<summary>Evidence</summary>

A project declares events (`event GoalScored(team: f32);`) that any script
emits and any script handles with `on GoalScored(team) { }`, delivered after
the pass and checked across files.

</details>

<a id="shared-state"></a>

### Shared game state, declared and typed

`state Game { var score: f32 = 0.0; }` makes a checked, typed name on the game's board.

Engine 🟡 · Editor ✅ · Decay 🟡 · Proof 🟡 — parity row *Shared game state, declared and typed*

**Tested by:** [`crates/sindri-decay/tests/scripts_share_state.rs`](../../crates/sindri-decay/tests/scripts_share_state.rs), [`decay/crates/decay-semantic/src/tests/states.rs`](../../decay/crates/decay-semantic/src/tests/states.rs)

**Exercised in:** [`games/scorchball`](../../games/scorchball), [`games/platformer`](../../games/platformer)

<details markdown="1">
<summary>Evidence</summary>

The board earns its place for facts that belong to the game rather than to an
entity — the score, whether the game is won. A project can declare what goes on
it: `state Game { var score: f32 = 0.0; }` in any file makes `Game.score` a
checked name every script reads and writes, with one type and one starting
value, stored under the same board name so `Game.get("score", 0.0)` still sees
it. A misspelt name, a wrong type and a write to a `let` are compile errors
across files.

</details>

<a id="tag-queries"></a>

### Tags and queries

`World.with_tag` and `World.has_tag` answer by authored `sindri.tags`, in deterministic order, bounded and refused past it.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Tags and queries*

**Tested by:** [`crates/sindri-decay/tests/a_script_asks_for_a_group.rs`](../../crates/sindri-decay/tests/a_script_asks_for_a_group.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

**Limits:**

- No query by more than one tag at a time, and no measured cost for a query at combat density.

<details markdown="1">
<summary>Evidence</summary>

`World.with_tag` answers with a `List<Entity>` — every active entity carrying
an authored `sindri.tags` tag, in deterministic world order, bounded at 8192
and refused rather than truncated past it. A tag says what an entity *is*,
which is the question a game that makes its enemies as it goes actually has.
The answer is a snapshot of handles, so an entity despawned mid-walk leaves one
that `World.exists` answers false for. `World.has_tag` answers the same
question for one entity without walking the world; Orbital Last Stand uses it
on collision handles so dense projectile traffic does not become one
full-world query per impact.

</details>

<a id="spatial-queries"></a>

### Deterministic gameplay nearest / radius queries

`World.nearest` and `World.within_radius` find tagged entities by world position, deterministically ordered.

Engine ✅ · Editor — · Decay ✅ · Proof ✅ — parity row *Deterministic gameplay nearest / radius queries*

**Tested by:** [`crates/sindri-decay/src/host/query/tests.rs`](../../crates/sindri-decay/src/host/query/tests.rs), [`crates/sindri-decay/tests/a_script_queries_spatial_entities.rs`](../../crates/sindri-decay/tests/a_script_queries_spatial_entities.rs), [`crates/sindri-decay/tests/spatial_demo.rs`](../../crates/sindri-decay/tests/spatial_demo.rs), [`games/orbital-baked/tests/spatial_targeting.rs`](../../games/orbital-baked/tests/spatial_targeting.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked), [`examples/spatial`](../../examples/spatial)

**Limits:**

- These are O(n) scans without an index; cone and box queries are deferred.

<details markdown="1">
<summary>Evidence</summary>

`World.nearest(tag, position)` returns the closest active tagged entity to a
world-space `Vec3`, or `null`. `World.within_radius(tag, position, radius)`
returns an inclusive-radius snapshot sorted nearest first, with stable world
order breaking ties. Both use `World::world_transform`, skip
missing/non-finite world positions, and share `with_tag`'s authored-tag and
inherited-active semantics. Radius results are refused above 8192; negative and
NaN radii are errors, zero includes coincident entities, and positive infinity
searches all spatial matches. Orbital Last Stand's player uses `nearest` when
the closest enemy is visible, with a sorted unbounded-radius fallback
otherwise; Arc selects from sorted radius results with its impact-point
exclusion.

</details>

<a id="camera-trauma"></a>

### Camera impacts are gameplay

`Camera.add_trauma` lets Decay decide when an impact happens while the engine owns the shake.

Engine ✅ · Editor — · Decay ✅ · Proof ✅

**Tested by:** [`crates/sindri-decay/tests/a_script_controls_camera.rs`](../../crates/sindri-decay/tests/a_script_controls_camera.rs)

**Exercised in:** [`examples/camera`](../../examples/camera)

**See:** [`docs/camera-runtime-scripting.md`](../../docs/camera-runtime-scripting.md)

<details markdown="1">
<summary>Evidence</summary>

Decay exposes `Camera.add_trauma(amount)`, which adds impact trauma to the one
authored world camera carrying `sindri.camera.behavior`. Decay decides when the
impact occurs; the engine-owned camera behavior remains responsible for shake
strength, waveform, frequency, decay, and the resulting transform. The camera
acceptance demo uses this call directly.

</details>

<a id="tweening"></a>

### Tweening / easing

Typed `Tween` handles for numbers, vectors and colours with delay, loops, yoyo and sequencing, sharing easing with Weave.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Tweening / easing*

**Tested by:** [`crates/sindri-decay/tests/scripts_tween.rs`](../../crates/sindri-decay/tests/scripts_tween.rs), [`crates/sindri-decay/tests/tween_demo.rs`](../../crates/sindri-decay/tests/tween_demo.rs), [`games/orbital-baked/tests/pickup_tween.rs`](../../games/orbital-baked/tests/pickup_tween.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked), [`examples/tween`](../../examples/tween)

**See:** [`docs/scripting.md`](../../docs/scripting.md)

**Limits:**

- No editor timeline, arbitrary component property binding, completion callback or CSS `@keyframes` surface.

<details markdown="1">
<summary>Evidence</summary>

`Tween.number`, `vec2`, `vec3` and `color` create typed handles, advanced once
before their owner's subsequent script updates. Typed value reads, pause,
resume, cancel, restart, progress/completion and disposal are exercised;
ownership, reclamation and the 8192 retained-handle bound are covered by the
runtime store tests. A tween composes on its handle: `Tween.set_delay`,
`Tween.set_loops` (0 for ever), `Tween.set_yoyo`, and `Tween.after` to hold one
until another has finished. Named easing lives in `sindri-core` and is also
used by Weave's CSS transitions.

Orbital's powerups appear with a short eased scale animation, pausing with the
game's menu state, and breathe with an endless yoyo that starts after their
appearance. The Pages feature example at `examples/tween` compares the five
curves on movement, scale, rotation and colour, with mouse/touch buttons and
keyboard playback controls; its real browser smoke observes movement and pause
on desktop and portrait viewports, and Tween Lab's sixth row crosses, waits and
returns as a sequence while it breathes.

</details>

## Language

<a id="vectors"></a>

### Vectors as values (`Vec2`, `Vec3`)

The language's own vector types, handed out and taken back whole by the engine, with mistakes as compile errors.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Vectors as values (`Vec2`, `Vec3`)*

**Tested by:** [`decay/crates/decay-runtime/src/tests/vectors.rs`](../../decay/crates/decay-runtime/src/tests/vectors.rs)

**Exercised in:** [`games/scorchball`](../../games/scorchball), [`games/orbital-baked`](../../games/orbital-baked)

<details markdown="1">
<summary>Evidence</summary>

Built with `Vec2(x, y)` and `Vec3(x, y, z)`, added, subtracted, scaled, negated
and compared, with `x`/`y`/`z` components that read and assign, and `length`,
`normalized` (zero stays zero rather than NaN), `dot`, `distance` and `lerp`.
The engine hands them out and takes them back whole —
`this.transform.{position,world_position,scale}`, the same through a reference,
`Pointer.position`, `Pointer.overlay` and `Stick.direction` — through the same
per-component accessors the `.x` paths use. A vector `@export` is stored as
`[x, y]` or `[x, y, z]`. Mistakes are compile errors: a `Vec2` plus a `Vec3`, a
vector times a vector (pointing at `dot`), `length()` called, a component of a
`let`. Scorchball's ball velocity, kick, aftertouch and bot aim and Orbital
Last Stand's volley spread and bullet homing use them.

</details>

<a id="timers"></a>

### Timers as values

`Timer` values in a script's fields run down before its `update`, so an early return cannot stop the clock.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Timers as values*

**Tested by:** [`crates/sindri-decay/tests/scripts_keep_time.rs`](../../crates/sindri-decay/tests/scripts_keep_time.rs), [`decay/crates/decay-runtime/src/tests/timers.rs`](../../decay/crates/decay-runtime/src/tests/timers.rs)

**Exercised in:** [`games/scorchball`](../../games/scorchball)

<details markdown="1">
<summary>Evidence</summary>

`var cooldown = Timer(0.0);`, restarted with `Timer(2.4)` and read through
`done`, `left`, `duration` and `progress`. Every timer a script's fields hold
runs down by each frame before its `update`. Scorchball's banners, power-ups,
burning, aftertouch and bot reaction times use them.

</details>

<a id="shared-functions"></a>

### Shared helper functions across files

`shared fn` belongs to the project; every script calls it by name, checked across files.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Shared helper functions across files*

**Tested by:** [`crates/sindri-decay/tests/scripts_share_functions.rs`](../../crates/sindri-decay/tests/scripts_share_functions.rs), [`decay/crates/decay-semantic/src/tests/functions.rs`](../../decay/crates/decay-semantic/src/tests/functions.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

<a id="constants"></a>

### Constants

`const` values are worked out at compile time; `shared const` makes one every file's.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Constants*

**Tested by:** [`crates/sindri-decay/tests/scripts_share_constants.rs`](../../crates/sindri-decay/tests/scripts_share_constants.rs), [`decay/crates/decay-runtime/src/tests/constants.rs`](../../decay/crates/decay-runtime/src/tests/constants.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

<a id="structs"></a>

### Structs

Value types with named fields and methods, copied on assignment and authored in scenes as JSON.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Structs*

**Tested by:** [`crates/sindri-decay/tests/scripts_use_structs.rs`](../../crates/sindri-decay/tests/scripts_use_structs.rs), [`crates/sindri-decay/tests/scripts_use_struct_methods.rs`](../../crates/sindri-decay/tests/scripts_use_struct_methods.rs), [`crates/sindri-decay/tests/scenes_author_lists_and_structs.rs`](../../crates/sindri-decay/tests/scenes_author_lists_and_structs.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

<details markdown="1">
<summary>Evidence</summary>

A `struct` in any file is a value type every script builds with named fields
(`OfferCard(card: c, name: n, blurb: b)`), reads, writes through the `var`
holding it, and keeps in lists; it is copied where it is assigned. Functions
written after its fields are its methods, asked of one value with `this` a
read-only copy. A list or struct `@export` is authored in the scene as JSON
shaped like its type and drawn by the inspector as rows per item and per field.

</details>

<a id="lists"></a>

### Lists and ranges

Lists change in place, are asked and walked; `for i in 0..n` walks a range; `List<T>` is what host queries return.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Lists and ranges*

**Tested by:** [`decay/crates/decay-runtime/src/tests/lists.rs`](../../decay/crates/decay-runtime/src/tests/lists.rs), [`decay/crates/decay-runtime/src/tests/loops.rs`](../../decay/crates/decay-runtime/src/tests/loops.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

**Limits:**

- A script's list is capped at 10,000 elements; loops are bounded by the operation budget.

<a id="maps"></a>

### Maps

Maps keyed by text, numbers, flags, variants or entities, in first-set order.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Maps*

**Tested by:** [`decay/crates/decay-runtime/src/tests/maps.rs`](../../decay/crates/decay-runtime/src/tests/maps.rs), [`decay/crates/decay-semantic/src/tests/maps.rs`](../../decay/crates/decay-semantic/src/tests/maps.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

<details markdown="1">
<summary>Evidence</summary>

Maps are written (`["a": 1.0]`, `[:]`), read by key (`m[k]`, `get`,
`contains`, `keys()`, `values()`, `length`) and changed in place where a `var`
or field holds them (`m[k] = v`, `remove`, `clear`).

</details>

<a id="decay-text"></a>

### Text: joining and methods

Text joins with `+` and has search, slicing, replacement, case and number formatting, capped at 64 KiB.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Text: joining and methods*

**Tested by:** [`crates/sindri-decay/tests/scripts_use_text.rs`](../../crates/sindri-decay/tests/scripts_use_text.rs), [`decay/crates/decay-runtime/src/tests/text.rs`](../../decay/crates/decay-runtime/src/tests/text.rs)

**Exercised in:** [`games/scorchball`](../../games/scorchball), [`games/orbital-baked`](../../games/orbital-baked)

<a id="enums"></a>

### Enums and exhaustive `match`

Enums are shared types, `match` must be exhaustive, and an enum `@export` is a dropdown in the inspector.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Enums and exhaustive `match`*

**Tested by:** [`crates/sindri-decay/tests/scripts_use_enums.rs`](../../crates/sindri-decay/tests/scripts_use_enums.rs), [`decay/crates/decay-semantic/src/tests/enums.rs`](../../decay/crates/decay-semantic/src/tests/enums.rs)

**Exercised in:** [`games/scorchball`](../../games/scorchball)

<a id="optionals"></a>

### Optional values

Nullable types such as `f32?` and `Entity?` are checked before use.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Optional values*

**Tested by:** [`decay/crates/decay-runtime/src/tests/optionals.rs`](../../decay/crates/decay-runtime/src/tests/optionals.rs), [`decay/crates/decay-semantic/src/tests/optionals.rs`](../../decay/crates/decay-semantic/src/tests/optionals.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

<a id="colours"></a>

### Colours

Colours are language values.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Colours*

**Tested by:** [`decay/crates/decay-runtime/src/tests/colors.rs`](../../decay/crates/decay-runtime/src/tests/colors.rs), [`decay/crates/decay-semantic/src/tests/colors.rs`](../../decay/crates/decay-semantic/src/tests/colors.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

<a id="language-pipeline"></a>

### Parser, analyzer, IR and runtime

A recovering parser, a checking analyzer, a symbolic IR and a runtime behind a three-method `Host` trait, all on wasm32.

Engine ✅ · Editor — · Decay ✅ · Proof ✅

**Tested by:** [`decay/crates/decay-syntax/src/tests.rs`](../../decay/crates/decay-syntax/src/tests.rs), [`decay/crates/decay-semantic/src/tests/language.rs`](../../decay/crates/decay-semantic/src/tests/language.rs), [`decay/crates/decay-ir/tests/language_reference.rs`](../../decay/crates/decay-ir/tests/language_reference.rs), [`decay/crates/decay-runtime/tests/example_script.rs`](../../decay/crates/decay-runtime/tests/example_script.rs)

**Exercised in:** [`decay/examples/player.decay`](../../decay/examples/player.decay)

**See:** [`decay/LANGUAGE.md`](../../decay/LANGUAGE.md)

**Limits:**

- The only numeric type is spelled `f32` and every value it holds is an `f64`.

<details markdown="1">
<summary>Evidence</summary>

`decay-syntax` lexes and parses a script, reporting diagnostics with a span,
line, and column; the parser recovers rather than stopping at the first error,
and survives two hundred thousand random token sequences without panicking or
hanging. `decay-semantic` resolves names through block scopes, checks the type
model, enforces `let` against `var`, and rejects duplicate members and locals.
Host globals such as `Input` enter through an `Environment` rather than being
builtins.

`decay-ir` lowers a checked program to a symbolic instruction list, with member
chains becoming paths such as `this.transform.position.x` rather than anything
the IR interprets. `decay-runtime` executes it. Everything external crosses a
three-method `Host` trait — load a path, store a path, call a path — each
taking a subject: `None` for a path rooted at something the host owns, and the
reference for one rooted at a value the script is holding. The whole workspace
compiles for `wasm32-unknown-unknown`, which its CI checks.
`decay/examples/player.decay` is executed by a test.

</details>

## Tooling

<a id="language-server"></a>

### Language server: highlight, complete, hover, definitions, diagnostics

The VS Code extension starts the real `decay-lsp` at the project root, with project-aware diagnostics, completion and hover.

Engine — · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Language server: highlight, complete, hover, definitions, diagnostics*

**Tested by:** [`tools/decay-lsp/src/lifecycle_tests.rs`](../../tools/decay-lsp/src/lifecycle_tests.rs), [`tools/decay-lsp/src/protocol.rs`](../../tools/decay-lsp/src/protocol.rs)

**Exercised in:** [`editors/vscode-decay`](../../editors/vscode-decay)

**See:** [`docs/decay-lsp-modernization.md`](../../docs/decay-lsp-modernization.md)

<details markdown="1">
<summary>Evidence</summary>

The official VS Code extension registers `.decay`, highlights the current
language, and supplies comments, pairs and indentation. It starts the real
`decay-lsp` at the nearest Sindri project root, so diagnostics, completion,
hover and document symbols use the project's script types, events and shared
state. The Sindri project browser opens scripts in VS Code with that root;
saving uses the editor's existing watched compile/reload path.
`docs/decay-lsp-modernization.md` is the active checklist for what remains.

</details>

## Not yet

<a id="formatter"></a>

### Formatter

There is no Decay formatter.

Engine ❌ · Editor ❌ · Decay ❌ · Proof — — parity row *Formatter*

<a id="debugger"></a>

### Debugger (breakpoints, stepping)

There is no Decay debugger.

Engine ❌ · Editor ❌ · Decay ❌ · Proof — — parity row *Debugger (breakpoints, stepping)*

<a id="coroutines"></a>

### Coroutines / sequencing

Scripts cannot suspend; sequencing is written with timers and state.

Engine ❌ · Editor — · Decay ❌ · Proof — — parity row *Coroutines / sequencing*

<a id="script-unit-tests"></a>

### Script unit tests

There is no way to write a test in Decay itself.

Engine ❌ · Editor ❌ · Decay ❌ · Proof ❌ — parity row *Script unit tests*

<a id="closures"></a>

### Sets, closures and first-class functions

The language has no sets, closures or first-class functions.

Engine — · Editor — · Decay ❌ · Proof —

<a id="typescript-sdk"></a>

### TypeScript embedding SDK

Browser games expose narrow application entry points and run their gameplay in Decay; there is no optional TypeScript SDK.

Engine ❌ · Editor — · Decay — · Proof —

<a id="generic-fractional-component-writes"></a>

### Fractional component writes from integer-authored numbers

Existing generic numeric component leaves round writes when the stored JSON slot is an integer, so fractional 2D light intensity or tint can be lost

Engine ❌ · Editor — · Decay ❌ · Proof ❌ — parity row *Fractional component writes from integer-authored numbers*

<details markdown="1">
<summary>Evidence</summary>

Existing generic numeric component leaves round writes when the stored JSON slot is an integer, so fractional 2D light intensity or tint can be lost. Checked 3D light/environment setters use dedicated floating-point storage; other leaves still need declared storage semantics (`sindri-decay/host/access.rs`)

</details>

<a id="quaternion-operations"></a>

### Quaternion composition and interpolation

No quaternion multiplication, axis-angle value constructor or slerp API; existing Euler and world-space aim/orbit controls remain available

Engine ❌ · Editor — · Decay ❌ · Proof ❌ — parity row *Quaternion composition and interpolation*

**See:** [`docs/scripting.md`](../../docs/scripting.md)

<details markdown="1">
<summary>Evidence</summary>

No quaternion multiplication, axis-angle value constructor or slerp API; existing Euler and world-space aim/orbit controls remain available

</details>

## 3D transform values

<a id="rotations"></a>

### Rotations as values

Copied parent-relative Quat values support normalized atomic whole writes; composition and interpolation remain absent.

Engine 🟡 · Editor — · Decay 🟡 · Proof 🟡 — parity row *Rotations as values*

**Tested by:** [`crates/sindri-decay/tests/spatial_transforms/quaternion.rs`](../../crates/sindri-decay/tests/spatial_transforms/quaternion.rs)

**Exercised in:** [`examples/transform`](../../examples/transform)

**See:** [`docs/scripting.md`](../../docs/scripting.md)

<details markdown="1">
<summary>Evidence</summary>

parent-relative `Quat` x/y/z/w value, using
Decay's existing struct constructor and copy semantics. Whole writes normalize
finite nonzero values atomically; zero/non-finite writes retain the transform.
Stored components are readable; edit a copied local value and assign it back.
Script regressions cover huge/tiny magnitudes, invalid input, parented rotation,
copy independence and runtime rejection of direct component writes. Transform
Lab saves its rotation, transfers its facing to the goal and restores its original
on arrival through Session and exported Chromium. Generated host metadata exposes
the type to the checker and LSP. This is a general transform capability added to
finish the planned 3D scripting surface; Explorer game proof remains pending.

</details>
