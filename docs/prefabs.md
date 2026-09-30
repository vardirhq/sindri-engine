# Prefabs

An authored reusable entity definition, and the thing `World.spawn` creates.

## Why there is one

A script could find another entity, reach through it, check whether it still
existed, and remove it. It could not make one. That was not an oversight —
`docs/scripting.md` recorded it as a real dependency: creating an entity means
saying *what* to create, and the engine had nothing to say it with.

So a game could only ever have the entities its scene was authored with. Every
enemy, bullet, pickup, and effect had to be placed by hand before the run began.
`docs/orbital-last-stand-audit.md` found this first among the reasons a
real-time action game could not be authored through the editor and Decay.

## What one is

A scene fragment with exactly one root.

```json
{
  "format_version": 1,
  "metadata": { "name": "Bullet" },
  "entities": [
    {
      "id": "bullet",
      "name": "Bullet",
      "transform_3d": { "position": [0.0, 0.0, 0.0] },
      "components": {
        "sindri.script": { "source": "scripts/bullet.decay", "script": "Bullet" },
        "sindri.sprite": { "texture": "textures/bullet.png", "layer": 3 }
      }
    }
  ]
}
```

That is a `SceneEntity`, unchanged, and deliberately: a prefab reuses the
scene's entity shape, its component payloads, its identities, its canonical
serialization, and its validation. **It is not a second document format.** A
separate shape for "a subtree of entities" would be a copy of the scene format
that drifts from it, and a prefab that could not hold a component a scene can
hold would be a trap discovered late.

The rules a scene's entities obey are in `scene::graph`, shared by both, rather
than written twice — the second copy of a validator is the one that stops
matching the first.

### The one rule a prefab adds

**Exactly one root.** A prefab with several is refused when it is read.
`World.spawn` answers with a reference, and a document with two roots would
make it answer with one of them and leave the other attached to nothing an
author can name.

### Its own format version

`PREFAB_FORMAT_VERSION` is not `SCENE_FORMAT_VERSION`. The two documents share
an entity shape and will share its migrations, but they are separate files with
separate histories: a prefab gaining a document-level field is not a reason to
step every scene in a project. A version this runtime does not understand is
refused rather than guessed at, exactly as a scene's is.

## Spawning one

From Decay, `World.spawn(prefab)` creates a root-level instance.
`World.spawn_child(prefab, parent)` creates the same prefab and attaches its root
to an existing entity before the spawned script can start. The prefab root's
authored transform remains local to that parent, and the prefab's own descendants
remain under its root. A stale parent is refused before anything is created.

The full contract — that a prefab reference is a typed `Prefab` rather than text,
how overrides work, when a spawned script's `start` runs, and what the bounds are
— is in `docs/scripting.md`, because it is a statement about the scripting
surface.

From Rust, `World::spawn_prefab` returns a `SpawnedPrefab`: the root, every
entity created, and which authored identity each became.
`World::spawn_prefab_from` does the same for a prefab with others nested in it,
given the library to make them from; a nested entity is keyed by its path,
`loot/sparkle`. `World.spawn` in Decay does this against the project's
prefabs, and counts what the nesting expands to against the spawn limit.

