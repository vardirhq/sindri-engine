<!-- Generated from docs/capabilities/core.toml by `cargo run -p sindri-capabilities -- --write`. Edit the source, not this file. -->

# Engine core

The engine loop, the world and its entities, prefabs, scenes on disk, and the
command layer every edit goes through.

Each entry links its tests and the projects that exercise it. Surface marks match the row in [parity](../parity.md) where one is named: ✅ works and is exercised · 🟡 a useful slice, with the gap named · ❌ absent · — not applicable.

| Capability | Engine | Editor | Decay | Proof |
| --- | :-: | :-: | :-: | :-: |
| **Lifecycle and time** | | | | |
| [Engine lifecycle and fixed-step time](#lifecycle-and-time) | ✅ | — | — | ✅ |
| **World and entities** | | | | |
| [Entities and hierarchy](#entities-and-hierarchy) | ✅ | ✅ | ✅ | ✅ |
| [Child transforms follow their parent](#child-transforms) | ✅ | ✅ | ✅ | ✅ |
| [Prefabs](#prefabs) | ✅ | ✅ | ✅ | ✅ |
| [Nested prefabs](#prefab-instances) | ✅ | ✅ | ✅ | ✅ |
| **Scenes** | | | | |
| [Scene save / load](#scene-documents) | ✅ | ✅ | — | ✅ |
| **Commands and undo** | | | | |
| [Undo / redo](#commands-and-undo) | ✅ | ✅ | — | — |
| **Not yet** | | | | |
| [Extensible system scheduler](#system-scheduler) | ❌ | — | — | — |

## Lifecycle and time

<a id="lifecycle-and-time"></a>

### Engine lifecycle and fixed-step time

Enforced lifecycle transitions, capped frame time, bounded fixed-step catch-up and rational time scale.

Engine ✅ · Editor — · Decay — · Proof ✅

**Tested by:** [`crates/sindri-core/src/lifecycle.rs`](../../crates/sindri-core/src/lifecycle.rs), [`crates/sindri-core/src/engine.rs`](../../crates/sindri-core/src/engine.rs), [`crates/sindri-platform/src/clock.rs`](../../crates/sindri-platform/src/clock.rs), [`crates/sindri-platform/tests/game_loop.rs`](../../crates/sindri-platform/tests/game_loop.rs)

<details markdown="1">
<summary>Evidence</summary>

Legal state transitions are enforced rather than assumed: an engine can be
created, started, paused, resumed, and stopped, and an illegal transition is a
`LifecycleError` rather than a silently ignored call. Frame time is capped so a
long stall cannot produce one enormous step, and fixed-step accumulation runs
simulation at a steady rate with a bound on catch-up steps that prevents a
spiral of death. Time scale is rational rather than floating-point, so slow
motion does not accumulate drift. `ManualClock` drives the whole loop from a
test with no window and no sleeping.

</details>

## World and entities

<a id="entities-and-hierarchy"></a>

### Entities and hierarchy

Generation-checked entity handles, safe spawning and destruction, and reparenting with cycle prevention.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Entities and hierarchy*

**Tested by:** [`crates/sindri-core/src/world/tests/mod.rs`](../../crates/sindri-core/src/world/tests/mod.rs), [`crates/sindri-core/src/world/tests/hierarchy.rs`](../../crates/sindri-core/src/world/tests/hierarchy.rs)

**See:** [`docs/2d-model.md`](../../docs/2d-model.md), [`docs/entity-scaling.md`](../../docs/entity-scaling.md)

<details markdown="1">
<summary>Evidence</summary>

Entities are generation-checked slot handles, so a handle to a despawned entity
is detected rather than silently addressing whatever took its place. Spawning,
access, recursive destruction, and slot reuse are all safe. Hierarchies support
reparenting with cycle prevention, and `World::check_set_parent` answers whether
a move is legal without making it. Each entity carries a name, a parent,
children, an optional `Transform3D`, arbitrary JSON components, and an
editor-only section the runtime never interprets. There is one transform, with
2D-shaped accessors that read and write X and Y and cannot express a change to
Z, and a Z lock a transform can declare that the command layer refuses to write
past: a 2D entity is one that keeps to a plane, not one with a different
transform type — see `docs/2d-model.md`.

Entity storage was measured at 1k, 10k, and 100k entities before considering an
archetype ECS; `docs/entity-scaling.md` records why one is not warranted.

</details>

<a id="child-transforms"></a>

### Child transforms follow their parent

A child's transform is local to its parent everywhere something is placed in the world.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Child transforms follow their parent*

**Tested by:** [`crates/sindri-core/src/world/space_tests.rs`](../../crates/sindri-core/src/world/space_tests.rs), [`crates/sindri-decay/tests/a_script_reads_where_a_child_is.rs`](../../crates/sindri-decay/tests/a_script_reads_where_a_child_is.rs)

**Exercised in:** [`games/scorchball`](../../games/scorchball), [`games/low-tide`](../../games/low-tide)

**Limits:**

- Scale composes per axis, which is exact unless a rotated parent is scaled unevenly.

<details markdown="1">
<summary>Evidence</summary>

Moving, turning or growing a parent carries its children, and
`World::world_transform` is where an entity ends up with every parent folded
in. Everything that places something in the world reads that: sprites, shapes,
meshes, tilemaps, tile volumes, voxel worlds, cameras and their follow targets,
lights, grid placement, particle bursts, and 2D physics, which starts a child
body at its world pose and writes its answer back as the local transform that
puts it there. The editor draws and drags a child's handles in the world and
stores the local result, and moving an entity to a new parent in the hierarchy
keeps it where it was on screen. Scripts' `transform.position` is the stored
local value and `world_position` the world one; `World.set_parent` keeps the
local transform. Screen UI keeps its own layout, which already placed an
element inside its parent's box.

</details>

<a id="prefabs"></a>

### Prefabs

Single-root reusable entity definitions in the scene document shape, spawned whole or not at all.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Prefabs*

**Tested by:** [`crates/sindri-core/src/prefab/tests.rs`](../../crates/sindri-core/src/prefab/tests.rs), [`crates/sindri-core/src/world/tests/prefab_identity.rs`](../../crates/sindri-core/src/world/tests/prefab_identity.rs), [`games/platformer/tests/coins_are_one_prefab.rs`](../../games/platformer/tests/coins_are_one_prefab.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**See:** [`docs/prefabs.md`](../../docs/prefabs.md)

**Limits:**

- An entity spawned at runtime is not linked to its prefab, by design; only a scene's instances are.

<details markdown="1">
<summary>Evidence</summary>

A **prefab** is an authored reusable entity definition: a single-root scene
fragment in the same document shape a scene uses, sharing its entity rules,
component payloads, versioning, and canonical serialization.
`World::spawn_prefab` creates the whole subtree or none of it, and answers with
the root, every entity made, and which authored identity each became.
Instances carry no `source_id`, because a prefab's identities name entities
inside the prefab and two instances would collide on every one.

</details>

<a id="prefab-instances"></a>

### Nested prefabs

Scenes place prefab instances with per-component overrides; prefabs nest, and edits reach every instance.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Nested prefabs*

**Tested by:** [`crates/sindri-core/tests/prefab_instances.rs`](../../crates/sindri-core/tests/prefab_instances.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

**See:** [`docs/prefabs.md`](../../docs/prefabs.md)

<details markdown="1">
<summary>Evidence</summary>

A scene places a prefab as an **instance**: an entity naming the prefab, with
its own ID, parent, name and transform and nothing else but its overrides — a
JSON merge patch per component, keyed by the prefab's entity IDs. Loading
expands it into the prefab's entities (`coin-3`, `coin-3/sparkle`), each
carrying a `PrefabLink`; saving writes it back as the reference, working the
overrides out as the difference from the prefab. Prefabs nest, and an
override reaches inside by path. The editor keeps instances linked: an edit to
the prefab reaches each one as a single undoable reconciliation that keeps its
overrides, and the inspector reverts, applies and unpacks them. Hosts that play
a scene expand it once with `SceneDocument::expanded`; the exporter ships every
prefab a scene places. The platformer's ten coins are instances of
`prefabs/coin.prefab`. An instance can do without some of its prefab's
entities (`removed`), patch a list by index, and keep its inner entities'
editor state.

</details>

## Scenes

<a id="scene-documents"></a>

### Scene save / load

Versioned JSON scenes with stable authored IDs, validated on load and saved canonically byte for byte.

Engine ✅ · Editor ✅ · Decay — · Proof ✅ — parity row *Scene save / load*

**Tested by:** [`crates/sindri-core/src/scene/tests/canonical.rs`](../../crates/sindri-core/src/scene/tests/canonical.rs), [`crates/sindri-core/src/scene/tests/validation.rs`](../../crates/sindri-core/src/scene/tests/validation.rs), [`crates/sindri-core/tests/scene_round_trip.rs`](../../crates/sindri-core/tests/scene_round_trip.rs), [`crates/sindri-core/tests/baked_documents_are_canonical.rs`](../../crates/sindri-core/tests/baked_documents_are_canonical.rs), [`crates/sindri-core/src/migration/tests/chain.rs`](../../crates/sindri-core/src/migration/tests/chain.rs)

**See:** [`docs/scene-serialization.md`](../../docs/scene-serialization.md), [`docs/versioning.md`](../../docs/versioning.md)

<details markdown="1">
<summary>Evidence</summary>

Scenes are versioned JSON documents with stable authored IDs kept separate from
runtime handles, so saving a loaded scene reproduces the authored identities
rather than inventing new ones. Loading validates duplicate IDs, missing
parents, and hierarchy cycles. Unknown component payloads are preserved by
default rather than dropped, which is what makes a scene written by a newer
editor survive an older runtime.

Serialization is canonical and a fixed point: entities and keys are sorted,
empty sections omitted, short scalar arrays kept on one line. Saving an unedited
scene reproduces the file byte for byte, which is what makes saving safe to
offer at all. Golden fixtures enforce it, and legacy fixtures migrate forward
through the migration chain.

</details>

## Commands and undo

<a id="commands-and-undo"></a>

### Undo / redo

Every world edit is an invertible command; transactions are all-or-nothing and drags merge into one step.

Engine ✅ · Editor ✅ · Decay — · Proof — — parity row *Undo / redo*

**Tested by:** [`crates/sindri-core/src/command/tests/undo.rs`](../../crates/sindri-core/src/command/tests/undo.rs), [`crates/sindri-core/src/command/tests/transaction.rs`](../../crates/sindri-core/src/command/tests/transaction.rs), [`crates/sindri-core/src/command/tests/merge.rs`](../../crates/sindri-core/src/command/tests/merge.rs), [`crates/sindri-core/src/command/tests/locked.rs`](../../crates/sindri-core/src/command/tests/locked.rs)

<details markdown="1">
<summary>Evidence</summary>

Every world edit can go through a `WorldCommand` that produces its own inverse:
set name, set the transform, set parent, set or remove a component.
`Transaction` is all-or-nothing, and `CommandHistory` gives bounded undo and
redo with labelled steps and merge runs, so a continuous drag collapses into one
undoable step rather than several hundred. The history also numbers the state
the world is in, so a tool that remembers the number it last saved can tell
whether the world and the file still agree — including after undoing back to it.

</details>

## Not yet

<a id="system-scheduler"></a>

### Extensible system scheduler

The fixed session pipeline has an explicit order, but no scheduler or dependency graph for third-party systems.

Engine ❌ · Editor — · Decay — · Proof —
