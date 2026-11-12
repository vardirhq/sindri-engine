# Object parts and the object workspace

Status: proposal. Steps 1 to 3 below are built: sprite `size` and `offset`,
light `offset`, and the mobile object workspace for a scene object or a
prefab. The rest is not.

## The problem

A thing in a Sindri scene is often several objects. Before this proposal,
Lamplighter's Wick was eight:

```text
wick                   body: collider, rigid body, script
├ wick-sprite          the art, scaled 3 × 2.75
└ wick-lantern         pivot the script swings about Wick
  ├ wick-lantern-sprite
  ├ wick-embers        particles
  ├ wick-glow          particles
  ├ wick-light         light
  └ wick-halo          light
```

Only one of those children is there because the game needs it: the lantern
moves on its own. The rest are there because of two engine rules:

1. **A part could not be placed on its object.** A sprite had no size or
   offset, so art sized differently from its body needed an object whose
   scale sized it; a light could not sit anywhere but its object's origin.
2. **An object holds one component of each kind.** Components are a map from
   type name to payload, so two lights or two emitters need two objects.

On a phone every extra object is a row in Layers, a thing a tap can land on,
and a transform to get wrong. The first device pass found exactly that: the
lamps' and Wick's parts covered them with markers and got dragged instead of
them. The goal, on mobile above all, is fewer layers: an object is one thing,
and its parts are edited inside it.

This is a capability found by making Lamplighter a good game to edit on a
phone, and it is added generally.

## What is built

`sindri.sprite` has `size` (default `[1, 1]`) and `offset` (default
`[0, 0]`), and `sindri.light2d` has `offset`, all in the object's own space:
they move, turn and scale with it. Extraction, picking, selection boxes, cast
shadows and `Lighting.level` read them. Existing scenes draw as before.

Wick's and the lantern's art moved onto them, so Wick is six objects:

```text
wick            body, collider, script, sprite (size 3 × 2.75), shadow, animation
└ wick-lantern  sprite (size 0.6 × 0.8)
  ├ wick-embers, wick-glow   particles
  └ wick-light, wick-halo    lights
```

Particles already had `offset` and `shape_size`, colliders already hold a
list of pieces with their own offset and rotation, and the contact shadow has
`contact_offset`, so every 2D part a lamp or a character needs can now be
placed on its object. What remains is rule 2 and the editor.

## Several parts of one kind

The remaining children of the lantern exist because it has two lights and two
emitters. Three shapes are possible:

| Shape | Scene payload | What breaks |
| --- | --- | --- |
| A list inside each component (`"sindri.light2d": {"lights": [...]}`) | Changes every existing light's payload | Every scene, every Decay field access (`this.beam.light2d.intensity`), every inspector |
| **Named instances in the component map** (`"sindri.light2d"`, `"sindri.light2d#halo"`) | Existing payloads unchanged; a second part is a second key | Queries return each instance; schema lookup reads the type before `#` |
| Children the editor shows as parts | Unchanged | Nothing, and nothing gets simpler: the objects are still there |

**Recommended: named instances.** The unnamed key stays the object's first
part, so every scene, script and test keeps working. A second light is
`sindri.light2d#halo`. A file from a newer editor still opens in an older one,
which keeps an unknown component key as an unknown payload. What changes:

- `ComponentSchemaRegistry` resolves `type#name` to `type`'s schema, and
  `query::<T>` yields every instance, each with its name.
- Extraction draws each instance; nothing about a light or an emitter assumes
  it is the only one on its object.
- Decay keeps `this.entity.light2d` for the unnamed part and needs a way to
  name another. The syntax is an open question for `docs/decay-direction.md`,
  and nothing is added until it is settled there.
- Saving stays canonical: keys sort, so `sindri.light2d#halo` follows
  `sindri.light2d`.

With that, the lantern is one object holding its art, two emitters and two
lights, and Wick is two: the body and the lantern that moves.

### Colliders

A collider already holds several pieces, each with its own shape, offset,
rotation, sensor flag, layers, friction and restitution, so a solid body and a
trigger zone on one object already work. What is missing is which piece:
contacts and sensor events name the object, so a script cannot tell "something
entered my sensor" from "something bumped my body", and a game that needs to
tell them apart still splits them into two objects. Pieces gain an optional
`name`, and contacts and sensor events carry the names of the pieces involved.

## The object workspace

The mobile collider workspace already opens one object full height, shows its
sprite and lets its collider pieces be edited without the scene around them.
It becomes the editor for every part of an object.

**It is a prefab editor.** Opening an object in the workspace is what desktop's
*Edit prefab* does for a prefab: the thing is shown on its own and edited in
its own coordinates. The workspace opens either:

- **an object in the scene**: edits are ordinary commands in the scene's
  history, saved with the scene, and undo steps through them as through any
  other edit; or
- **a prefab file**, from the Add sheet or from an instance's inspector: edits
  save through `sindri-authoring`'s prefab file path, and instances in the open
  scene follow it, as on desktop.

In it:

- The canvas shows the object and its family alone, framed to fit, on a plain
  ground with the origin marked.
- Every part draws its own handle: the sprite's frame (drag to move it, corner
  to size it), each collider piece (as now), a light's position and radius
  ring, an emitter's shape, the contact shadow's ellipse.
- A part list along the bottom replaces Layers inside the object. Tap a part to
  select it, and its fields open beneath the canvas; *Add part* uses the
  existing component picker.
- Children that are really separate moving things, like Wick's lantern, are
  listed as children and open in the workspace themselves.

The scene canvas then only ever selects whole objects, and Layers lists
things, not parts.

## Order of work

1. **Done:** sprite `size` and `offset`, light `offset`; Lamplighter's Wick and
   lantern carry their art.
2. **Done:** the object workspace on mobile for a scene object: the collider
   workspace grown to every placeable part, with handles for the art, the
   light's reach, a box emitter and the shadow, and a part list
   (`ObjectWorkspace.kt`, `ObjectParts.kt`; `mobile/first-slice.md` lists
   what it does not do yet).
3. **Done:** open a prefab in the workspace, from an instance's inspector;
   instances in the open scene follow through `sindri_authoring::prefab::follow`,
   keeping their overrides (`mobile/native/src/prefab_edit.rs`). Only the
   prefab's root is shown.
4. Named component instances in the engine and both editors; Decay access once
   `docs/decay-direction.md` settles the syntax. Collapse Lamplighter's lantern.
5. Collider piece names, reported by contacts and sensor events.
6. The same part handles in desktop's Scene view, so both editors agree.

## Open questions

- Is `type#name` the right key, or should names live inside the payload and
  the map hold a list per type? The key keeps payloads and old files unchanged;
  the list keeps one key per type.
- How does Decay address a named part?
- Should the workspace allow a part to be turned into a child object (and back)
  for the rare part that must move on its own?
