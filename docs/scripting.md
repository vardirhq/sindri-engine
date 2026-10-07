# Scripting

How a Decay script reaches a Sindri world, and what it is allowed to touch.

This is the contract for `sindri-decay`. The language itself is documented in
`decay/README.md`, and why it exists at all in `docs/decay-direction.md`. AI
agents changing scripts or host APIs must also follow the concise
[Decay agent guide](decay-agent-guide.md) and run its typed preflight.

Every namespace, call, and member this build actually offers is written down,
from the surface itself, in [`docs/generated/decay-api.md`](generated/decay-api.md)
and its machine-readable twin `decay-api.json`. That is the list; this page is
what the entries mean.

## The shape of it

```text
decay/                         the language: syntax, semantics, IR, runtime
  |                            knows nothing about entities
crates/sindri-decay/           the binding: this crate, and only this crate,
  |                            knows both halves
editor/src/scripts.rs          the host: fetches sources, decides what a frame
                               is worth, restores the world when play stops
```

The dependency runs one way only. **Nothing under `decay/` may depend on a
`sindri-*` crate, and no engine crate depends on `sindri-decay`** — the editor
does, as a host. `decay` is also `exclude`d from the engine workspace in the
root `Cargo.toml`: a path dependency alone does not keep a nested workspace
separate, because cargo makes the path dependencies of a member into members
too, which silently gave the decay crates the engine's version and lints
instead of their own.

That isolation is the whole insurance policy on having written a language. It
is what makes the decision reversible, and it is worth more than any feature
that would cost it.

## `sindri.script`

```json
"sindri.script": {
  "source": "scripts/spin.decay",
  "script": "Spin",
  "properties": { "turns_per_second": 0.25 },
  "enabled": true
}
```

`source` is a logical `AssetId`, resolved against the scene's directory like any
other asset. `script` names which container in that file drives this entity, so
one file may hold a library of behaviours rather than being one behaviour.

`properties` are the authored values for the container's `@export` fields. They
are in the scene rather than the script because that is exactly the distinction:
the script says a speed exists and what it defaults to, the scene says what
*this* entity's speed is. This is also the capability that justified a typed
language over an embedded dynamic one — a property panel needs a declared,
named, typed field it can draw without executing anything.

A list or a struct `@export` is authored as JSON shaped like its type — a list
as `[...]` of its elements, a struct as `{"field": value}` — and read by the
type the script declared, so an enum inside either is written by the variant's
name and a vector as its components:

```json
"properties": {
  "card_names": ["Guidance Kernel", "Arc Imprint"],
  "loot": [{ "name": "gem", "weight": 1.5, "kind": "Gem" }],
  "tuning": { "speed": 5.0 }
}
```

The inspector draws a list with a row per item and buttons to add, remove and
move one up, and a struct with a row per field, all the way down; a new item
starts blank — zero, false, empty, the first variant. Orbital's module chooser
reads its four cards' names from a list the scene authors.

A property is **refused rather than ignored** in every failing case: a field the
script does not declare, a field that is not `@export`, or a value Decay has no
type for — and, for a list or a struct, the item or field that was wrong and
why. An authored number that silently goes nowhere is precisely the failure
this component exists to make visible.

`enabled` defaults to true. A disabled script is still authored, still
inspectable, and still saved — it simply does not tick, which is what an author
wants while narrowing down which script is misbehaving.

### Reusable profiles

An exported `Profile` names reusable project data rather than an entity in the
scene:

```rust
script Weapon {
    @export let tuning: Profile;

    fn start() {
        let damage = Profiles.number(this.tuning, "damage", 1.0);
    }
}
```

The editor draws that field as a profile asset picker, hosts load the reference
before the script runs, and export includes it in the build. `Profile` is opaque
to Decay; typed reads are made through `Profiles`:

| Call | Returns |
| --- | --- |
| `Profiles.name(profile)` | `String` |
| `Profiles.kind(profile)` | `String` |
| `Profiles.number(profile, key, fallback)` | `f32` |
| `Profiles.text(profile, key, fallback)` | `String` |
| `Profiles.flag(profile, key, fallback)` | `bool` |
| `Profiles.count(profile, collection)` | `f32` |
| `Profiles.number_at(profile, collection, index, key, fallback)` | `f32` |
| `Profiles.text_at(profile, collection, index, key, fallback)` | `String` |
| `Profiles.flag_at(profile, collection, index, key, fallback)` | `bool` |

The `*_at` calls read fields from objects in a top-level list. A missing key,
wrong value type, or out-of-range index returns the authored fallback; a missing
profile reference is a host error. Profiles are read-only during gameplay. The
document and editor contract are in [`profiles.md`](profiles.md).

## What a script can reach

Everything, in one table. Decay knows paths, not engine concepts:
`this.transform.position.x` arrives at the runtime as four strings the IR never
interprets, and `WorldHost` is the only place that gives them a meaning.

### The entity a script is on

| Path | Type | Read | Write |
| --- | --- | --- | --- |
| `this.transform.{position,world_position,scale}` | `Vec3` | yes | yes |
| `this.transform.position.{x,y,z}` | `f32` | yes | yes |
| `this.transform.world_position.{x,y,z}` | `f32` | yes | yes |
| `this.transform.scale.{x,y,z}` | `f32` | yes | yes |
| `this.transform.rotation_z` | `f32` | yes | yes |

**A vector member is reached whole or one component at a time.**
`this.transform.position` is a `Vec3` a script can hold, add to and store back
— `this.transform.position += velocity * dt` — and
`this.transform.position.x` is still the one number it always was. Both go
through the same accessors, so they cannot disagree: writing a whole vector
writes its three components, in order. Vectors themselves are the language's,
not the engine's; `decay/LANGUAGE.md` has what they can do.

**A child's transform is relative to its parent.** Moving, turning or growing a
parent carries its children, so a marker spawned under a player with
`World.spawn_child` stays over the player's head at the local offset it was
given, and a turret put on a ship turns with the ship. `position`, `scale` and
`rotation_z` are that local transform, which is also what the inspector shows
and the scene stores; for an entity with no parent they are simply where it is.
`world_position` is where it ends up once every parent has had its say. Read it
to compare two entities in different places in the hierarchy; write it to put a
child at a point in the world, which stores whatever local position puts it
there. `World.set_parent` keeps the local transform, so something spawned at
local zero and then parented sits on its new parent.

Screen UI is placed by the overlay's layout rather than this, and its
`position` is already relative to its parent's box.

| Path | Type | Read | Write |
| --- | --- | --- | --- |
| `this.sprite.{tint,color_multiply,color_offset}` | `Color` | yes | yes |
| `this.sprite.tint.{r,g,b,a}` | `f32` | yes | yes |
| `this.sprite.color_multiply.{r,g,b,a}` | `f32` | yes | yes |
| `this.sprite.color_offset.{r,g,b,a}` | `f32` | yes | yes |

**A colour is reached whole or one channel at a time**, as a vector is:
`this.sprite.tint = Color("#ff8800")` writes all four channels, `let was =
this.sprite.tint` holds them, and `this.sprite.tint.a = 0.5` is still one
number.

**`tint` multiplies; `color_offset` adds.** A tint can only scale a channel the
art already has, so a sprite drawn nearly black stays nearly black whatever
colour is asked for — `0.05 * 1.0` is still `0.05`. The offset is what reaches
past that, because it adds light the texture never carried: the whole is
`sample * tint * color_multiply + color_offset`. Both default to the identity,
so a script that never touches them changes nothing.

Two things to know before reaching for the offset. It applies to every pixel of
the sprite equally, so it lifts shadows and highlights alike and flattens
whatever shading the art has — the same trade Flash's own colour transform
makes. And `color_offset.a` raises the alpha of pixels the art left
transparent, which turns the sprite's empty corners into a visible rectangle;
leave it at zero unless that is the intent. Nothing is clamped: a channel that
lands outside zero to one is clipped by the render target, while one that is
not a number at all is refused when the scene is extracted.

| `this.sprite.layer` | `f32` | yes | yes |
| `this.ui_image.tint` | `Color` | yes | yes |
| `this.ui_image.tint.{r,g,b,a}` | `f32` | yes | yes |
| `this.ui_image.layer` | `f32` | yes | yes |
| `this.shape.{fill,stroke}` | `Color` | yes | yes |
| `this.shape.fill.{r,g,b,a}` | `f32` | yes | yes |
| `this.shape.stroke.{r,g,b,a}` | `f32` | yes | yes |
| `this.shape.count` | `f32` | yes | yes |
| `this.shape.stroke_width` | `f32` | yes | yes |
| `this.shape.sweep_start` | `f32` | yes | yes |
| `this.shape.sweep_turns` | `f32` | yes | yes |
| `this.shape.dashes` | `f32` | yes | yes |
| `this.shape.dash_duty` | `f32` | yes | yes |
| `this.shape.layer` | `f32` | yes | yes |

`sprite` is the thing in the world and `ui_image` is the thing on the viewport:
two components, so two paths. A script says which it means rather than writing
a tint that lands wherever the entity happened to be drawn — and an entity is
only ever one of the two, so on any given entity exactly one of these paths has
anything behind it.

`shape` is the world-space drawn shape, and it carries more than a tint because
there is more to drive. A sprite has one colour; a shape has a fill and a
stroke, and they animate separately — an enemy that flashes its outline without
lighting up its interior is one write. `count` changes a polygon's authored side
count at runtime, so one reusable shape can become a triangle, quad, hexagon, or
other regular polygon without swapping assets. `sweep_turns` is the reason the
group is worth having at all: a cooldown ring, a charge meter and a boss's
health arc are each a single float the script already holds, where drawing the
same thing from a sprite would need a frame of art per step. `stroke_width` is
how something pulses without changing size, and `dashes` is how a marker reads
as scanning; `dash_duty` is how much of each dash is drawn rather than gap.

An irregular polygon can carry up to eight authored 2D points. Those points are
not exposed as a mutable array path; the bounded host call writes them instead:

```rust
World.set_shape_point(0, 0.0, 1.0);
World.set_shape_point(1, 0.8, -0.6);
World.set_shape_point(2, -0.8, -0.6);
```

`World.set_shape_point(index, x, y)` writes one vertex on the current script
entity's `sindri.shape`. The index must be a whole number from 0 through 7. The
bound is part of the renderer contract: eight points fit in the fixed
per-instance payload while staying inside conservative WebGPU vertex-attribute
limits. Calling it on an entity without a world-space shape is an error rather
than a silent no-op.

Either path reaches into the entity's stored payload — `sindri.sprite` or
`sindri.ui.image` — rather than through the typed view, because a component is a
`Deserialize`-only view over a payload and the payload is what gets written
back: going through the view would mean rebuilding and reserializing it, which
is how a field the view does not know about gets dropped. A number written where
the payload held an integer is rounded back to one, so touching a layer does not
change a scene byte for byte.

An entity with no sprite is not an error at compile time — the surface says a
script *may* reach one, not that every entity has one — and a write says so
plainly at runtime.

### Other entities

A script can hold a reference to an entity, which is the thing that lets it say
anything about a world beyond itself.

`this.entity` is a script's reference to the entity it runs on. It reads and
cannot be written — a script gets to name another entity, not to reassign which
one it is running on — and through it every path below is the same path the
table above lists, reaching the same numbers.

| Path | Type | Read | Write |
| --- | --- | --- | --- |
| `this.entity.transform.{position,world_position,scale}` | `Vec3` | yes | yes |
| `this.entity.transform.position.{x,y,z}` | `f32` | yes | yes |
| `this.entity.transform.world_position.{x,y,z}` | `f32` | yes | yes |
| `this.entity.transform.scale.{x,y,z}` | `f32` | yes | yes |
| `this.entity.transform.rotation_z` | `f32` | yes | yes |
| `this.entity.sprite.{tint,color_multiply,color_offset}` | `Color` | yes | yes |
| `this.entity.sprite.tint.{r,g,b,a}` | `f32` | yes | yes |
| `this.entity.sprite.color_multiply.{r,g,b,a}` | `f32` | yes | yes |
| `this.entity.sprite.color_offset.{r,g,b,a}` | `f32` | yes | yes |
| `this.entity.sprite.layer` | `f32` | yes | yes |
| `this.entity.ui_image.tint` | `Color` | yes | yes |
| `this.entity.ui_image.tint.{r,g,b,a}` | `f32` | yes | yes |
| `this.entity.ui_image.layer` | `f32` | yes | yes |
| `this.entity.shape.{fill,stroke}` | `Color` | yes | yes |
| `this.entity.shape.fill.{r,g,b,a}` | `f32` | yes | yes |
| `this.entity.shape.stroke.{r,g,b,a}` | `f32` | yes | yes |
| `this.entity.shape.count` | `f32` | yes | yes |
| `this.entity.shape.stroke_width` | `f32` | yes | yes |
| `this.entity.shape.sweep_start` | `f32` | yes | yes |
| `this.entity.shape.sweep_turns` | `f32` | yes | yes |
| `this.entity.shape.dashes` | `f32` | yes | yes |
| `this.entity.shape.dash_duty` | `f32` | yes | yes |
| `this.entity.shape.layer` | `f32` | yes | yes |

| Call | Returns |
| --- | --- |
| `World.find(name)` | `Entity`, or `null` |

`World.find` prefers an entity that is taking part in the scene, and otherwise
answers with the first match in world order. That matters once a world holds
more than one scene: a game with a farm and a farmhouse has a `Player` in each,
and only one of them is anywhere the player is.

It *prefers* rather than filters. A switched-off screen is looked up by name
precisely so that something can switch it back on, so a lookup that skipped what
is out of play would make every such screen unreachable. `World.with_tag` does
filter, because a query answering with things out of play is a different
question from a lookup of one thing somebody already knows the name of.

| `World.exists(entity)` | `bool` |
| `World.despawn(entity)` | nothing |
| `World.spawn(prefab)` | `Entity` |
| `World.spawn_child(prefab, parent)` | `Entity` |
| `World.set_parent(entity, parent)` | nothing |
| `World.set_shape_point(index, x, y)` | nothing |
| `World.set_property(entity, name, value)` | nothing |
| `World.property_number(entity, name, fallback)` | `f32` |
| `World.send_signal(entity, name, value)` | nothing |
| `World.take_signal(name)` | `f32` |
| `World.with_tag(tag)` | `List<Entity>` |
| `World.nearest(tag, position)` | `Entity`, or `null` |
| `World.within_radius(tag, position, radius)` | `List<Entity>` |
| `World.has_tag(entity, tag)` | `bool` |
| `World.set_active(entity, on)` | nothing |
| `World.is_active(entity)` | `bool` |

Unlike the other `World.*` calls in that table, `World.set_shape_point` acts on
the current script entity and therefore takes no entity argument. That keeps the
new capability narrow instead of introducing general mutation of arbitrary
component arrays.

An `Entity` is opaque. A script can hold one in a `var` or a field, pass it,
compare it, and reach through it to the same transform and sprite paths it
reaches on itself; it cannot build one, do arithmetic on one, or read the number
inside. The engine packs a runtime handle — a slot and a generation — into that
number, and **it is never serialized**: an `@export` of an entity is not
authorable, and the inspector shows one as empty rather than as a number nobody
can act on. See `docs/FEASIBILITY.md` on why runtime handles are not scene IDs.

A script names itself with `this.entity`, so it can pass itself to something
that takes an entity — or leave itself on the board for another script to pick
up. Reaching through it is the same as reaching directly, and that redundancy is
the point: it makes the two forms one rule rather than two.

**A reference outlives what it names**, which is exactly what generation checking
is for. `World.exists` is how a script holding one across frames asks before
using it; reaching through a stale reference is an error naming the path, not a
silent no-op, because a script holding a dead handle is a bug in the script.
Reaching through `null` is likewise an error. `World.despawn(null)` is not,
because `World.despawn(World.find("gone"))` is a reasonable thing to write.