This is not how a scene places one. A spawn makes the prefab's entities once,
for a world that will not be saved; a scene's instance keeps the reference —
see [Placing one in a scene](#placing-one-in-a-scene).

### A spawn has no stable identity

Spawned entities carry no `source_id`. A prefab's identities name entities
*inside the prefab*; two instances carrying them would collide on every one, and
a scene saved with the collision would refuse to load.

`World::assign_missing_source_ids` remains how a runtime entity earns a stable
identity. That is a decision about persisting a world, not about spawning, and
keeping the two apart is why saving a world full of bullets is a thing you ask
for rather than a thing that happens.

### Editor-only state does not come along

A prefab's `editor` sections describe the prefab in the editor — what is folded,
what is selected. An instance in a running world has no use for them, and
carrying them would put a fold state on every bullet.

### Nothing is half-created

The document is validated before anything reaches the world. A prefab with
several roots, a missing parent, or a non-finite transform spawns nothing at
all rather than leaving a partial subtree behind.

## Loading one

`.prefab`, resolved against the scene's directory like every other asset,
and delivered by the same text pipeline the scripts use — a prefab is JSON, and
the pipeline has no reason to know more than that. A document that will not
parse is reported once, when it loads, rather than on the frame a script spawns
it.

Which prefabs a scene needs is answered by the *declared type* of each script's
exported fields: a field declared `Prefab` is a prefab reference, and one
declared `String` is text however much it looks like a path. That is what makes
a scene's prefabs loadable before the first frame instead of discovered when a
script spawns.

Because the declared types are not known until a source has compiled, the set is
asked for every frame rather than once at open — which is also what makes a
prefab authored a moment ago load a moment later.

## Placing one in a scene

A scene places a prefab as an **instance**: an entity that names the prefab
and says only what is its own.

```json
{
  "id": "coin-3",
  "name": "Coin 3",
  "parent": "pickups",
  "transform_3d": { "position": [31.5, 6.0, 0.2] },
  "prefab": {
    "source": "prefabs/coin.prefab",
    "overrides": {
      "coin": { "components": { "sindri.sprite": { "layer": 6 } } },
      "sparkle": { "disabled": true }
    }
  }
}
```

The instance keeps its own ID, parent, name, transform and switch — those are
the prefab root's, as placed. Everything it is made of is the prefab's, so an
instance carries no `components` of its own; a scene that gives it some is
refused, because two places saying what an instance is made of is one place
too many.

When the scene loads, the instance becomes the prefab's entities. Its root
keeps the instance's ID; every entity under it is `<instance>/<id in the
prefab>`, so ten coins never collide and a scene can hang an entity of its own
under a prefab's child by naming `coin-3/sparkle` as its parent.

This is the one way a prefab changes a scene after it was placed: the scene
holds the reference, so an edit to `coin.prefab` is an edit to every coin the
next time the scene is opened, and at once in an editor that has it open.
`games/platformer` places its ten coins this way.

### Overrides

`overrides` is keyed by the prefab's own entity IDs, root included. Each says
what that entity's instance changed:

- `name`, `transform_3d`, `disabled` — written whole when they differ.
- `components` — a JSON merge patch (RFC 7386) per component: an object merges
  field by field, `null` removes the component (or a field), and any other
  value replaces it. A component the prefab lacks is added whole. A list is
  replaced whole: an instance cannot change one tile of a prefab's tilemap
  without owning the list.

Overrides are never tracked as they are made. Saving works each one out as the
difference between what the instance's entities are and what the prefab says,
so setting a value back to the prefab's stops being an override without anyone
having to say so, and an instance standing exactly where its prefab's root
does writes no transform. A key the prefab no longer has is dropped the next
time the scene is saved.

An override can change what an entity carries but not whether it exists or
where it hangs, so an instance's structure is its prefab's. The editor refuses
to delete, move or rename an entity inside an instance, and a world in which
one was changed anyway fails to save, naming the instance
(`WorldError::InstanceReshaped`). Unpacking the instance is the way out.

### Nested prefabs

A prefab is made of the same entities a scene is, so a prefab may place an
instance of another. A chest prefab can hold a coin; placing the chest makes
both. An override reaches inside by path — `loot/sparkle` is the `sparkle` of
the chest's `loot` coin. A prefab that contains itself, directly or not, is
refused naming the loop, and nesting stops at `MAX_PREFAB_NESTING` (16).

This is nesting, not variants: a prefab cannot be "a coin, but gold". That
remains a deliberate anti-goal in `docs/parity.md` until a game proves it
needs one; an instance with overrides covers the case so far.

### Loading a scene with instances

Core does no I/O, so every host hands it a `PrefabLibrary` — anything that
answers "which prefab is this asset ID":

- `World::from_scene_with` and `World::add_scene_with` expand instances and
  give every entity they make a `PrefabLink` (the prefab, the entity's path in
  it, and whether it is the root). `World::to_scene_with` writes instances back
  as references. The editor loads this way, because it saves.
- `SceneDocument::expanded` returns the scene with every instance made into
  plain entities. A host that plays a scene and never saves it uses this once
  and then loads through the paths it already had. The browser host does.
- `World::from_scene` and `World::to_scene` keep working for a world with no
  instances, and name the missing prefab when given one rather than loading an
  instance as nothing.

The exporter reads every prefab a scene places, and every one nested in those,
before it walks the scene, and ships them as prefabs: a placed coin's sprite is
named in the coin's prefab, not in the scene.

## In the editor

- **Placing.** Choosing a prefab in the project browser opens its panel. *Add
  to scene* places an instance in the middle of the Scene view, at the prefab's
  own depth; *Place on a cell* puts one on a grid cell. Both make a linked
  instance as one undo step. A prefab outside the scene's folder cannot be
  placed, because the scene would have no asset ID to name it by.
- **Seeing it.** An entity of an instance shows *Instance of coin.prefab* at
  the top of the inspector, with what it overrides listed below and a way to
  revert each row.
- **Revert all** puts the instance back to exactly its prefab; **Apply** writes
  what it overrides into the prefab file and brings every instance up to date;
  **Unpack** keeps the entities and cuts the link. Apply writes the file at
  once: Undo returns this scene's instances, not the file.
- **Making one.** *Make prefab* on a hierarchy row writes that entity and
  everything under it to `prefabs/<name>.prefab` beside the scene — an
  instance inside it stays one, nested — and replaces it with an instance.
- **Editing one.** *Edit prefab* opens the prefab as the document being edited,
  in the same panels a scene uses. Its asset IDs resolve against the scene's
  folder, as they do when it is placed. Saving writes a prefab, and refuses a
  second root. *Back to …* returns to the scene it was opened from.
- **Following it.** When the file watcher reads a changed prefab, or Apply
  writes one, every instance in the open scene is written down against the
  prefab it was made from and reconciled with what it now expands to. That is
  done with commands, so overrides survive, the entities keep their handles and
  the selection, and the update is one undo step.
- **Copying and renaming.** Duplicating an instance makes another instance;
  duplicating part of one makes plain entities. Renaming an instance's ID
  renames what is under it.
- **Script fields.** A script's `@export` `Prefab` field, and any component
  field that names a prefab, is picked from the project's prefabs.

## What is not here yet

- **No removing a prefab's entity from one instance.** Switching it off
  (`"disabled": true`) is how an instance does without one today.
- **No per-element list overrides.** A list component field is overridden
  whole.
- **Spawned instances are not linked.** `World.spawn` makes entities with no
  `PrefabLink`, as it makes them with no stable ID: nothing spawned is saved.
- **Apply is not undone with the scene.** It writes the prefab file.