`World.find` matches on the name a scene gave an entity — what an author typed
and can see in the hierarchy — and takes the first match. Two entities with one
name is an authoring mistake for the editor to catch, not something for this to
invent a rule about. A runtime-spawned entity has no scene ID, which is the
other reason the lookup is by name.

**Despawning is not undoable**, and no write a script makes is: a script's
transform writes produce no undo entry either, and play mode restores the world
from the snapshot it took when Play was pressed. `ROADMAP.md` keeps routing this
through `WorldCommand` as an open item rather than pretending it is done.

### Making one

`World.spawn` creates the entities a prefab describes and answers with its root.
`World.spawn_child` does the same thing while attaching that root to an existing
parent before the spawned script is scheduled to start. A prefab is an authored
reusable definition — one root and everything under it, in the same document
shape a scene uses. `docs/prefabs.md` is what one is.

**It takes a `Prefab`, not text.** A `Prefab` is opaque, like an `Entity`: a
script cannot build one, and the only way to hold one is for the scene to have
authored it into an `@export` field.

```rust
script Spawner {
    @export let bullet: Prefab;
    @export let speed: f32 = 400.0;

    fn update(dt: f32) {
        let shot = World.spawn(this.bullet);
        shot.transform.position.x = this.transform.position.x;
        World.set_property(shot, "speed", this.speed);
    }
}
```

That is a deliberate restriction and it buys four things a string literal could
not: the editor draws an asset picker for the field, the reference resolves
against the project like every other asset, the document loads before the frame
that needs it, and a reference naming nothing is refused when the project is
read rather than on the frame it is spawned. A prefab named in a script's source
would be invisible to all of it.

An exported prefab field the scene never filled in is `null`, and spawning it is
an error saying so — not a spawn that quietly makes nothing.

**Overrides are ordinary writes.** The call answers with a reference, and
everything above is reachable through it: position, scale, rotation, tint,
layer. `World.set_parent` puts it under something, or at the root when given
`null`. `World.spawn_child` is the one-step form when parenthood is part of the
spawn itself; it validates the parent before creating anything, so a stale
parent cannot leave an orphan behind. Nothing here takes a bag of JSON.

**A starting value for a script is `World.set_property`.** It is the one thing
the paths above cannot do: a script's own fields are not on the surface, because
the analyzer cannot know which container another entity runs. So a per-instance
starting value is set the way the *scene* sets one — by authoring the property
the instance will be built from — and it is refused, named, in exactly the cases
the scene's own properties are: a field the script does not declare, a field
that is not `@export`, a value Decay has no type for.

It is also refused once that entity's script is running, because properties are
applied when an instance is built and a later write would land in the payload
and change nothing a script could see.

`World.property_number` reads that authored starting value back. It does not
reach into another running script's private mutable fields: it reads the
property the scene or spawner supplied, which makes an immutable per-instance
fact such as a projectile's damage available to the entity it hits. The
fallback is required and is returned when the entity has no script, no property
by that name, or a property that is not numeric.

**Runtime interaction is a signal, not a property rewrite.**
`World.send_signal(entity, name, value)` addresses a number to one live entity;
repeated sends accumulate. That entity consumes the total with
`World.take_signal(name)`, which returns zero once the value has been taken.
Signals are runtime state and never rewrite a scene or prefab. Delivery order is
the world's deterministic script order: a receiver later in the pass can react
immediately, while one that already ran reacts on the next pass.

This is deliberately numeric and addressed. A mine can send `ignite` to the gas
cloud it overlaps, and several attacks can add `hazard_damage` to one prop,
without publishing per-entity facts on the global `Game` board or pretending
that `World.set_property` mutates a running script.

Signals predate typed access. A message (`Bolt.on(hit).bounce(2.0)`) or an
event (`GoalScored.emit(1.0)`) says the same with its name and values checked
when it compiles — see [Other scripts, by type](#other-scripts-by-type) and
[Events](#events) — and new code should use those.

**A spawned script starts in the same pass.** A bullet created during an update
moves during that update rather than standing still for a frame. It cannot start
during the call that created it — building an instance runs the container's
field initializers, which is Decay code, and the world is already lent to the
call in progress — so the pass finishes and then starts what it made. A script
started that way may spawn in turn, and those rounds are bounded: a cascade that
does not settle after eight is reported rather than taking the frame with it.
`World.spawn_child` follows the same scheduling rule, with the useful guarantee
that the parent link already exists by the time that `start()` runs.

**Spawning is bounded.** One pass may create 4096 entities. Decay's operation
budget already stops a loop that never ends, but it stops it after a million
instructions — long after a spawn loop with a mistaken bound has put a hundred
thousand entities in the world and taken the editor with it. The limit is the
same protection stated in the units the mistake is made in.

Spawned entities carry no stable scene ID. A prefab's identities name entities
*inside the prefab*: two instances carrying them would collide on every one, and
a scene saved with the collision would refuse to load.

### Groups of entities

A game that spawns hundreds of enemies cannot hold a reference to each of them.
`World.with_tag` is how it asks for them all at once, and `List<Entity>` — see
`decay/LANGUAGE.md` — is what it gets back.

`World.has_tag(entity, tag)` asks the corresponding question about one active
entity without walking the world. Collision code should use it to distinguish a
projectile from the other half of a contact; querying the whole projectile group
once per collision would turn a dense combat frame into a quadratic walk.

```rust
script Sweep {
    @export let enemy: String = "enemy";

    fn update(dt: f32) {
        for enemy in World.with_tag(this.enemy) {
            enemy.transform.position.y -= 40.0 * dt;
        }
    }
}
```

**By tag, not by name.** `World.find` matches the name a scene gave *one*
entity. A game whose enemies are "Scout 41" through "Scout 300" has three
hundred authored names and still no way to say "the enemies". A tag says what
an entity *is*, and `sindri.tags` is where one is authored.

**Active only**, which is the filter every other walk of the world uses: an
entity switched off — or whose parent is — takes no part in rendering,
stepping, scripting or picking, and a query that answered with one would be the
odd one out.

**In world order**, which is deterministic and is the same order twice for the
same world. It is not an order a game should depend on for *meaning* — it is
allocation order, and reusing a freed slot changes it — but a run is
reproducible and a test is stable.

**It is a snapshot of handles.** The collection does not change when the world
does. An entity despawned while a script is part way through walking one leaves
a handle that no longer names anything, which is exactly the case `World.exists`
is for; reaching through it is an error naming the path, as it is anywhere else.
That is the safe behaviour rather than a silent skip, because a script walking a
group it is also destroying should say which it meant.

**Bounded at 8192.** A query walks the world, so its cost is the world's size
whatever the answer; the bound is on what a script then holds and walks. Past
it the call is refused rather than truncated: a query that quietly returned the
first eight thousand enemies would be a game that quietly stopped hitting some
of them.

**Asking is not free.** One call walks every entity in the world. A script that
asks once per frame is fine; a script that asks once per bullet per frame is
quadratic, and the operation budget will eventually say so in a way that names
the script rather than the pattern. Ask once and hold the answer for the frame.

### Spatial queries

`World.nearest(tag: String, position: Vec3) -> Entity` returns the closest
active entity with the authored tag, or `null` if none has a usable spatial
transform. `World.within_radius(tag: String, position: Vec3, radius: f32) -> List<Entity>`
returns a snapshot of every such entity at distance **less than or equal to**
`radius`, or an empty list when none matches.

Both compare full three-dimensional world positions from the engine's canonical
`World::world_transform`, including parent translation, rotation and scale.
Pass `this.transform.world_position` when querying from a parented entity;
`this.transform.position` is local to its parent. Entities without transforms,
or whose composed world positions are non-finite, are skipped. An entity
switched off directly or through a parent is excluded, just as for `with_tag`.
Malformed authored tags retain the existing host error rather than silently
becoming an empty group.

The position must be a `Vec3` with finite components within the engine's `f32`
coordinate range. Wrong types, arity, or invalid positions produce a host error
naming the call. Distances are squared in `f64`, without narrowing the query
position or radius. A negative radius (including negative infinity) or NaN is
an error. Zero includes coincident entities. **Positive infinity is an
unbounded radius**, written `1.0 / 0.0` in Decay; it still skips unusable
transforms and obeys the result bound.

Radius results are **nearest first**, and equal distances retain deterministic
world order. `nearest` uses the same tie rule. The order does not depend on
hash-map iteration or entity-handle numbering. Radius snapshots share the
8192-result limit of `with_tag`: more matching results are refused, never
truncated. Out-of-radius entities do not count toward this bound. `nearest`
returns one handle and can search groups larger than 8192.

```decay
let origin = this.transform.world_position;
let target = World.nearest("enemy", origin);
if target != null { print(target.transform.world_position); }
for enemy in World.within_radius("enemy", origin, 5.0) {
    print(enemy.transform.world_position);
}
```

These are straightforward world scans, with a stable sort for radius results;
there is no spatial index yet. Orbital Last Stand proves the new capability:
its player uses `nearest` when that enemy passes the existing on-screen check,
falling back to the first acceptable radius result otherwise. Arc takes the
first radius result outside its impact-point exclusion. Their unbounded-radius
searches retain their old global reach. Their enemies and projectiles
are on the same Z plane, so the new 3D ordering preserves their former XY
ordering. `nearest` alone cannot express either gameplay filter. Cone and box
queries, physics casts, overlap queries and acceleration structures remain
follow-up work.

### Flecks that are not entities

| Call | Returns |
| --- | --- |
| `Effects.burst(entity)` | `f32` — how many flecks were made |
| `Effects.burst_at(entity, x, y)` | `f32` |
| `Effects.live()` | `f32` |

```rust
script Bullet {
    fn update(dt: f32) {
        for hit in Physics.sensor_entered() {
            Effects.burst(this.entity);
            World.despawn(hit);
            World.despawn(this.entity);
        }
    }
}
```

**A fleck is not an entity.** It has no identity a script can hold, no
components, no place in the hierarchy, and nothing can collide with it. That is
the entire trade, and it was made on a measurement rather than a hunch:
`docs/effect-scaling.md` puts eight thousand flecks-as-entities at 5.25 ms a
frame — a third of a 60 Hz budget — against 0.018 ms for the same population as
plain values. Over half the entity cost was re-reading each one's component
payload, every entity, every frame.

**What a burst looks like is authored, not argued.** How many, how fast, how big,
what colour and how long are a designer's numbers, and a call that named all of
them would be one nobody could read. An entity carries `sindri.effect.burst`, and
a script fires it:

```rust
// The scene: count 24, speed 6, spread 0.6, lifetime 0.4, tint orange.
Effects.burst_at(this.explosion, enemy.transform.position.x,
                 enemy.transform.position.y);
```

`burst` throws at the entity's own position, which is read at the moment of the
call — so the usual shape, firing a burst and then despawning what fired it,
works. `burst_at` throws somewhere else, which is what an explosion where
something *used to be* needs.

**Flecks draw their own random directions from their own stream**, never the
run's. A fleck drawn from the gameplay stream would shift every number after it,
so turning an explosion up would change which enemies spawned — and a seeded run
has to mean the same run whatever it looked like.

**The pool is bounded and says when it overflows.** Past its capacity the oldest
fleck makes way for the newest, because the newest action is the one someone is
looking at. `burst` answers with how many flecks it actually made, so a game that
wants to turn itself down can see that it should.

**A host running no effects refuses these**, rather than accepting flecks nobody
will ever see.

### What a game remembers

| Call | Returns |
| --- | --- |
| `Save.number(key, fallback)` | `f32` |
| `Save.set_number(key, value)` | nothing |
| `Save.flag(key, fallback)` | `bool` |
| `Save.set_flag(key, value)` | nothing |
| `Save.has(key)` | `bool` |
| `Save.clear()` | nothing |
| `Save.is_new()` | `bool` |
| `Save.is_damaged()` | `bool` |
| `Save.is_from_newer()` | `bool` |

```rust
script Progress {
    fn start() {
        this.best = Save.number("best_wave", 0.0);
    }

    fn on_run_ended(wave: f32) {
        if wave > this.best {
            Save.set_number("best_wave", wave);
        }
    }
}
```

**A save is a flat key/value document, not a tree.** Decay holds numbers, truths
and text and nothing else, so a structure a script could not build is a structure
nothing could write. A game that wants `settings.volume` and `progress.best_wave`
writes those keys, and the file stays something a person can read and repair.

**A fallback rather than an optional**, because every caller has one — a starting
score, a default volume — and a save is mostly read on the run where nothing has
been stored yet. Reading a key that holds the wrong kind of value gives the
fallback too: a script asking the wrong question should not get a
plausible-looking answer that lets the mistake run.

**Nothing here writes to a disk.** How often someone's storage is touched is a
decision about their machine, and a script asking for a write every frame would
make that decision badly on everyone's behalf. Writes go to a store in memory;
the host puts it somewhere, on its own schedule and before it stops. Writing the
same value again is not a change, so a game that stores its volume every frame
does not keep a disk busy.

**Three ways a save can be absent, and they are not the same.**

- `is_new` — nothing has been stored. A first run.
- `is_damaged` — something was stored and could not be read. Worth telling
  someone about *before* their progress is written over, which is the whole
  reason it is separate from the first.
- `is_from_newer` — a newer build wrote it. Its values are not loaded, because a
  reader that guessed at a format it does not know would corrupt the newer save
  the moment it wrote back.

```rust
fn start() {
    if Save.is_damaged() {
        Ui.set_text(this.notice, "Your saved progress could not be read.");
    }
}
```

**A value that is not a number is refused**, because a NaN written to a save
comes back next run and poisons whatever reads it, long after the frame that
produced it has gone.

**Editor preferences are not game saves** and never mix. A save belongs to the
game — it is the player's, it ships with the build, and it round-trips
identically in a browser and on a desktop. Editor state belongs to whoever is
running the editor and never leaves their machine.

**There is no `set_text`.** The reference game's saved state is settings,
statistics, unlocks, currency and run history — numbers and truths. A string a
script can neither build nor compare would be nearly inert, so the store carries
text and the surface does not expose it until something needs it.

### Numbers a run can be replayed from

| Call | Returns |
| --- | --- |
| `Random.value()` | `f32` in `[0, 1)` |
| `Random.range(min, max)` | `f32` in `[min, max)` |
| `Random.int(min, max)` | a whole `f32`, both ends included |
| `Random.pick(group)` | `Entity` |
| `Random.seed(value)` | nothing |

```rust
script Spawner {
    fn update(dt: f32) {
        if this.due(dt) {
            let enemy = World.spawn(this.enemy);
            enemy.transform.position.x = Random.range(-8.0, 8.0);
            enemy.transform.position.y = Random.range(-4.5, 4.5);
        }
    }
}
```

**The stream is the host's, and a seed completely determines it.** The same seed
and the same sequence of calls give the same numbers in the editor, in a native
build, and in a browser — which is what makes a run replayable from a number a
player can share.

What that does *not* promise is that adding a call somewhere leaves the rest
alone. There is one stream and everything shares it, so a number drawn early
shifts every number after it. That is the honest cost of a single stream, and it
is why a run's *seed* is worth storing while a frame's numbers are not.

**`int` includes both ends**, because "a number from 1 to 6" means six outcomes
to everyone who is not writing the loop themselves. It is drawn without modulo
bias: the naive remainder makes the first few values slightly more likely, which
nobody notices on a die and which becomes a drop table that feels wrong over a
long run.

**`pick` chooses from a group** — a target, a module offer, a spawn point —
which is most of what a game wants randomness for:

```rust
let enemies = World.with_tag(this.enemy);
if enemies.length > 0.0 {
    let target = Random.pick(enemies);
}
```

That guard is required: picking from nothing is refused rather than answered
with an entity that is not there, which would only move the same problem one
call further away.

**A range that runs backwards is refused**, because a spawner that never spawns
anywhere is worth hearing about. So is `int(1.2, 1.8)`, which has no whole number
to give.

**The engine never asks the platform for entropy.** It has no way to be
genuinely random and deliberately does not pretend otherwise: a host that seeds
nothing gets a fixed stream, so a run is repeatable rather than arbitrary. A game
that wants a different run each time calls `Random.seed` with something it knows
— a counter it saved, the moment the person pressed Start. `Random.seed` restarts
the one stream every script shares, which is what a run seed is for and why it
belongs at the start of a run rather than in the middle of one.

**This is not a source of secrets.** Anyone who can see a handful of outputs can
work out the state and predict the rest, which is fine for waves and drops and
disqualifying for anything else.

### Screen elements

| Call | Returns |
| --- | --- |
| `Ui.set_text(entity, "words")` | nothing |
| `Ui.set_number(entity, value)` | nothing |
| `Ui.set_numbers(entity, first, second)` | nothing |
| `Ui.set_fill(entity, amount)` | nothing |
| `Ui.set_slider_value(entity, value)` | nothing |
| `Ui.is_hovered(entity)` | `bool` |
| `Ui.is_pressed(entity)` | `bool` |
| `Ui.is_held(entity)` | `bool` |
| `Ui.slider_value(entity)` | `f32` |
| `Ui.slider_changed(entity)` | `bool` |
| `Ui.is_checked(entity)` | `bool` |
| `Ui.set_checked(entity, checked)` | nothing |
| `Ui.input_text(entity)` | `String` |
| `Ui.set_input_text(entity, "words")` | nothing |
| `Ui.scroll_offset(entity)` | `f32` |
| `Ui.set_scroll_offset(entity, offset)` | nothing |
| `Ui.selected(entity)` | `f32` |
| `Ui.set_selected(entity, index)` | nothing |
| `Ui.is_open(entity)` | `bool` |
| `Ui.caret(entity)` | `f32` |
| `Ui.selection_start(entity)` | `f32` |
| `Ui.selection_end(entity)` | `f32` |
| `Ui.changed(entity)` | `bool` |
| `Ui.submitted(entity)` | `bool` |
| `Ui.is_focused(entity)` | `bool` |
| `Ui.position(entity)` | `Vec2` |
| `Ui.size(entity)` | `Vec2` |

**Layout.** `Ui.position` and `Ui.size` read where an element was laid out and
how big, in overlay units: 0 is the middle of the screen, one unit is half its
height, and up is positive. They read the layout rather than the transform, so
an element a stylesheet grows with `flex-grow` reports the room it was given;
an element that is not laid out — a layout container with nothing drawn — is an
error. A game fits something in the world to a gap in its UI with them: the
Input example sizes its arena to the room between its bindings panel and its
status line.

**Widgets.** A `sindri.ui.toggle` is a switch or checkbox, and toggles that
share a `group` are a radio group: choosing one unchecks the rest. A
`sindri.ui.dropdown` opens a list of `sindri.ui.option` rows authored under it
and remembers which is `selected`, counted from zero in scene order. A
`sindri.ui.text_input` is a single-line field with a caret and a selection,
and a `sindri.ui.scroll` a region whose children scroll vertically and are
clipped to it. The person changes them by clicking, tapping, typing, pasting,
the wheel or a drag. Tab and Shift+Tab move focus in the order the scene is
written; the arrow keys and a pad's d-pad move it to the nearest control that
way, scrolling a row into view. With nothing focused the arrows take focus only
for a control marked `"autofocus": true`, which also takes focus when it
appears, so a game that walks with the arrows never has its buttons focused by
walking; Space, Enter or a pad's South button presses
what has it, and Escape or East backs out. In a field, Left, Right, Home and
End move the caret, Shift selects, Backspace and Delete erase, and Ctrl or Cmd
with A, C, X and V select all, copy, cut and paste. `Ui.changed` is true for
the one step a person changed a widget's value, and `Ui.submitted` for the
step Enter was pressed in a field. A script's own `set_*` call changes the
value without reporting a change, so a script that answers `changed` never
answers itself. While a field has the keyboard, the keys it uses are held back
from `Input`, so typing a callsign does not steer the ship. A field draws what
its script writes: `Ui.caret`, `Ui.selection_start` and `Ui.selection_end` say
where to put a caret or a highlight. In a browser, a field also takes IME
composition, a paste, and a phone's on-screen keyboard.

**The scene owns the words and the script owns the numbers.** A script can
join text — `Ui.set_text(label, "Score: " + score)` works — but a HUD's words
still belong in the scene, and a template also formats: `{.2}` is something
`+` cannot do.

So a `sindri.ui.text` component authors a **template**, and a script fills its
slots:

```rust
// The scene:  text = "Score: {}"
Ui.set_number(this.score_label, 1200.0);     // Score: 1200

// The scene:  text = "{}/{}"
Ui.set_numbers(this.health_label, 45.0, 100.0);   // 45/100
```

This is the better half of the trade rather than a consolation. The words stay
in the scene file, where they can be read, reviewed, and one day translated —
not assembled inside a script where none of that is possible.

In a template, `{}` prints as few decimals as the value needs, up to three
(`1200`, `1.5`); `{.2}` prints exactly two (`1.50`), for any count up to six;
and a doubled brace is a literal one, either way round. A lone `}` is simply
itself: text is content, and refusing to draw a label over a stray brace helps
nobody.

A slot no script has filled yet reads as `0`, so a scoreboard that has not been
written to shows a score of nothing. A value that is not a number prints `NaN`,
because a HUD saying so is telling the truth about a gameplay bug. Anything that
is not a slot — `{.x}`, `{9}` — is drawn exactly as written, so a designer who
typed it wrong sees it and fixes it rather than watching it vanish.

`set_text` replaces the template itself, for swapping one authored string for
another — a warning appearing, a label changing with the mode — or for text a
script builds, such as a player's name with their slot.

**`set_fill` is what makes a bar a bar.** A `sindri.ui.image` keeps its authored
rect and draws a fraction of it, from the edge the scene names — so the empty
part of the bar is where the full one was, rather than the bar closing towards
its middle:

```rust
Ui.set_fill(this.health_bar, hp / max_hp);
```

It clips the texture with the quad, so a segmented or lettered bar stays
correct instead of being squashed. A bar filled to zero draws nothing at all.
Which edge it empties towards is authored rather than set here: that is a
decision about how the bar reads, not a per-frame gameplay value.

### Buttons, screens, and who gets the click

A `sindri.ui.button` makes an element pressable. Its rect is the entity's own
transform — the same one that already decides where it draws — so a button on an
entity that draws something is immediately pressable, and a button on a bare
entity is a hit area with no art.

```rust
script StartButton {
    fn update(dt: f32) {
        if Ui.is_pressed(this.entity) {
            World.set_property(this.menu, "showing", false);
            World.set_property(this.game, "running", true);
        }
    }
}
```

**`is_pressed` is a click, not a press:** the pointer went down on this element
and came up on it. Sliding off before letting go is how a person changes their
mind, and it keeps working here for the same reason it does everywhere else.
`is_held` is the in-between state, for a button that should look pushed while
it is. Both are answers about *this* frame, computed before any script ran.

**A disabled entity is not hit-tested**, and neither is anything under a
disabled parent — `World.is_active` already governs a subtree. That is also all
a *screen* is: a menu, a pause overlay and a HUD are entities with children, and
showing one is `World.set_active(this.menu, false)`. There is no screen stack,
because the engine already had the mechanism and a second one would be a second
answer to the same question.

The switch is written on the entity named and not down through its children,
which is what makes switching a screen back on restore the entries that were
off on their own account — a locked menu row stays locked. `is_active` answers
the same question the engine asks before drawing or stepping anything: false
for an entity switched off, for anything under one, and for a handle the world
no longer holds.

**When elements overlap, the top one takes the click** — highest layer first,
which is the order the overlay draws in. A modal is a modal because it is on a
higher layer, not because it declared itself one.

**Nothing is silently taken away from gameplay.** An engine that withheld input
from "gameplay scripts" while a menu was up would have to know which scripts
those are, and a rule that guesses will guess wrong. So a gameplay script asks:

```rust
fn update(dt: f32) {
    if Pointer.just_pressed("Left") && !Pointer.over_ui {
        this.fire();
    }
}
```

That one line is why a click on a pause button does not also fire the gun
behind it.

### Rows and columns

A `sindri.ui.layout` on a parent places its children along an axis — `row` or
`column`, with a spacing in overlay units. Three buttons could be authored as
three offsets; what cannot be authored is what happens when one is switched off.
A hand-placed row leaves a hole, and everything below a hidden entry is in the
wrong place. A layout counts only the children that are actually there, so a
menu that loses an entry **closes up around its own middle**.

There is no scroll region. Nothing being built against this engine has a list
longer than a screen, and a scroll invented before something needs one would
have to decide about clipping, momentum, and where a drag stops being a press —
none of which has an answer yet.

### Screens that fit any screen, and text that fits with them

A font size is in those units too — two is the whole height of the screen — so
text scales with everything around it. It used to be pixels, which made it the
one number on a screen element that did not follow the screen: a HUD authored
on a desktop was unreadable on a phone, and a heading authored in the units the
rest of the element uses drew nothing at all, because an eighth of a pixel is a
positive number.

The overlay is authored in normalized units: two tall, centred on the origin,
running out to the aspect ratio either side. A corner-anchored element is in
that corner on a portrait phone and a wide desktop window alike, which is
responsive layout without a single breakpoint.

**How wide "either side" is depends on the screen**, and that is the part worth
designing for. A portrait phone gives about nine tenths of a unit across; a
desktop window gives three and a half. An element that fits the narrow one fits
both, which is why a layout is worth authoring against a phone first — and why
three cards side by side is a landscape idea that does not survive being turned
upright.

A **world** camera has the same question, and `sindri.camera` answers it with
`fit`. An orthographic camera framing `vertical_size` by `height` shows a fixed
amount vertically and whatever the width happens to be, so turning a wide window
tall takes the sides off the world. `"fit": "shorter"` makes the size a promise
instead — *this much world is visible whichever way the screen is turned* — so
an arena fills the height of a landscape window and the width of a portrait one
and is never cut off by either.

The one thing a designer cannot author around is the **safe area** — a notch, a
rounded corner, a home indicator — because it is not in the scene, it is in the
hardware the scene happens to be running on. A host reports it in pixels and
anchored elements move in from their own edge; a centred element stays centred,
because the screen does not shrink, the edges come in.

**Accessible labels are authored but not yet surfaced.** A button carries a
`label`, stored beside the thing it names in the file a designer edits, but no
backend reads it yet. Static web export exists, but its canvas/WebGPU
presentation has no semantic DOM bridge; native presentation likewise needs an
accessibility adapter.

**An entity that is not the kind of element the call needs is named**, rather
than the call quietly doing nothing — a HUD that stops updating because a script
points at the wrong element is the failure that survives a play-test. A host
laying out no screen UI refuses `Ui.is_pressed` for the same reason: a menu whose
buttons never respond should be heard about on the first frame, not mistaken for
a person who has not clicked yet.

### Independent 3D physics

| Call | Returns |
| --- | --- |
| `Physics3d.layer(name: String)` | numeric mask bit |
| `Physics3d.mask(names: List<String>)` | numeric mask |
| `Physics3d.raycast(origin: Vec3, direction: Vec3, max_distance, mask, include_sensors, exclude: Entity?)` | `RayHit3d?` |
| `Physics3d.overlap_sphere(centre: Vec3, radius, mask, include_sensors, exclude: Entity?)` | `List<Entity>` |
| `Physics3d.cast_sphere(origin: Vec3, radius, direction: Vec3, max_distance, mask, include_sensors, exclude: Entity?)` | `RayHit3d?` |
| `Physics3d.overlap_box(centre: Vec3, half_extents: Vec3, rotation_axis: Vec3, rotation_angle, mask, include_sensors, exclude: Entity?)` | `List<Entity>` |
| `Physics3d.cast_box(origin: Vec3, half_extents: Vec3, rotation_axis: Vec3, rotation_angle, direction: Vec3, max_distance, mask, include_sensors, exclude: Entity?)` | `RayHit3d?` |
| `Physics3d.overlap_capsule(centre: Vec3, half_height, radius, rotation_axis: Vec3, rotation_angle, mask, include_sensors, exclude: Entity?)` | `List<Entity>` |
| `Physics3d.cast_capsule(origin: Vec3, half_height, radius, rotation_axis: Vec3, rotation_angle, direction: Vec3, max_distance, mask, include_sensors, exclude: Entity?)` | `RayHit3d?` |
| `Physics3d.velocity(entity)` | `Vec3` |
| `Physics3d.set_velocity(entity, velocity: Vec3)` | nothing |
| `Physics3d.angular_velocity(entity)` | `Vec3` |
| `Physics3d.set_angular_velocity(entity, velocity: Vec3)` | nothing |
| `Physics3d.apply_impulse(entity, impulse: Vec3)` | nothing |
| `Physics3d.collision_started()` | `List<Entity>` |
| `Physics3d.collision_stopped()` | `List<Entity>` |
| `Physics3d.sensor_entered()` | `List<Entity>` |
| `Physics3d.sensor_exited()` | `List<Entity>` |

`layer(name)` selects a named bit from the active authored
`sindri.physics3d.world`; `mask(names)` combines those bits with OR. The first
32 labels map to bits 0–31; duplicate labels select the first bit and empty
labels cannot be selected. Repeated requested names are harmless and an empty
list returns zero. Lookup reads current authored names without stepping physics,
ignores inactive settings and never uses 2D world names. Missing 3D host, unknown
names, wrong argument types, malformed settings or multiple active 3D worlds
fail explicitly. With no active settings there are no names to select.

`Physics3d` takes Vec3 values without changing the existing 2D `Physics` API.
`velocity(entity)` and `angular_velocity(entity)` return copied Vec3 values;
`set_velocity(entity, velocity)`, `set_angular_velocity(entity, velocity)` and
`apply_impulse(entity, impulse)` control active 3D bodies. Velocity
controls accept dynamic/velocity-kinematic bodies; impulses require dynamic
bodies. Rotation-locked bodies retain zero angular velocity. Finite f32-range
vectors validate before mutation; controls leave authored motion unchanged.
Missing context and inactive/stale handles fail explicitly. Before synchronization,
valid authored 3D body and nonempty collider components are required, without
conflicting 2D physics or a moving depth lock. Setters/impulses queue in call order
and replay before solving against actual compound mass. Reads copy the last
queued setter or authored start; locked angular velocity is zero and impulses
resolve only at materialization. Successful synchronization expires unconsumed
requests; authored motion remains unchanged.

`collision_started()`, `collision_stopped()`, `sensor_entered()` and
`sensor_exited()` return copied sorted unique `List<Entity>` values naming the
other active entities from this script's last successful 3D step. All scripts
see the same non-draining snapshot; inactive/despawned references are filtered.
Shared game sessions and editor Play provide the context. See
[the physics contract](physics.md#decay) for timing and current limitations.

3D rays and sphere casts return null on misses or a copied `RayHit3d` with
`entity`, Vec3 `point`/`normal` and numeric `distance`. Sphere overlaps return
copied sorted unique entity lists. Query vectors/scalars must be finite and fit
engine f32 values; radii are positive, travel non-negative and directions nonzero
(normalized by the engine). Masks are whole u32 values, sensors opt in and
`exclude` skips every piece of one entity or is null. Queries use indexed
synchronized geometry without stepping physics and skip inactive/despawned
entities even before the next synchronization. Inside/on or initial-overlap hits
have zero distance/normal; sphere initial overlaps use the probe origin as point.
Exact hit ties prefer entity handle then piece order.

Box and capsule overlaps/casts share these filters, active-entity checks and
copied results. Box half-extents are positive Vec3 distances from centre to faces.
A capsule lies along local Y before rotation; `half_height` is half its straight
segment length (non-negative, zero makes a sphere) and `radius` is positive.
`rotation_axis` is a finite nonzero Vec3, normalized by the host;
`rotation_angle` is finite radians about it using the right-hand rule.
Identity orientation uses any nonzero axis and angle zero. Cast orientation stays
fixed during travel. Inputs must fit engine f32 range. Quaternion conversion
normalizes the axis using f64 to avoid length overflow/underflow; no quaternion
assembly is required in scripts. Query positions and rotation axes are world-space; dimensions use world units
independently of visual transform scale.

### Bodies, and what they touched

| Call | Returns |
| --- | --- |
| `Physics.contacts(entity)` | `List<Contact2d>` |
| `Physics.move_character(entity, displacement: Vec2, snap: bool)` | nothing |
| `Physics.character_motion(entity)` | `CharacterMotion2d?` |
| `Physics.apply_force(entity, force: Vec2)` | nothing |
| `Physics.apply_torque(entity, torque)` | nothing |
| `Physics.angular_velocity(entity)` | `f32` |
| `Physics.set_angular_velocity(entity, velocity)` | nothing |
| `Physics.apply_angular_impulse(entity, impulse)` | nothing |
| `Physics.apply_impulse_at_point(entity, impulse: Vec2, point: Vec2)` | nothing |
| `Physics.drop_through(entity, seconds)` | nothing |
| `Physics.continuous_collision(entity)` | `bool` |
| `Physics.set_continuous_collision(entity, enabled)` | nothing |
| `Physics.velocity_x(entity)` | `f32` |
| `Physics.velocity_y(entity)` | `f32` |
| `Physics.set_velocity(entity, x, y)` | nothing |
| `Physics.apply_impulse(entity, x, y)` | nothing |
| `Physics.connect_distance(first, second, max_distance)` | nothing |
| `Physics.set_hinge_motor(joint, velocity, max_torque)` | nothing |
| `Physics.set_hinge_position_motor(joint, target_angle, stiffness, damping, max_torque)` | nothing |
| `Physics.set_slider_motor(joint, velocity, max_force)` | nothing |
| `Physics.set_slider_position_motor(joint, target_distance, stiffness, damping, max_force)` | nothing |
| `Physics.set_spring(joint, rest_length, stiffness, damping)` | nothing |
| `Physics.joint_enabled(joint)` | `bool` |
| `Physics.set_joint_enabled(joint, enabled)` | nothing |
| `Physics.set_distance(joint, max_distance)` | nothing |
| `Physics.set_joint_endpoints(joint, first, second)` | nothing |
| `Physics.remove_joint(joint)` | nothing |
| `Physics.create_distance_joint(joint, first, second, max_distance)` | nothing |
| `Physics.create_hinge_joint(joint, first, second, first_anchor, second_anchor)` | nothing |
| `Physics.create_spring_joint(joint, first, second, first_anchor, second_anchor, rest_length, stiffness, damping)` | nothing |
| `Physics.create_slider_joint(joint, first, second, first_anchor, second_anchor, first_axis, second_axis, limits_enabled, lower_distance, upper_distance)` | nothing |
| `Physics.raycast(origin, direction, max_distance, mask, include_sensors, exclude)` | `RayHit2d` or `null` |
| `Physics.cast_circle(origin, radius, direction, max_distance, mask, include_sensors, exclude)` | `RayHit2d` or `null` |
| `Physics.cast_box(origin, half_size, rotation, direction, max_distance, mask, include_sensors, exclude)` | `RayHit2d` or `null` |
| `Physics.overlap_circle(center, radius, mask, include_sensors, exclude)` | `[Entity]` |
| `Physics.overlap_box(center, half_size, rotation, mask, include_sensors, exclude)` | `[Entity]` |
| `Physics.layer(name)` | `f32` |
| `Physics.mask(names)` | `f32` |
| `Physics.collision_started()` | `List<Entity>` |
| `Physics.collision_stopped()` | `List<Entity>` |
| `Physics.sensor_entered()` | `List<Entity>` |
| `Physics.sensor_exited()` | `List<Entity>` |

`Physics.contacts(entity)` copies solid solver contacts from the last fixed
step. A `Contact2d` contains `entity` (the other body), world `point` and
`normal` as Vec2, `normal_impulse`, signed `tangent_impulse`, and world `force`
as Vec2. The normal points towards the queried body; the tangent is
`Vec2(-normal.y, normal.x)`. Force is total impulse divided by fixed-step seconds.
Points are ordered by entity, point, normal and impulse, including multiple
points/pieces on one entity. Sensors are excluded. Sleeping contacts keep their
support geometry but report zero new impulses/force. Copies can be edited without
changing physics. Before the first step or spawn synchronization the list is
empty; absent physics and active entities without bodies fail. Inactive and
despawned others are filtered immediately. Runtime teleports/removal invalidate
contacts; scene transform writes reach physics at the next synchronization.
Platformer uses this for support grounding and the crate's hard-landing flash.

Forces and torques add together for the next fixed step, then clear. Impulses
act immediately and do not scale with dt. All points/vectors are world-space;
angles are radians and positive angular motion is counterclockwise. Dynamic
bodies accept forces/impulses; angular setters also accept velocity-kinematic
bodies. Rotation locks suppress turning. Nonfinite values, missing
bodies and absent physics hosts report errors. Spawn-window requests replay in
call order after collider mass is known; the angular getter before that returns
the latest queued setter or authored initial value. Removal/abandonment discards
pending work. A host with its own driver can control live bodies without scene
body components; only spawn-window requests require an authored body.
Platformer's wind crate is the gameplay proof.

`Physics.drop_through(entity, seconds) -> unit` ignores only one-way solid
platforms for an authored dynamic body or Character 2D. Characters queue the
scene timer, covering all movement-query phases at the next fixed pass; dynamic
bodies retain their solver timer. The duration is finite and nonnegative;
zero cancels, repeated requests replace the timer. Time advances only on fixed
physics steps and starts at materialization for a newly spawned body. Ordinary
floors and sensors stay active. Missing authored movement owners, wrong body
kinds and hosts without the corresponding physics/controller context report
errors. Geometric queries still see one-way pieces
while dropping; gameplay must decide when a query grants jump permission.

`Physics.continuous_collision(entity) -> bool` reads the live 2D body setting,
or its authored setting before a newly spawned body is synchronized.
`Physics.set_continuous_collision(entity, enabled) -> unit` requires an authored
dynamic body and updates its live state and runtime payload without replacing
it. Velocity, joints and contacts survive CCD-only edits. Hosts without physics
and missing authored bodies report errors. The platformer wind crate uses this control.
Sensors stay discrete; swept bullet-versus-bullet collision is not guaranteed.

This is **Sindri physics, never Rapier**. `docs/physics.md` makes the backend a
private implementation detail, and a namespace that leaked its vocabulary would
make the backend unreplaceable one script at a time.

A body is authored, not created here: an entity carries `sindri.physics2d.collider`
and optionally `sindri.physics2d.rigid_body`, and `ScenePhysics2d` keeps the
simulation in step with what the scene says. A prefab carrying those components
spawns with them, which is how a bullet gets a body.

`Physics.joint_enabled(joint)` reads an authored constraint's enabled flag,
including before synchronization. It does not report whether the backend has
connected valid active endpoints. `Physics.set_joint_enabled(joint, enabled)`
releases or reconnects the constraint at the next fixed synchronization, keeping
its owner, endpoints and settings. Bodies retain their motion; reconnection may
change that motion through the ordinary solver. `Physics.set_distance(joint,
max_distance)` tunes an authored maximum-distance constraint, including while
suspended or before endpoints are built. The length must be finite and positive.
These calls require physics and exactly one valid authored joint; distance tuning
requires the distance kind. They reject invalid calls before mutation and preserve
unknown payload fields. Omitting `enabled` in old scenes means true.

`Physics.create_distance_joint(joint, first, second, max_distance)` authors an
enabled maximum-distance component on an existing owner with no authored 2D joint
of any kind. Endpoint handles become scoped stable IDs or local prefab paths;
null leaves an endpoint unbound and inactive targets suspend until active. Physics
and a finite positive length are required. Creation validates before mutation,
including owner conflicts, stale/unstable/out-of-scope and identical endpoints.
It works before bodies exist; the next fixed synchronization creates the owned
constraint without replacing bodies or legacy connections. Other owner components
are retained. Platformer C repairs its cut cord at the selected hook and length.

`Physics.create_hinge_joint(joint, first, second, first_anchor, second_anchor)`
uses the same owner, physics and scoped endpoint contract. Its finite `Vec2`
anchors are body-local world units without transform scale. The enabled hinge
starts with angular limits and motor disabled; `set_hinge_motor` drives it after
creation, even before initial body synchronization. Invalid anchors fail before
mutation. Platformer H rebuilds the placed windmill hinge and restarts its motor,
without affecting the separately spawned windmill.

`Physics.create_spring_joint(joint, first, second, first_anchor, second_anchor,
rest_length, stiffness, damping)` uses the same owner/endpoint/physics contract.
Finite `Vec2` anchors use body-local world units without transform scale. Rest
length must be finite and positive; stiffness and damping finite and non-negative,
including zero. Validation precedes mutation; next fixed synchronization creates
the enabled spring, preserving other components, body motion and legacy constraints.
`set_spring` can retune it before synchronization. Platformer B rebuilds the light's
spring while retaining its current rest-length phase and subsequent tuning.

`Physics.create_slider_joint(joint, first, second, first_anchor, second_anchor,
first_axis, second_axis, limits_enabled, lower_distance, upper_distance)` shares
the owner/endpoint/physics contract. Anchors are finite body-local `Vec2` world
units without transform scale; axes must be finite unit `Vec2` vectors. Bounds
must be finite even when disabled, and enabled lower distance cannot exceed upper
distance. The new slider is enabled with its motor disabled; `set_slider_motor`
can configure drive before synchronization. Invalid settings fail before mutation.
Platformer J rebuilds its trolley slider with bounded travel, restoring the current
motor direction while retaining the separately owned lantern spring.

`Physics.remove_joint(joint)` removes exactly one valid authored distance, hinge,
slider or spring component, releasing its solver constraint at the next fixed
synchronization. It keeps the owner, its other components and endpoint bodies,
and leaves legacy `connect_distance` constraints alone. It works while suspended
or before bodies exist. Missing, conflicting or malformed joints and hosts without
physics fail before mutation. Later joint controls fail until a joint is authored
again. Use `set_joint_enabled` for reversible suspension.

`Physics.set_joint_endpoints(joint, first, second)` retargets any one valid
2D authored joint using entity handles. Null clears either endpoint. It stores
canonical local prefab paths or stable scene IDs, never runtime handles. Both
references validate before either changes: stale, unstable, out-of-scope or
identical endpoints fail atomically. Inactive endpoints are valid references but
suspend the constraint until active. Settings, enabled state, unknown fields and
body motion remain intact; the next fixed synchronization reconnects available
bodies, including requests made before initial body synchronization. Physics is
required. A name lookup that returns null explicitly clears that endpoint.

`Physics.set_hinge_motor(joint, velocity, max_torque)` takes a hinge-owner entity,
a relative angular speed in radians/second and a finite non-negative torque cap.
It explicitly selects velocity mode, including after position drive.
Positive speed turns its second endpoint counterclockwise relative to the first.
Zero torque disables the drive (coasting); zero speed with positive torque brakes.
The call validates and updates the runtime hinge component, preserving unknown
fields, and physics applies it at the next fixed synchronization. This also
works before newly spawned endpoints are built, and survives their rebuilds.
It needs a physics host and an authored `sindri.physics2d.hinge_joint` component;
invalid requests leave the payload unchanged. Full prefab endpoint references
remain incomplete; see [the physics contract](physics.md).
Platformer's windmill uses this typed control to reverse its powered axle.

`Physics.set_hinge_position_motor(joint, target_angle, stiffness, damping, max_torque)`
selects a damped force-based position drive on the authored hinge owner. The
relative target is finite radians within `[-pi, pi]`; gains and torque cap are
finite and non-negative. Enabled angular limits still bound motion. Zero torque
coasts. Validation is atomic, unknown fields and body motion are preserved, and
settings apply at next synchronization, including before bodies exist and after
suspension/rebuild. Platformer P holds both windmills at 0.6 radians or resumes
velocity reversal. H recreation restores the selected drive through a typed
`Windmill` message. Slider position drive follows the same mode-switching contract below.

`Physics.set_slider_position_motor(joint, target_distance, stiffness, damping, max_force)`
selects a damped force-based position motor on an authored slider owner. The
finite target is signed local-anchor separation along the first body's local
axis in unscaled world units; gains and force cap are finite and non-negative.
Enabled travel limits still bound motion, including targets outside their range.
Zero force coasts. Validation is atomic, unknown fields and body motion remain,
and the next synchronization applies settings even before bodies exist or after
suspension/rebuild. The velocity setter switches back explicitly. Platformer O
parks/releases its trolley and J retains its selected drive through recreation.

`Physics.set_slider_motor(joint, velocity, max_force)` tunes an authored slider
owner's relative translation speed in world units/second along the first local
axis, explicitly selecting velocity mode, with a finite non-negative force cap. Zero force coasts; zero speed with
positive force brakes. `Physics.set_spring(joint, rest_length, stiffness, damping)`
tunes a spring owner with positive rest length and non-negative stiffness/damping.
All values must be finite. Both validate before modifying runtime component
fields, preserve unknown fields, apply at the next fixed synchronization and
survive endpoint rebuilds, including calls before the bodies are built. Wrong
components or missing physics fail; invalid values leave the payload unchanged.
Platformer's lantern trolley reverses and retunes its suspended light with these
calls. Joint creation/removal and full prefab references remain incomplete.

`Physics.connect_distance` creates a maximum-distance connection between two
authored 2D bodies. They may move closer and rotate freely, but their centres
cannot separate beyond `max_distance`. Calls made while freshly spawned bodies
are waiting for synchronization are resolved before the next physics step.

```rust
script Bullet {
    @export let speed: f32 = 400.0;

    fn start() {
        Physics.set_velocity(this.entity, this.speed, 0.0);
    }

    fn update(dt: f32) {
        for hit in Physics.sensor_entered() {
            World.despawn(hit);
            World.despawn(this.entity);
        }
    }
}
```

**The event calls take nothing and answer about the entity the script is on.**
An event is about a *pair*, and the pair a script cares about is the one it is
half of — so the answer names the other half. A manager script that wanted every
collision in the world would be asking a different question, and the surface does
not offer it yet.

They are **queries rather than callbacks**, because Decay now has a value that
can hold several entities and a lifecycle function would be a second way for the
host to enter a script. The order is the order the step reported, which is the
same order twice for the same simulation.

**Started, not touching.** `collision_started` answers with what began touching
during the last step, so a projectile that should hit each target once hits each
target once without keeping a list. That is the shape the reference game's
piercing bullets need, and it falls out of the event rather than being a feature.

**Despawning from an event is safe.** Removing either half — the thing that was
hit, or the thing that hit it — is an ordinary `World.despawn`, and the body
leaves the simulation before the next step. Walking the answer while despawning
from it is the case `World.exists` covers, exactly as for any other collection of
references.

**A host with no physics refuses these**, rather than reporting a velocity of
zero for a body that does not exist. A game whose bullets never move because
nothing is stepping should hear about it on the first frame.

`Physics.raycast` casts a finite world-space segment through the simulated 2D
collider pieces, including merged tilemap boxes. `origin` and `direction` are
`Vec2`; direction must be nonzero and is normalized, so `max_distance` and the
hit's `distance` are world units. Distance must be non-negative, all geometry
arguments finite, and `mask` a whole number from 0 to 4294967295. Mask 0 misses;
4294967295 selects every collider membership. This is a query mask, independent
of a collider's physical interaction filter. `include_sensors` explicitly opts
into trigger pieces. `exclude` skips all pieces of an entity, or is `null`.

A hit is a copied `RayHit2d` struct with `entity: Entity`, `point: Vec2`,
`normal: Vec2` and `distance: f32`. A miss is `null`; check before reading it.
The maximum distance is inclusive. Starting inside/on a collider gives distance
0, point equal to origin and normal (0, 0), because there is no entry surface.
Other normals describe the world-space surface. Exact distance ties prefer the
smaller generation-checked entity handle, then authored piece order.

```decay
let p = this.transform.world_position;
let hit = Physics.raycast(Vec2(p.x, p.y), Vec2(0.0, -1.0), 12.0, 4294967295.0, false, this.entity);
if hit != null { print("Ground distance: " + hit.distance); }
```

Queries see current physics body poses. A scene/script transform write, a new
collider or a spawn is reflected at the next fixed-step synchronization;
inactive/despawned entities are skipped immediately. A held result stays a
snapshot after a later step or despawn; check `World.exists(hit.entity)` before
acting on that entity. Queries currently scan collider pieces, with no separate
spatial index. The platformer displays clearance below its hero, preserving its
foot-sensor jump rules; Physics Playground makes masks, sensors, inside hits,
misses, hit points and normals visible.

Scene-owned 2D characters use `Physics.move_character(entity, displacement, snap)`
to replace their queued world-space displacement for the next fixed step and
`Physics.character_motion(entity)` to read a copied optional `CharacterMotion2d`
from the previous completed pass. Check for null before reading support,
translation, collision or carry fields. The runtime/editor offer a controller
context separately from ordinary physics, preserving independent physics hosts.
`Physics.drop_through` selects the controller timer for authored characters and
the existing solver timer for dynamic bodies. Gameplay owns speed, gravity,
jumps and recovery; the engine supplies collision movement. See
[the character contract](character-movement.md#typed-decay-requests-and-motion-snapshots)
for validation, copied fields, filtering, spawn-window behavior and solver timing.
The platformer has not yet adopted the character API.

`Physics.overlap_circle` and `Physics.overlap_box` are area checks: every
entity with a piece overlapping a circle, or a box `half_size` from its centre
to each edge and turned by `rotation` radians, as an array holding each entity
once, in handle order. `Physics.cast_circle` and `Physics.cast_box` sweep the
same shapes from `origin` along `direction`, without turning, and return the
first piece touched as a `RayHit2d`: `point` is where the two touch, `normal`
the touched surface's, and `distance` how far the shape's centre travelled.
Starting already overlapping gives distance 0 and normal (0, 0). The mask,
sensor and exclude arguments, the validation and the synchronization rules are
the raycast's. A radius or half size must be positive. Orbital Last Stand's
hostile mine damages what its blast circle overlaps, so a target whose edge the
blast reaches is hit; Physics Playground shows a swept circle stopping short of
the ray and an area naming what it holds.

```decay
let centre = Vec2(this.transform.position.x, this.transform.position.y);
for target in Physics.overlap_circle(centre, 1.75, 4294967295.0, true, this.entity) {
    World.send_signal(target, "hazard_damage", 1.0);
}
```

A scene's `sindri.physics2d.world` names its collision layers in `layers`,
bit by bit: `["ground", "hero", "pickups"]` makes `ground` bit 0 (mask 1),
`hero` bit 1 (mask 2) and `pickups` bit 2 (mask 4), up to 32. `Physics.layer`
is the mask for one name and `Physics.mask` for several at once, for any
query's `mask` argument; a name the world does not give is an error naming the
ones it does, so a misspelt layer is heard about rather than masking nothing.
Colliders still store their memberships and filters as masks, which is what
physics reads, and the editor's inspector shows a mask as the layers it holds
by name. The platformer names its layers, and its hero's ground probe asks for
`Physics.layer("ground")`.

**Contact detail (points, normals and impulses of a collision) remains absent.**

### Grid position

| Call | Returns |
| --- | --- |
| `Grid.position_x(entity, grid)` | `f32` |
| `Grid.position_y(entity, grid)` | `f32` |
| `Grid.place(entity, grid, x, y)` | nothing |
| `Grid.can_reach(mover, grid, target)` | `bool` |
| `Grid.step_toward(mover, grid, target)` | `bool` |

The `grid` argument is an entity carrying either `sindri.tilemap` or
`sindri.tile_grid`. The two describe the same geometry — columns, rows, cell
size, projection — under different field names, and place a cell's origin and
run their Y axis identically, so a cell does not move when the component
describing it changes. A script therefore never says which of the two its floor
is, and a scene can move onto a stackable tile volume without its scripts moving
with it. When an entity carries both, which is what a migration in progress
looks like, the flat map answers.

### Where the game is

| Call | Returns |
| --- | --- |
| `Scene.current()` | `String` |
| `Scene.go(name)` | nothing |

`Scene.go` records an intention; it does not move anything. The script asking is
running *in* the scene being left, from a world the change would rearrange
underneath it, so the host performs the move between frames — the same shape as
`Audio.play` recording a sound for whoever owns a speaker.

`Scene.current()` therefore answers the scene being played, never the one asked
for. A script that read back its own request would see the move happen a frame
before it did.

The first request in a frame is the one that counts. Two scripts asking is a
conflict with no right answer, and taking the last would make a door beside a
door depend on which script the pass reached first.

A host that plays exactly one scene offers neither call, and `Scene.go` says so
rather than accepting a request nothing will perform — a game whose doors
silently never open should be heard about on the first frame.

What a scene contains, where its file is, and when it loads are the host's
business. This namespace knows a name.

### Grid cells

Where the calls above are about an entity's position on a map, these are about
what the map *holds* there.

| Call | Returns |
| --- | --- |
| `Grid.tile(map, column, row)` | `f32` |
| `Grid.set_tile(map, column, row, index)` | nothing |
| `Grid.columns(map)` | `f32` |
| `Grid.rows(map)` | `f32` |

A cell is named by whole column and row rather than by a world position: the map
is the authority on which cell a position falls in, and a script doing that
arithmetic itself would be a second answer free to disagree with the first.

`columns` and `rows` are geometry, so they answer on a `sindri.tile_grid` the
same way they answer on a `sindri.tilemap`. `tile` and `set_tile` are not: a
palette index into a flat array is the flat map's own model, and a volume stacks
named cells instead. Asked of a grid that carries only `sindri.tile_grid`, both
fail and name the component they would need rather than reporting that a tilemap
the entity never had holds no tiles.

### Stacked cells

| Call | Returns |
| --- | --- |
| `Grid.block(volume, column, row, level)` | `String` |
| `Grid.set_block(volume, column, row, level, tile)` | nothing |
| `Grid.tagged(volume, column, row, level, tag)` | `bool` |

A volume's cells sit at an integer level as well as a column and row, and they
name tiles in a tile set several scenes may share rather than indexing a palette
one map carries. Neither shape fits `tile` and `set_tile`, so these are separate
calls: a script that asked for a palette index and got a level-shaped answer
would be a worse outcome than being told the two models differ.

Empty is the empty string, in both directions. A volume stores absence as
absence — a cell that is not there — so unlike the flat map it needs no sentinel
number standing in for nothing, and writing `""` removes the cell rather than
putting something blank in it.

`tagged` asks whether the block in a cell carries one of the words its tile set
gives it: `"tags": ["hot", "liquid"]` on lava, say. The engine reads no tag
itself; tags are how a game says what its blocks mean to *it*, so a script can
burn whoever stands in something `hot` without the engine growing a flag for
every game's idea of ground. An empty cell carries no tags and answers `false`.

Column, row and level must be whole. A cell between two levels is not a cell,
and a script computing one from a float it got wrong should hear about it where
the mistake is rather than land somewhere plausible.

### The top of a voxel world

| Call | Returns |
| --- | --- |
| `Grid.surface(world, column, row)` | `String` |
| `Grid.height(world, column, row)` | `f32?` |

What a voxel world's column has on top, and how high the top of it is: the
block a map view draws there (`"view": "map"`), and what a top-down game is
standing on. `Grid.block` already answers about a cell, but a game seen from
above knows a column and a row, not a level, and asking for the level first
is the question it cannot answer.

The top is the highest block that holds anything up, so water counts and a
flower does not, as for placement. Edits count: a tree cut down leaves its
column's top as whatever it stood on. Nothing in the column is `""` and
`null`, a value a script has to handle rather than a sentinel number that
looks like a height. Column X runs across and row Z runs down, which is how a
map view lays the world out and how `Grid.block` names its cells.

Either is refused on anything that is not a voxel world, naming what it needs.

### A reversible flood on a voxel map

| Call | Returns |
| --- | --- |
| `Grid.set_flood(map, level, block)` | nothing |
| `Grid.flooded(map, column, row)` | `bool` |

`Grid.set_flood(map, level, block)` sets a water overlay on a voxel world with
`view: map`. `level` is a finite continuous voxel height; `block` names a block
in that world's palette whose top face draws the water. `""` clears the overlay.
`Grid.flooded(map, column, row)` is true exactly where the existing surface is
strictly below the flood level. Integer columns and rows use the same coordinates
as `Grid.surface`. Both calls reject a world viewed as blocks.

`Grid.height`, `Grid.surface`, `Grid.block` and `Grid.tagged` still read the
underlying terrain: flood water is not an edit. Scripts decide how submersion
affects movement or gathering. Low Tide's `tide.decay` drives the season and its
crew and crawler read `Grid.flooded`; no game rules live in the engine.

### Standing on the ground

| Call | Returns |
| --- | --- |
| `Grid.walkable(grid, x, y)` | `bool` |

Whether a walker can stand where a point falls. This is the question a script
could not previously ask: occupancy is the engine's own answer and the
pathfinder reads it directly, but the only question a script had was
`Grid.can_reach`, which is about a route between two *entities*. A game that
moves its own player therefore had to tag every solid thing and compare
positions — which knows nothing about terrain, so water was walked over and a
hill walked into, because neither is an entity.

Continuous rather than whole, unlike the cell calls above: a script asks where
it is about to step, and that is a fraction of the way into a cell. Which cell
the point falls in is the grid's arithmetic rather than the caller's, for the
same reason the cell calls give — a script rounding differently would be
standing in one cell and asking about another.

The answer comes from the same walkable surface navigation walks, so a script
and a pathfinding entity cannot disagree about where the ground is. Water,
decoration, an empty column and anywhere off the grid all answer `false`: none
of them is ground a walker stands on, and a caller asking about the edge should
not have to bound the question itself.

It is about a cell, not about an edge. Whether a *step* is legal — the rise
`max_step` allows navigation to climb — is not yet askable, so a script-driven
walker can climb a cliff that a pathfinding one treats as a wall.

Asked of a grid with no volume, or in a host that has bound no tile sets,
nothing about the ground blocks. A host binding no tile sets is one where
nothing could draw the volume either, which is the same reasoning
`Grid.can_reach` uses when it falls back to what the scene authored.

`set_block` refuses a tile the volume's own tile set does not define. Writing an
unknown name would otherwise fail the next extraction, a frame later and nowhere
near the call that caused it; the host holds the tile set precisely so the
refusal lands on the script. A host that has bound no tile sets refuses every
write for the same reason — it cannot tell whether the name is real, and a host
binding none is one where nothing could draw the volume either.

`tile` answers with an index into the map's `palette`, or `-1` where the cell
holds nothing. Reading a cell the map does not have is `-1` as well rather than
an error, because anything that moves will ask about the edge and every caller
would otherwise wrap the call in a bounds check the map can do itself.

`set_tile` takes the same index, or any negative number to empty the cell.
Writing outside the map *is* an error, because unlike reading it has no sensible
meaning and silently dropping it would hide the mistake. So is an index the
palette cannot answer: a map holding one fails validation on the next load, long
after and nowhere near the script that wrote it.

The write edits the stored payload in place rather than going through
`TilemapComponent`, for the same reason the sprite and shape paths do: the view
is `Deserialize`-only, so rebuilding and reserializing it would drop any field
the view does not model.

This is what makes ground a thing gameplay can change — tilled, watered, grown,
burnt, flooded — rather than scenery a script can only put entities on top of.

The second entity must carry a world-space `sindri.tilemap`. Its projection,
cell size, and complete world-XY transform define the coordinate space; there is no implicit
"first grid" and no second set of projection settings for scripts to disagree
with. The coordinates are continuous logical positions, not rounded cell
indices, so a character can move smoothly between cells while gameplay still
speaks in grid axes.

`Grid.place` writes X and Y together and preserves the positioned entity's Z.
Moving, rotating, or scaling the tilemap changes the world position produced by
the same logical coordinate. `position_x` and `position_y` perform the inverse,
including that map transform, so placing and reading round-trip. A tilted map
or one with zero XY scale is refused: this surface describes a grid on Sindri's
world XY plane, not an arbitrary plane in 3D.

The split X/Y reads are a consequence of Decay not having a structured vector
or grid-coordinate value yet. They are kept behind one namespace so that value
can replace the pair later without exposing tilemap storage to gameplay code.

### Animation

| Call | Returns |
| --- | --- |
| `Animation.play(entity, clip)` | nothing |
| `Animation.stop(entity)` | nothing |
| `Animation.restart(entity)` | nothing |
| `Animation.is_finished(entity)` | `bool` |
| `Animation.frame(entity)` | `f32` |
| `Animation.clip(entity)` | `String` |
| `Animation.set_speed(entity, speed)` | nothing |

`clip` names one of the clips the entity's `sindri.animation.sprite` already
holds. A script picks from what the scene authored and cannot build one, which
is what keeps the editor's clip list the whole record of what an entity can do.

The two halves of animation live in two places, and this surface keeps them
there. *Which* clip plays is authored state, so `play` and `stop` write the
component in the world and the playback cursor follows on the next advance —
gameplay writes the world, and playback is derived from it. *Where* the clip has
got to is derived, so `is_finished` and `frame` read the cursor beside the
world, which is what stops watching an animation run from rewriting the scene it
came from.

`play` is idempotent: naming the clip already playing does nothing. A script
says what state it is in on every frame — `if moving { Animation.play(e, "walk") }`
is how Gather's player drives its walk cycle — and a `play` that started the
clip again would hold it on its first frame for ever. `restart` is the other
thing to want, and is the way back to the start of a one-shot that has finished.

Reads answer for the step that has already happened, because scripts run before
animations advance. A clip cannot finish during the frame a script asks about
it, which is what makes `is_finished` mean what it says.

An entity with no `sindri.animation.sprite` is an error rather than a silent
nothing: a script telling something to play a clip it cannot hold is a mistake
worth hearing about on the frame it happens. A clip name the component does not
hold is reported by the advance, the same way a broken clip authored by hand is.

### Sequence

| Call | Returns |
| --- | --- |
| `Sequence.play(entity, sequence)` | nothing |
| `Sequence.stop(entity)` | nothing |
| `Sequence.restart(entity)` | nothing |
| `Sequence.is_finished(entity)` | `bool` |
| `Sequence.time(entity)` | `f32` |
| `Sequence.cued(entity, cue)` | `bool` |
| `Sequence.playing(entity)` | `String` |
| `Sequence.set_speed(entity, speed)` | nothing |

A sequence is choreography authored in the editor's Timeline and held by the
entity's `sindri.sequence`: tracks that move numbers on it, its named
children, or anything in the scene named by a path from the top (`/camera`),
and cues that mark moments. A script names one the
scene holds, exactly as it names an animation clip, and the same two halves
apply: `play`, `stop` and `set_speed` write the component, and the playhead
beside the world follows on the next advance. `play` is idempotent;
`restart` starts the current one again.

Sequences advance after scripts, so `play` takes effect on the step that
calls it, and a cue reached by that advance is answered by `cued` on the next
step — for that one step only. `is_finished` is true once a sequence that does
not loop has reached its end; it then lets go of what it moved, so a script
can take over. A cue may also play a sound, which needs no script at all.

```decay
script Director {
    fn update(dt: f32) {
        if Sequence.cued(this.entity, "landed") { print("touchdown"); }
        if Sequence.playing(this.entity) == "intro" && Sequence.is_finished(this.entity) {
            Sequence.play(this.entity, "idle");
        }
    }
}
```

Sequence Stage (`examples/sequence`) plays an intro, hears its cues and moves
on to an idle loop this way.

### Audio

| Call | Returns |
| --- | --- |
| `Audio.play(clip, volume)` | nothing |
| `Audio.loop(clip, volume)` | nothing |
| `Audio.play_on(bus, clip, volume)` | nothing |
| `Audio.loop_on(bus, clip, volume)` | nothing |
| `Audio.set_volume(bus, volume)` | nothing |
| `Audio.volume(bus)` | `f32` |
| `Audio.stop_all()` | nothing |
| `Audio.pause_all()` | nothing |
| `Audio.resume_all()` | nothing |

`clip` is a logical audio asset ID and `volume` is a finite normalized number
from 0 through 1. `play` is one-shot and `loop` repeats until stopped. Decay
only emits typed playback intent; it never owns or talks to an audio device.
The host drains those requests through the platform audio boundary, which keeps
headless tests silent and lets browser playback obey its user-interaction unlock
without teaching the language about either platform.

Every sound plays through a bus: `play` through `"effects"`, `loop` through
`"music"`, and `play_on`/`loop_on` through any bus named. A bus exists once it
is named, at full volume, and every bus is under `"master"`: a voice is heard
at its own volume times its bus's times the master's. `set_volume` moves a bus
at once for every voice playing through it and every voice started later;
`volume` reads it back, 1 for a bus never set. Bus volumes outlive `stop_all`
and a scene change: they are the player's settings, which a game saves with
`Save.set_number` and restores at start. An authored `sindri.audio.source`
names its bus with `bus`, or leaves it empty to follow `loop`/`play`. Orbital
Last Stand's pause screen moves the master, music and effects buses with three
sliders and remembers them.

### Input actions

| Call | Returns |
| --- | --- |
| `Action.held(name)` | `bool` |
| `Action.pressed(name)` | `bool` |
| `Action.released(name)` | `bool` |
| `Action.axis(name)` | `f32` |
| `Action.vector(name)` | `Vec2` |
| `Action.bindings(name)` | `[String]` |
| `Action.rebind(name, index, binding)` | nothing |
| `Action.last_pressed()` | `String` |

A scene declares its input actions in a `sindri.input.actions` component: a
list of `{ "name", "kind", "bindings" }`, `kind` one of `button`, `axis` or
`vector`, in the actions-document shape `sindri_platform::ActionMap` reads.
A binding is a source name — `key.Space`, `mouse.Left`, `gamepad.South`,
`gamepad.axis.left_x` — or a composite: `{"axis": {"negative", "positive"}}`
or `{"vector": {"up", "down", "left", "right"}}`. Every action reads the
strongest of its bindings, so one bound to a keyboard and a pad answers to
whichever is being used. A declaration that does not read is reported once,
when it changes, and leaves the scene with no actions.

Scripts read actions by name; a name the scene does not declare is an error
listing the ones it does. Every script in a pass reads the same step's values,
worked out before any script runs. `bindings` and `rebind` speak one text form:
one source, `negative/positive` for an axis, or `up/down/left/right` for a
direction. `rebind` replaces the binding at `index`, or adds one at the next
index, refuses one that cannot make the action's kind, and writes the change
into the scene's component, so it is read on the next step and the editor's
Play shows it. `last_pressed` names the key, mouse button or pad button pressed
this step, for a screen that waits for the key to rebind to. A rebinding
lasts as long as the scene: `Save` holds numbers and flags, not text, so
keeping one between sessions is not yet possible. The platformer's hero runs and jumps by actions, and the Input
example rebinds them.

### The keyboard

| Call | Returns |
| --- | --- |
| `Input.axis(negative, positive)` | `f32`, one of -1, 0, 1 |
| `Input.is_down(key)` | `bool` |
| `Input.just_pressed(key)` | `bool` |
| `Input.just_released(key)` | `bool` |

Keys are named physically, by where they are rather than what they type, so a
binding survives a change of layout: `"W"`, `"ArrowLeft"`, `"Space"`,
`"Digit1"`, `"ShiftLeft"`. Matching ignores case, because the name is typed by a
person. `sindri_platform::Key::ALL` is the list.

**A name nothing answers to is refused**, not read as never-held. A control that
silently does nothing is a bug report nobody can reproduce.

Holding two opposing keys gives an axis of zero, so opposed movement keys cannot
cancel into whichever the operating system reported last. A press is not a hold:
`just_pressed` is true for one frame however long the key stays down, and the
operating system's key repeat is not a second press.

### Gamepads, and which player holds which

| Call | Returns |
| --- | --- |
| `Gamepad.joined()` | `f32`, the player slot claimed this frame, or 0 |
| `Gamepad.left()` | `f32`, the player slot given back this frame, or 0 |
| `Gamepad.count()` | `f32`, how many slots are held |
| `Gamepad.is_connected(slot)` | `bool` |
| `Gamepad.is_down(slot, button)` | `bool` |
| `Gamepad.just_pressed(slot, button)` | `bool` |
| `Gamepad.just_released(slot, button)` | `bool` |
| `Gamepad.axis(slot, axis)` | `f32` |

A pad is read by **player slot**, not by device. The first pad a face button or
Start is pressed on claims slot 1, the next slot 2, up to eight. A slot stays
claimed while its pad is connected and is given back when it is unplugged, so a
couch game adds a player when `joined()` is non-zero and removes one when
`left()` is. Each is an edge like a key press, and never names two slots in one
frame: a second pad pressing in the same frame joins the frame after, so reading
one join per frame misses nobody. Pressing Play again in the editor starts the
players over.

**Slot 0 is every pad at once**, for a game with one player who might pick up
any of them: `Gamepad.just_pressed(0, "south")` is a press on any pad.

The press that claims a slot is not reported through that slot. It meant "I am
playing", and a game reading it again as "jump" would have every player jump as
they join. Slot 0 still sees it.

Buttons are named by position, so one name means the same button on every make
of pad: `"south"`, `"east"`, `"west"`, `"north"`, `"left_bumper"`,
`"right_bumper"`, `"left_trigger"`, `"right_trigger"`, `"select"`, `"start"`,
`"left_stick"`, `"right_stick"`, `"dpad_up"`, `"dpad_down"`, `"dpad_left"` and
`"dpad_right"`. Axes are `"left_x"`, `"left_y"`, `"right_x"`, `"right_y"`,
`"left_trigger"` and `"right_trigger"`. Sticks read -1 to 1 in screen axes, as
`Stick` does, right and *down* positive, with a round dead zone taken out so a
stick at rest reads zero and a diagonal push is not snapped to an axis.
Triggers read 0 to 1. A name nothing answers to is refused, as a key's is, and
so is a slot that is not a whole number from 0 to 8.

An action binding reads any pad as `gamepad.south` or `gamepad.axis.left_x`.

### Where the person is pointing

| Path | Type |
| --- | --- |
| `Viewport.aspect` | `f32` |

`Viewport.aspect` is the viewport width divided by its height. It is a screen
fact, not a camera: a scene that authored its camera can combine the aspect with
that authored framing to decide which world positions are visible, without the
host guessing which camera controls gameplay.

| Path | Type |
| --- | --- |
| `Camera.pan_x` | `f32` |
| `Camera.pan_y` | `f32` |
| `Camera.pan_z` | `f32` |

`Camera.pan_{x,y,z}` is the gameplay camera's accumulated world-space pan offset.
A script writes the offset it wants; the host applies it to the authored camera
without changing the scene's authored transform.

| Call | Returns |
| --- | --- |
| `Camera.add_trauma(amount)` | unit |

`Camera.add_trauma` adds a finite, non-negative amount to the authored gameplay camera's shake trauma, clamped by the engine to one. Decay decides when an impact happens; `sindri.camera.behavior` owns the waveform, strength, frequency, and decay. The call requires exactly one authored `sindri.camera` carrying `sindri.camera.behavior`; ambiguity or absence is a runtime error rather than a silently ignored camera effect.

| Path | Type |
| --- | --- |
| `Pointer.x` | `f32` |
| `Pointer.y` | `f32` |
| `Pointer.overlay_x` | `f32` |
| `Pointer.overlay_y` | `f32` |
| `Pointer.position` | `Vec2` |
| `Pointer.overlay` | `Vec2` |
| `Pointer.inside` | `bool` |
| `Pointer.over_ui` | `bool` |

| Call | Returns |
| --- | --- |
| `Pointer.is_down(button)` | `bool` |
| `Pointer.just_pressed(button)` | `bool` |
| `Pointer.just_released(button)` | `bool` |

| Path | Type |
| --- | --- |
| `Touch.count` | `f32` |

| Call | Returns |
| --- | --- |
| `Touch.x(index)` | `f32` |
| `Touch.y(index)` | `f32` |

| Path | Type |
| --- | --- |
| `Gesture.tapped` | `bool` |
| `Gesture.tap_x` | `f32` |
| `Gesture.tap_y` | `f32` |
| `Gesture.held` | `bool` |
| `Gesture.hold_x` | `f32` |
| `Gesture.hold_y` | `f32` |
| `Gesture.dragging` | `bool` |
| `Gesture.drag_x` | `f32` |
| `Gesture.drag_y` | `f32` |
| `Gesture.pinching` | `bool` |
| `Gesture.pinch` | `f32` |

`Gesture` says what the person *meant*, where `Pointer` says which button is
down. The difference matters as soon as a game is played with a finger: a mouse
has a second button to mean "remove" and a finger does not, so removing becomes
a hold, and panning becomes a drag that must not also count as a tap when it
ends. None of that can be worked out from `Pointer` alone, because the
difference between a tap and the start of a drag is how the press *ends* and how
far it wandered getting there — a question about a press's whole life rather
than about this instant.

Each gesture is guarded by its own question for the reason `Aim.hit` is: nothing
happened is the common case, and every coordinate reads zero when it did, which
is a real place on the screen. `Gesture.pinch` reads one rather than zero when
no pinch is happening, so a script that multiplies by it without asking does not
collapse its camera to a point.

```rust
script Build {
    fn update(dt: f32) {
        // Drag pans, tap builds, hold takes back. The same three lines are the
        // mouse controls and the touch controls.
        if Gesture.dragging {
            Camera.pan_x = Camera.pan_x + Gesture.drag_x;
            Camera.pan_y = Camera.pan_y + Gesture.drag_y;
        }
        if Gesture.tapped && Aim.hit {
            Grid.set_block(floor, Aim.place_x, Aim.place_y, Aim.place_z, "stone");
        }
        if Gesture.held && Aim.hit {
            Grid.set_block(floor, Aim.x, Aim.y, Aim.z, "");
        }
    }
}
```

How still a press has to be and how long a hold takes are authorable, because
the answer is not universal: a stylus is steadier than a thumb, and a game
played at arm's length wants more slack than one played at a desk.

`Pointer` is **one namespace for the mouse and the finger**, and that is the
whole point of it: a game that aims at a point should not have to ask which
device the person is using, and a game written for a mouse then works on a phone
without a second code path.

```rust
script Aim {
    fn update(dt: f32) {
        if Pointer.inside && Pointer.is_down("Left") {
            this.transform.position.x = Pointer.x;
            this.transform.position.y = Pointer.y;
        }
    }
}
```

What each unified answer means when both a mouse and a finger are present:

- **Position** is the mouse when there is one, and the first finger otherwise. A
  machine with both is a machine someone is using the mouse on.
- **`is_down("Left")`** is the left mouse button *or* any finger. A tap and a
  click are the same line of gameplay code, which is the convention the web
  settled on. A finger is `Left` and nothing else — it is not a right-click.
- **`just_released("Left")`** for a finger means the *last* one left. A second
  finger lifting while one is still down is not the pointer coming up, any more
  than releasing the right mouse button releases the left.

Buttons are named `"Left"`, `"Middle"`, `"Right"`, matching case-insensitively
because the name is typed by a person. **A name nothing answers to is refused**,
exactly as a key name is: a control that silently does nothing cannot be
reproduced.

`Pointer.inside` is false when the mouse has left the window and nothing is
touching the screen. A position read while it is false is zero rather than an
error — the mouse leaving mid-frame is an ordinary thing, not a mistake in a
script — so a script that cares must ask `inside` *before* it believes a
position.

**Coordinates are viewport pixels with the origin at the top left of the
viewport**, which is the same thing on every host: the window on native and in
the browser, and the Game view's own rectangle in the editor. A script reading
`Pointer.x` gets the same meaning in editor Play as in the real build, which is
what makes playtesting in the editor worth anything.

Viewport pixels are *physical* ones — the same pixels the surface is configured
with and the same ones a screen element's hit rect is worked out in. This page
used to say logical, and it was wrong in a way that only showed up off the
desktop: on a device reporting three physical pixels per logical one, a position
converted to logical is a third of the way to where the person actually pressed,
so everything below the top third of a phone screen was unreachable. A scale
factor of 1.0 makes the two spellings identical, which is why a desktop, and
every test written on one, could not tell them apart.

The practical consequence for a game: `x` and `y` are bigger numbers on a dense
display than on a coarse one, for the same physical spot. A game comparing them
against authored constants wants `overlay_x` and `overlay_y` instead.

**`overlay_x` and `overlay_y` are the same point in the overlay's units** — two
tall, centred on the origin, running out to the aspect ratio either side. They
exist because `x` and `y` are viewport *pixels*, and how many pixels tall a
window is is not something a scene knows: a script could say where the pointer
was on the screen and not what it was pointing at.

They stop at the overlay rather than going on to the world, because going on
means a camera, and a script that could ask a camera anything would be a script
the renderer answers to. A scene authored its own camera and knows how much
world it frames, so the conversion is a multiplication the game owns:

```rust
// The camera frames `view_height` world units from top to bottom.
let world_x = Pointer.overlay_x * this.view_height * 0.5;
let world_y = Pointer.overlay_y * this.view_height * 0.5;
```

A host laying out no UI has no overlay, and these read zero for the same reason
a position read while the pointer is outside does.

`Touch` is the raw fingers, for a game that wants more than "where is the person
pointing" — a second finger, or a pinch. `Touch.count` is how many are down, and
`Touch.x`/`Touch.y` take which one, counting from zero, in a stable order: a
finger keeps its place while it stays down, so a drag cannot jump from one
finger to another. Asking for a finger that is not down is refused rather than
answered with zero, which would read as a finger in the corner of the screen.

### Pointing at a block

| Path | Type |
| --- | --- |
| `Aim.hit` | `bool` |
| `Aim.x` | `f32` |
| `Aim.y` | `f32` |
| `Aim.z` | `f32` |
| `Aim.place_x` | `f32` |
| `Aim.place_y` | `f32` |
| `Aim.place_z` | `f32` |

`Pointer` stops at the overlay, and the section above says why: going on to the
world means a camera. `Aim` is the host going on anyway, for the one case where
a game cannot do the multiplication itself — a world made of blocks, where the
answer is not a point on a plane but *which block, and which of its six sides*.
No amount of arithmetic on `overlay_x` recovers a side.

So the host works it out before scripts run, exactly as it works out which
screen element the pointer is over: it holds the camera and the viewport, casts
the ray, and walks the volume. A script reads the result.

`Aim.x`, `y` and `z` are the block under the pointer — the column, row and
level of the cell you would remove. `Aim.place_x`, `place_y` and `place_z` are
the empty cell against the side being looked at — where a block attached by
this click would go. Both are needed because a cell alone cannot say which of
its six neighbours a click meant, which is the whole reason picking reports a
face at all.

**Ask `Aim.hit` first.** Every other value here reads zero when the pointer is
on nothing, and zero is a real cell: a script that skipped the check would
quietly build a tower at the origin whenever the pointer crossed the sky. It
reads false rather than failing, because pointing at nothing is an ordinary
thing to do — and false is also what a host that picked nothing reports, so a
game running without a pointer finds the person pointing at nothing rather than
an error.

Together with `Grid.set_block`, that is a whole builder:

```rust
if Aim.hit && !Pointer.over_ui {
    if Pointer.just_pressed("Primary") {
        Grid.set_block(floor, Aim.place_x, Aim.place_y, Aim.place_z, chosen);
    }
    if Pointer.just_pressed("Secondary") {
        // The empty string clears a cell.
        Grid.set_block(floor, Aim.x, Aim.y, Aim.z, "");
    }
}
```

Only a grid whose cells are boxes has sides to point at. A projected grid draws
a picture of blocks, and a picture has no near side, so `Aim.hit` is false over
one however convincing it looks.

### Steering with a thumb

| Path | Type |
| --- | --- |
| `Stick.x` | `f32` |
| `Stick.y` | `f32` |
| `Stick.held` | `bool` |
| `Stick.anchor_x` | `f32` |
| `Stick.anchor_y` | `f32` |
| `Stick.direction` | `Vec2` |

`Stick` is a joystick made out of whichever finger is steering. It anchors
where that finger landed, and `x`/`y` are how far it has been pulled from
there — from -1 to 1, in screen axes, never longer than 1 no matter how far
past the radius the thumb goes.

This is a different question from `Pointer`, which is why it is a different
namespace. `Pointer` says *where on the screen* the person is pointing, which is
absolute. `Stick` says *which way and how hard* they are pushing, which is
relative to wherever they put their thumb down. A game that steers wants the
second, and steering towards `Pointer` instead gives the ship a lunge every time
a thumb lands, no way to ask for gently-left, and a thumb parked over the part
of the screen the player is trying to watch.

`held` is not the same as a non-zero reading: a thumb resting inside the dead
zone reads centred but is still holding the stick, which is what a game drawing
the control needs to know. `anchor_x`/`anchor_y` are where the thumb landed, so
a game that wants to draw the ring draws it from the same numbers the input came
from rather than from a second guess at where the stick is.

**This page used to say there was no drag abstraction, deliberately** — that a
game's deadzone and radius are tuning, and that baking one game's numbers into
an engine is how an engine acquires a genre. The principle stands and the
conclusion did not. What is genre-specific is the *numbers*; what is not is the
*shape* — anchor where it lands, clamp at the radius, rescale out of the dead
zone, centre on release, and keep the press that started it so a second thumb
cannot snatch the steering mid-turn. That shape is the same in every game that
has ever had a stick, and "four lines that belong to the game" is four lines
each game gets subtly wrong: this engine's own game shipped steering that
followed the finger. So the shape is here and the numbers are authorable, which
is what the original objection was actually asking for.

### The frame

| Path | Type |
| --- | --- |
| `Time.delta` | `f32` |
| `Time.elapsed` | `f32` |

`delta` is what `update` receives as its argument, offered again so a function
that is not `update` can reach it. `elapsed` is per script instance rather than
per world: a script attached later has not been running as long.

### Timers

```decay
script Mine {
    var cooldown = Timer(0.0);
    fn update(dt: f32) {
        if !this.cooldown.done { return; }
        this.cooldown = Timer(2.4);
        drop();
    }
}
```

A `Timer` replaces the countdown a script used to keep by hand — a number, a
`-= dt` every frame, and a check. **A timer a script's field holds runs down
by each frame before that script's `update`**, whatever the update then does,
so an early `return` above the countdown no longer stops the clock, which is
the mistake the hand-written form invites. A timer started this frame is not
run down until the next; one in a local is never run down.

`done`, `left`, `duration` and `progress` read it; a new `Timer(seconds)`
restarts it. Another script reads one through the script's type
(`Mine.on(e).cooldown.left`). The inspector shows a timer field as empty: it
is the running game's, not the author's.

A countdown that should pause — Scorchball's kickoff countdown, which runs
only while players are counting down — is still a number the script runs
down itself. Scorchball's banners, power-ups, burning, aftertouch and bot
reaction times are timers.

### Shared functions

```decay
// view.decay — any file in the project.
shared fn visible_half_x(view_size: f32) -> f32 {
    let aspect = max(Viewport.aspect, 0.001);
    if aspect >= 1.0 { return view_size * 0.5 * aspect; }
    return view_size * 0.5;
}

// Any script, in any file.
let half = visible_half_x(VIEW_SIZE);
```

A function written outside any script belongs to its file: the scripts in
that file call it by name, and no other file sees it. Declared `shared fn`,
it belongs to the project: every script calls it by name, checked across
files, with no `import` line — as every script already sees the others'
types, events and state. Sharing is a decision the author writes down, so a
helper one file keeps for itself never collides with another file's.

Neither kind has a `this`, so what it needs, it is passed; either can call
the engine and other top-level functions.

Each script that calls a shared function runs its own copy, linked in when
it is compiled, so editing one recompiles everything that calls it. A shared
function in a file that does not compile fails the scripts that call it, with
that file's errors, and no others. Declaring one twice, or with a name a
script, event, state or engine function already has, is refused.

Orbital Last Stand and Orbital Baked each keep `visible_half_x`,
`visible_half_y` and `on_screen` as `shared fn` in `view.decay`, where
twenty-five scripts used to carry identical copies.

### Constants

```decay
// view.decay — any file in the project.
shared const VIEW_SIZE: f32 = 11.0;

// Any file. This one's own, worked out from the project's.
const EDGE: f32 = VIEW_SIZE / 2.0 + 1.2;
const OPENING: Phase = Phase.Lobby;
```

A constant is a name for a value worked out when the project compiles, so it
never changes while the game runs and costs what the literal would. It holds
a number, a flag, text or an enum's variant, and its value may use literals,
operators, variants and other constants — in any file, for shared ones — but
cannot call anything or read the world. Like a function, a plain `const` is
its file's and a `shared const` the project's; assigning one, declaring one
twice, or giving it a name something else already has is refused.

Reach for one where a value must be the same everywhere and nothing should
tune it per entity. Where a scene should be able to tune it — a speed, a
cooldown — keep an `@export` field instead: the inspector shows fields, not
constants.

Orbital Last Stand and Orbital Baked each declare `VIEW_SIZE`, the camera's
`vertical_size`, once in `view.decay`, where every enemy, hazard and the
director used to carry the same `@export let view_size: f32 = 11.0;`.

### Other scripts, by type

Every `script` in a project is a type every other script can name, whichever
file it is in:

```decay
// In turret.decay, about the Bolt declared in bolt.decay.
let shot = Bolt.on(World.spawn(this.bolt));
shot.damage = 3.0;              // a field, checked when it compiles
shot.heading = Vec2(0.0, 1.0);

let hit = Bolt.on(other);       // null when `other` does not run Bolt
if hit != null {
    this.hp -= hit.damage;      // its live value, not what the scene authored
    hit.bounce(2.0);            // a message
}
```

`Name.on(entity)` answers with the entity typed as that script when it runs
it, and `null` when it does not. A script type is also an `Entity`: it reaches
`transform` and the rest, and goes wherever an entity does. Where a script's
own name matches an entity's (`transform`, say), the entity's wins.

**Fields are live.** Reading one reads the other script's field as it is now;
writing one changes it for that script's next line. A `let` cannot be written
once its script has started. On a script that has not started yet — one
spawned a moment ago — a write sets what it starts with, applied after
anything the scene authored, and may name things only a running game has, such
as another entity. This is what `World.set_property` did by name, and the
typed form checks the name and the type. So that a project can move over one
call at a time, a starting value written to an `@export` field is also what
`World.property_number` reads, exactly as `World.set_property` made it.

**A call is a message.** It is not run when it is made: it is queued, and
delivered once the pass has run, in the order sent. A message returns nothing,
may send more — delivered in a following round, up to eight — and to an entity
that has gone it simply does nothing, which is the ordinary end of a
projectile. A conversation that never settles is stopped and reported.
Lifecycle functions (`start`, `update`) are the engine's to call and are not
messages.

**Everything is checked when it compiles**: an unknown script, a misspelt
field or message, a wrong argument, and using a message's result. A name two
scripts declare is left out — which one was meant would be a guess — as is one
the engine already uses.

A reference is reached however it is held: in a local, in a field
(`this.target.hit(1.0)`), or straight from the lookup
(`Bolt.on(hit).damage`), which is computed once and then walked.

Because any script may name any other, every `.decay` file in a project is
loaded and exported, not only those a scene names; the editor's error preview
and `decay-lsp` check each script against its whole project.

### Events

A message is for one script the sender knows. An event is for whoever cares:
the sender says what happened and does not need to know who is listening.

```decay
// events.decay — any file in the project; each event is declared once.
event GoalScored(team: f32);

// ball.decay
GoalScored.emit(1.0);

// match.decay, and any other script that cares.
script Match {
    on GoalScored(team) {       // types come from the declaration
        this.score += 1.0;
    }
}
```

**An event is delivered like a message**, after the pass and in the order
emitted, to every running script with an `on` handler for it — in the order
the pass runs them, and the sender's own handler included. An event nobody
handles goes nowhere, quietly. An event emitted by a handler is delivered in
a following round, under the same bound of eight as messages, and one that
never settles is stopped and reported.

**Everything is checked when it compiles**: an unknown event, a wrong value in
an emit, a handler taking the wrong number or type of values, and calling an
event as though it were a function (`GoalScored(1.0)` rather than
`GoalScored.emit(1.0)`). A handler may leave a parameter's type unwritten and
take the event's; an event's own parameters must be typed, since every handler
relies on them. An event declared in two files cannot be used until one is
removed, and an event may not take a name a script or the engine already has.

A handler returns nothing, because nothing waits for it, and is not a function
a script can call by name. `event` and `on` are special only where no other
name could stand — at the top of a file, and at the start of a member — so
`Bolt.on(hit)` and a local called `event` keep working.

Scorchball's goals, wind and fireball are events; its kicks, aftertouch and
power-ups are messages to the one script they concern.

### The board scripts share

| Call | Returns |
| --- | --- |
| `Game.get(name, fallback)` | `f32` |
| `Game.set(name, value)` | nothing |

The smallest thing that lets two scripts cooperate, and it predates references:
when a script could not name another entity at all, leaving a number under a
name was the only way for one to tell another anything.

References do not retire it. A board is still the right shape for a fact that
belongs to the game rather than to an entity — a score, a countdown, whether the
game is won — and for those, `World.find` every frame would be a lookup to answer
a question no entity owns. What references replace is the *workaround*: a
collectible no longer publishes its position as two numbers for a player to
compare against, it asks the player where it is.

The fallback on `get` is **not optional**, because a note nobody has left yet is
the ordinary case on the first frame. A `get` that silently answered zero would
make a mistyped name read as a legitimate value, which is the failure this whole
surface is arranged to avoid.

The board is runtime state and goes when a run does — stopping and playing again
does not begin with the last game's score.

This was deliberately a stopgap, and its shape admits it: names are strings and
nothing checks them. Declared state, below, is the checked form of the same
board.

### Declared state

```decay
// game.decay — any file in the project.
enum Phase { Lobby, Countdown, Play }

state Game {
    var phase = Phase.Lobby;
    var score_blue: f32 = 0.0;
    var won: bool = false;
    let target: f32 = 5.0;
}

// Anywhere else.
Game.score_blue += 1.0;
if Game.score_blue >= Game.target { Game.won = true; }
```

A `state` declares values every script shares, each with one type and one
starting value, written once. A misspelt name is a compile error rather than a
fallback read silently, and two uses can no longer disagree about what the
fallback is — Orbital Last Stand reads `hp` with five different ones.

- **Several files may add to one state**, as long as no field is declared
  twice; a field declared twice is refused where it is declared and wherever
  it is used, since the file declaring it may be one nothing runs.
- **A field holds an `f32`, a `bool` or an enum** and starts from a literal
  or a variant, because it exists before any script has run to compute it
  from. On the board an enum is kept as the variant's position, `0` for the
  first, so `Game.get("phase", 0.0)` still reads a number. `let` makes one no
  script may change: a tuning value every script agrees on.
- **`Game` fields live on the board, under their own names.** `Game.score`
  and `Game.get("score", 0.0)` read the same number, and a `true` is `1.0`, so
  a project moves over one call at a time and a test reading the board sees
  either. A field nothing has written yet reads as its declared start.
- **A state of another name** — `state Tuning { let gravity: f32 = 30.0; }` —
  is reached as `Tuning.gravity` and kept on the board as `Tuning.gravity`.
  It may not take a script's or an event's name, and may add to a namespace
  the engine offers only as `Game` does: without redefining its members.

Like the board, state is runtime state and goes when a run does. Scorchball's,
the platformer's and Orbital's shared values are declared this way.
Each Orbital game keeps its declarations in `game-state.decay` and still reads a
few values with `Game.get`: keys built at runtime, and eight values whose
callers disagree about the fallback, which need a starting value chosen before
they can be declared.

### Enums and `match`

```decay
// game.decay — any file in the project.
enum Power { Grow, Boost, Wind, Fire }

// powerup.decay
script PowerUp {
    @export let kind = Power.Grow;

    fn give(player: Player) {
        match this.kind {
            Power.Grow | Power.Boost => { player.power_up(this.kind); }
            Power.Wind => { WindPicked.emit(1.0); }
            Power.Fire => { FirePicked.emit(player.team); }
        }
    }
}
```

An `enum` names a fixed set of values, and every script in the project sees
it, as it sees the others' types, events and state. A variant is written with
its enum's name — `Power.Grow` — and is not a number: it is compared with
`==` and `!=` and nothing else. `match` runs the arm its value names and must
say what happens for every variant or end with `_`, so a variant added later
is a compile error at every `match` that has not caught up.

Where a value goes, `match` gives one — each arm `=>` a value, separated by
commas, every arm the same type:

```decay
let clip = match this.kind {
    Power.Grow => "enlarger",
    Power.Boost => "push",
    Power.Wind => "wind",
    Power.Fire => "fireball",
};
```

Scorchball picks each power-up's sign this way, where it declared a `var` and
assigned it in every arm.

An enum `@export` is authored **by the variant's name**: the scene stores
`"kind": "Grow"`, the inspector offers the variants as a dropdown, and a name
the enum does not have is refused with the ones it does. A `state` field may
hold one too. `print` shows a variant by its full name, `Power.Grow`.

Scorchball's match phase and its four power-ups are enums, where they were
`0`/`1`/`2` and `1`–`4` with a comment to decode them.

### Text

```decay
let clip = "d" + score % 10.0;              // "d0" to "d9"
Game.set(stat + "_add", 0.0);               // "damage_add"
if name.starts_with("enemy_") { }
```

`+` joins text with text, numbers, `bool`s, vectors and variants: a whole
number is written without its `.0`, and a variant by its own name (`Lobby`).
Text has `length`, `uppercase`, `lowercase` and `trimmed`, and
`contains`, `starts_with`, `ends_with`, `find`, `slice` and `replace`,
counting characters rather than bytes; `decay/LANGUAGE.md` has the table. A
field initialized with a text literal is a `String` without an annotation.

Text is capped at 64 KiB, so a loop that keeps doubling a string fails that
entity's script rather than the editor.

A number is written a set way by asking it: `n.fixed(2)` gives exactly two
decimals (`"12.50"`), and `n.padded(2)` the nearest whole number led with
zeros to two digits (`"05"`). Neither writes `-0`.

```decay
shared fn clock_text(seconds: f32) -> String {
    let whole = floor(max(seconds, 0.0));
    let minutes = floor(whole / 60.0);
    return minutes.fixed(0) + ":" + (whole - minutes * 60.0).padded(2);   // "1:05"
}
```

A HUD label whose words a designer owns is still best as a template in the
scene — `"Score {}"` filled with `Ui.set_number` — so the words stay in the
scene file. Reach for `fixed` and `padded` where a script composes the text
itself, or where a slot cannot say it: `{.2}` asks for decimals, not digits.

Scorchball picks its score digit's and ball ring's clips by joining, where it
had ten and six `if`s, and Orbital clears each stat's `_add` and
`_mul` keys from its one name. Orbital writes its run clock and
the results screen's "Survived" time with `clock_text`, where the scene's
`"{}:{.2}"` read 65.3 seconds as `1:5.30`.

### Lists and ranges

```decay
var taken: List<f32> = [];
for slot in 0..4 {                 // 0, 1, 2, 3
    let pick = choose(taken);
    taken.push(pick);
}
if taken.contains(7.0) { }
for card in ["Guidance Kernel", "Arc Imprint"] { this.slots.push(World.find(card)); }
```

A script writes a list with `[a, b]` and changes the one a `var` or field
holds with `push`, `pop`, `insert`, `remove_at`, `clear` and `xs[i] = v`;
`contains`, `index_of` and `length` ask it. `List<T>` is the same type a
host query such as `World.with_tag` hands back — `Array<T>` is its older
spelling — so a group from the world can be kept, filtered into another
list and walked like one the script built. A list is a value: assigning one
copies it, so a change never reaches a list somewhere else.

`for i in 0..count` walks `0` up to, not including, `count`, without making a
list of them. A script's list holds at most 10,000 elements; growing one past
that fails the script rather than the editor.

Orbital's module chooser keeps its four offer slots in one list built from the
cards' names, where it had twelve numbered fields, walks its catalogue with
ranges, and passes the modules already on offer as one list rather than three
parameters.

### Maps

```decay
var kills: Map<Kind, f32> = [:];
kills[kind] = kills.get(kind, 0.0) + 1.0;
let prices = ["arc": 3.0, "nova": 5.0];
for name in prices.keys() { print(name + " " + prices[name]); }
```

A map finds values by key: text, numbers, flags, enum variants or entities.
`m[key] = v`, `remove` and `clear` change the one a `var` or field holds;
`m[key]` reads a key it must have, `get(key, fallback)` one it may not, and
`contains`, `keys()`, `values()` and `length` ask it. Its keys are listed in
the order they were first set, so a walk over one is the same every run. Like
a list it is a value, copied where it is assigned, and holds at most 10,000
keys. A scene cannot author one yet: an `@export` map starts as the script
wrote it.

### Colours

```decay
const HURT: Color = Color("#ff4060");
this.sprite.tint = HURT.lerp(this.base, this.recovery);
this.shape.stroke = Color(1.0, 0.9, 0.6).with_alpha(this.shape.stroke.a);
```

`Color` holds `r`, `g`, `b` and `a`, and the engine's colour fields are of
that type, so a tint is read, held, blended and assigned whole — where a
script used to write three or four channels one by one — while
`this.sprite.tint.a` is still one number. Build one from three or four
numbers or from `"#rrggbb"`; a palette is best written once as constants.
Orbital's chargers colour their elite traits this way.

### Values that may be missing

```decay
fn first_free(slots: List<bool>) -> f32? {
    for i in 0..slots.length { if !slots[i] { return i; } }
    return null;
}

let slot = first_free(this.taken);
if slot == null { return; }          // full
this.taken[slot] = true;             // `slot` is an `f32` from here on
let shown = best_time ?? 0.0;        // or give a fallback
```

`f32?` — and `String?`, `Vec2?`, `List<T>?` — is a value or `null`, where a
script used to return `-1` and hope every caller checked. It is never used as
a number until `??` gives a fallback or a check says it is there: inside
`if x != null { }`, or after `if x == null { return; }`, a `let` or parameter
`x` is its plain type. Entities, structs and enums may be `null` already and
need no `?`.

### Structs

```decay
// Any file in the project.
struct OfferCard { card: Entity, name: Entity, blurb: Entity }

// Any script.
var slots: List<OfferCard> = [];
slots.push(OfferCard(card: World.find(n), name: World.find(n + " Name"), blurb: World.find(n + " Blurb")));
let shown = slots[slot];
Ui.set_text(shown.name, title);
```

A `struct` keeps values that belong together in one value, where a script kept
parallel lists or numbered fields in step by hand. Every script in the project
sees every struct, as it sees enums. One is built with every field named,
read with `.field`, and written through the `var` that holds it; like a list
it is a value, so assigning one copies it. `decay/LANGUAGE.md` has the rules.

A struct `@export` is authored in the scene as an object of its fields, and
the inspector draws a row for each; a field the scene leaves out keeps the
script's default for it. A `state` holds numbers, flags and enums only.
`print` shows every field.

A struct may have methods, written after its fields and asked of one value:

```decay
struct OfferCard {
    card: Entity, name: Entity, blurb: Entity,

    fn show(title: String, text: String) {
        Ui.set_text(this.name, title);
        Ui.set_text(this.blurb, text);
        World.set_active(this.card, true);
    }
}

this.slots[slot].show(title, blurb);
```

`this` in a method is the value, read as `this.card`; it is a copy, so a
method that works something out returns it rather than changing `this`.

A field may have a default, `weight: f32 = 1.0`, and building one may then
leave that field out. A default is worked out when the project compiles, as a
constant is, so it is a number, flag, text or variant, and may name a
`shared const` from any file.
Methods are checked like any call and, like the struct, reach every file.

Orbital's module chooser keeps each offer slot's card, name and blurb in one
`OfferCard`, where it kept three lists in step, and asks a slot to `show` a
module or `hide`, where it set each of the three itself.

### Saying something

`print(anything)` puts a line in the host's log, tagged with the entity that
said it. It takes any type, so `print(hp)` needs no text around it; to say more,
join it: `print("hp " + hp)`. A number prints as text spells it — `3`, `0.1` —
and a variant by its full name, `Phase.Play`.

### Maths

`abs`, `sqrt`, `sin`, `cos`, `exp`, `atan2`, `min`, `max`, `floor`, `ceil`,
`round`, `sign`, `clamp`, `lerp`.
That is the entire standard library of functions, and `PI` and `TAU` are the
two named numbers.
Decay has no modules and no imports, so each is a bare global name, and every one
added is a name a script can no longer use for its own. `exp` is what lets a
script write frame-rate-independent interpolation such as
`1 - exp(-speed * dt)` without inventing an approximation in gameplay code.

`clamp(value, low, high)` keeps a number between two others, and takes bounds
given the wrong way round as meant rather than failing. `lerp(a, b, t)` is the
number `t` of the way from `a` to `b`. `sign` is -1, 0 or 1 — zero for zero,
which is the answer "which way is this moving" wants. `round` halves away from
zero, so `round(-2.5)` is `-3`.

These work on numbers. A vector has its own `length`, `normalized`, `dot`,
`distance` and `lerp`, described in `decay/LANGUAGE.md`, because a vector
operation belongs to the value rather than to a global name.

### Why this list can be trusted

**Every path above is typed, so a misspelling is a compile error.**
`this.transfrom` and `this.transform.position.w` are both refused with a line
number when the script compiles, rather than failing on the first frame with a
path name and no idea where it came from.

That holds because the tables above are not written twice. `surface.rs` is the
single description; the analyzer's `Environment` and `WorldHost`'s accessors are
both derived from it, and a test walks every path the analyzer would accept and
asserts the host answers it. A path accepted by one and not the other is the
worst failure available here — a clean compile followed by a runtime error — and
it cannot be shipped.

**And this document is checked against that description.**
`crates/sindri-decay/tests/documented_surface.rs` parses the tables above and
asserts they name exactly what a script can reach — no more, no less. A surface
that grew without the documentation growing with it fails the build, because a
list of what a script can do is believed, and one that is quietly wrong is worse
than none at all.

### What is deliberately absent

Two absences, each for a reason worth stating.

**No full 3D rotation**: a gameplay script should not be asked to assemble a
quaternion by hand, and offering a third of a 3D rotation API is worse than
offering none.

**No query by component type.** A script asks for a group by authored tag, and
that is the only way to ask. Spelling `sindri.sprite` in a script would put
engine internals in gameplay code, and it would make every enemy that happens to
have a sprite an enemy. If a game wants a group, it says so by tagging it.

The **Z lock is honoured**. A script is a write path like any other, and one
that could ignore the lock would be the hole that makes the lock worthless.

## Lifecycle

`start()` runs once, after the authored properties are applied and before the
first update. `update(dt)` runs once a frame with the frame's delta. Both are
optional; a container declaring neither simply does nothing.

Both signatures are exact — `start` takes no parameters, `update` takes one.
One way of saying a thing beats two that can disagree, and an arity mismatch
reports itself clearly.

## Where the state lives

The same split `docs/scene-extraction.md` describes for sprite animation, for
the same reason. A clip and its timing are authored, so they are in the scene;
the frame it has reached is not, so it is not.

A script's fields drift as it runs. `Scripts` holds the live instances beside
the world rather than in it, so watching a scene play is never an unsaved change
to the file it came from.

## Where this surface came from

Not invented. `docs/2d-inventory.md` read the legacy engine's
`examples/scripted_asteroids/scripts/player.lua` — real gameplay rather than an
imagined sample — and wrote down the whole surface it needed. That list is the
acceptance criteria, and it is now met: authored properties, lifecycle,
transform, sprite, input, and `print`.

Both of its values are now the language's own: `vec2(x, y)` is `Vec2`, and
`sprite:set_tint([f32; 4])` is `this.sprite.tint = Color(r, g, b, a)`, while
`this.sprite.tint.r` still reaches one channel as `position.x` reaches one
component.

## Failure is per script

`Scripts::advance` returns every failure rather than stopping at the first. One
script must not be able to silence the others: in the editor that would mean a
typo in one object freezing every other, and the author looking for the wrong
bug entirely.

A script that failed this frame **keeps its instance**. A runtime error is not a
reason to discard the state an author is trying to inspect, and restarting it
would hide the failure behind a fresh `start` sixty times a second.

## No I/O, anywhere in the crate

`sindri-decay` never opens a `.decay` file. It has no more business doing so
than `sindri-core` does, and staying out of it is what lets every test in the
crate run with no filesystem and no browser.

Sources arrive through `ScriptSources`, filled by whoever owns the asset
pipeline — exactly as textures arrive at `sindri-scene` through
`TextureBindings` rather than being loaded there. In the editor that is
`editor/src/scripts.rs`, which drives `AssetLoader<TextAssetDecoder>` and gets
hot reload out of the same `AssetWatch` the textures use: a changed file is
fetched again, and `Scripts` recompiles when the text it holds stops matching.

## Play, and putting the world back

Scripts write to the world, which sprite animation never did. So the editor
snapshots the world when Play is pressed and restores it on Stop.

The snapshot rather than the authored document, deliberately: a scene edited and
then played must come back to the edit, or pressing Play would quietly discard
unsaved work. Undo history is left alone — a script moving something is not an
action the author took, so it was never on the history, and putting the world
back does not change what undo means.

## Known gaps

- Despawning and other script writes are not routed through `WorldCommand` and
  therefore do not produce undo entries. Editor play mode restores its snapshot
  on Stop instead.
- Decay's only numeric type is spelled `f32` and holds an `f64`; `WorldHost` is
  the one place the two meet and the one place that narrows.
- The script instance is created on first sight and lost when the world is
  reloaded. There is no state migration across a hot reload of the source: a
  changed file recompiles, and the running instance keeps its fields.



### Gameplay tweens

Gameplay tweening is a managed Decay API. Weave's UI animation surface remains
CSS-inspired (`transition`, and future `@keyframes`); the two share named easing
math in `sindri-core`, not authoring syntax.

| Call | Return | Meaning |
| --- | --- | --- |
| `Tween.number(from, to, duration, easing)` | `NumberTween` | Start a number tween |
| `Tween.vec2(from, to, duration, easing)` | `Vec2Tween` | Start a 2D vector tween |
| `Tween.vec3(from, to, duration, easing)` | `Vec3Tween` | Start a 3D vector tween |
| `Tween.color(from, to, duration, easing)` | `ColorTween` | Start an RGBA colour tween |
| `Tween.number_value(tween)` | `f32` | Read a number tween's current value |
| `Tween.vec2_value(tween)` | `Vec2` | Read a 2D vector tween's current value |
| `Tween.vec3_value(tween)` | `Vec3` | Read a 3D vector tween's current value |
| `Tween.color_value(tween)` | `Color` | Read a colour tween's current value |
| `Tween.progress(tween)` | `f32` | Linear fraction of the current play in [0, 1] |
| `Tween.is_done(tween)` | `bool` | Naturally completed; cancellation is not completion |
| `Tween.is_paused(tween)` | `bool` | Paused playback |
| `Tween.is_cancelled(tween)` | `bool` | Cancelled playback |
| `Tween.pause(tween)` | unit | Hold the current value |
| `Tween.resume(tween)` | unit | Resume paused playback; cancellation remains |
| `Tween.cancel(tween)` | unit | Stop and retain the current value |
| `Tween.restart(tween)` | unit | Replay the original endpoints; clear pause/cancellation |
| `Tween.dispose(tween)` | unit | Release the handle and invalidate all aliases |
| `Tween.set_delay(tween, seconds)` | unit | Hold the start value before playing |
| `Tween.set_loops(tween, count)` | unit | Play a whole number of times; 0 plays for ever |
| `Tween.set_yoyo(tween, on)` | unit | Every other play runs back to the start |
| `Tween.after(tween, previous)` | unit | Hold still until `previous` has finished: a sequence |

Factories take two endpoints of the indicated type, a duration in seconds, and
one of `"linear"`, `"ease"`, `"ease-in"`, `"ease-out"`, `"ease-in-out"`.
Wrong types are compile errors. Unknown curves, non-finite endpoints, negative
or non-finite durations, null and disposed handles produce named host errors.
Zero duration is complete immediately, returning the exact target.

A tween starts playing at its initial value. Its owner is the creating script;
time advances once before each **subsequent** owner update, using the script
runner's delta. A tween created during an update or delivered message starts at
time zero and first advances on the next owner update. Reading it repeatedly
does not advance it. Pause/resume affect future advancement; an early return in
`update` does not pause it. Owners that do not tick do not advance tweens.
Removal of a script instance (including disable/removal), script replacement,
and `Scripts::clear` release its handles. Completed and cancelled handles remain
readable until disposed or their owner is removed. Copies alias the same tween.
At most **8192 retained handles** exist per script runner; creation beyond that
fails rather than evicting a handle. Dispose replaced/unused tweens.

```decay
script Fade {
    var tint: ColorTween = null;
    fn start() {
        this.tint = Tween.color(Color(1.0, 1.0, 1.0, 1.0),
                                Color(1.0, 1.0, 1.0, 0.0), 0.3, "ease-out");
    }
    fn update(dt: f32) {
        this.sprite.tint = Tween.color_value(this.tint);
        if Tween.is_done(this.tint) { World.despawn(this.entity); }
    }
}
```

A tween composes on its handle, usually as it is made. `set_delay` holds the
starting value for some seconds first. `set_loops` plays it a whole number of
times, and 0 plays it for ever, which is never done. `set_yoyo` makes every
other play run back from `to` to `from`, so a pulse is one tween rather than a
`sin` of a clock. `after` holds a tween still until another has finished and
starts it on the next step, so a sequence is each tween after the one before;
one whose predecessor is disposed goes on without it, and a tween cannot wait
for itself. `progress` is the current play's fraction, running back down on a
yoyo's return, and `restart` replays the delay too. Orbital Last Stand's
pickup pulses with a looping yoyo that starts after its appearance tween; the
tween example chains a move, a pause and a return, and bobs with a yoyo.

```decay
this.grow = Tween.number(0.0, 1.0, 0.2, "ease-out");
this.glow = Tween.number(0.2, 0.6, 0.5, "ease-in-out");
Tween.set_loops(this.glow, 0.0);
Tween.set_yoyo(this.glow, true);
Tween.after(this.glow, this.grow);
```

Vectors keep the coordinate space of their endpoints. Tweening world positions
means supplying world positions and assigning `transform.world_position`;
local positions go to `transform.position`. The ordinary setters still enforce
hierarchy conversion and Z locks. Colours interpolate the supplied four channels,
including alpha, without extra gamma conversion or channel clamping.

For a smooth replacement, read the displayed value, cancel/dispose the old
handle, then create a new tween from that value to the new target. The new
handle starts at that value with a new duration; `restart` instead replays the
original endpoints. Completion is polled, so a game can use its existing typed
messages/events for follow-up actions. This slice does not add property-path
binding, timelines, sequences, callbacks, loop/yoyo modes or CSS keyframes.
Orbital's `powerup.decay` uses a managed vector tween for its pickup appearance.
