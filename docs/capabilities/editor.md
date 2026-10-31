<!-- Generated from docs/capabilities/editor.toml by `cargo run -p sindri-capabilities -- --write`. Edit the source, not this file. -->

# The editor

The editor renders the real runtime frame through eframe's shared WGPU device —
it is not a mock, and it does not create a second device. An action that fails
says so: a sticky notice from the last thing you did is kept apart from the
per-frame render result. `docs/editor-audit.md` and
`docs/editor-authoring-audit.md` are the control-by-control sweeps behind this
page.

Every entry here was used, not inferred. Entries marked as having no automated
test were exercised by hand in the native editor; a test naming them would let
the generator check them like the rest.

Each entry links its tests and the projects that exercise it. Surface marks match the row in [parity](../parity.md) where one is named: ✅ works and is exercised · 🟡 a useful slice, with the gap named · ❌ absent · — not applicable.

| Capability | Engine | Editor | Decay | Proof |
| --- | :-: | :-: | :-: | :-: |
| **Projects and scenes** | | | | |
| [Opens a project, not only a scene](#projects) | — | ✅ | — | ✅ |
| [Shows a project's scenes as a board](#scene-board) | — | ✅ | — | ✅ |
| [Opens, saves, makes and forks scenes](#scene-files) | — | ✅ | — | ✅ |
| [Unsaved marker and confirmations](#unsaved-work) | — | ✅ | — | — |
| **Hierarchy and selection** | | | | |
| [Hierarchy tree with world and UI groups](#hierarchy) | — | ✅ | — | — |
| [Create, delete, rename, duplicate and reparent](#entity-verbs) | — | ✅ | — | — |
| [More than one entity at a time](#multi-select) | — | ✅ | — | — |
| [Sibling order, moved rather than renamed](#sibling-order) | — | ✅ | — | — |
| [Switching an entity off without deleting it](#active-switch) | ✅ | ✅ | — | — |
| [Copy and paste entities](#clipboard) | — | ✅ | — | — |
| [A right-click menu wherever one is expected](#context-menus) | — | ✅ | — | — |
| **Inspector** | | | | |
| [Name, transform, stable ID and the scene itself](#inspector-basics) | — | ✅ | — | — |
| [Editing any component's fields](#component-fields) | — | ✅ | — | — |
| [Controls that know what a field means](#field-meanings) | — | ✅ | — | — |
| [When an inspector edit applies](#apply-modes) | — | ✅ | — | — |
| [Adding and removing components](#add-components) | — | ✅ | — | — |
| [`@export` properties discovered and authored](#script-exports) | ✅ | ✅ | ✅ | ✅ |
| [Text authoring](#text-authoring) | — | ✅ | — | — |
| [Creates and edits reusable profiles](#profile-authoring) | — | ✅ | — | — |
| **Scene view** | | | | |
| [Viewport camera, axis indicator and 2D view](#viewport-camera) | — | ✅ | — | — |
| [Everything the Scene view draws can be clicked](#viewport-picking) | — | ✅ | — | — |
| [Select, Move, Rotate and Scale tools with snapping](#transform-tools) | — | ✅ | — | — |
| [Gizmos for what simulates without drawing](#simulation-gizmos) | — | ✅ | — | — |
| [Scene and Game views](#scene-and-game-views) | — | ✅ | — | — |
| **Play** | | | | |
| [Play runs the session a shipped game runs](#play-runs-the-build) | ✅ | ✅ | — | ✅ |
| [A scene can be edited while it plays](#live-editing) | — | ✅ | — | — |
| [A run can be scrubbed and stepped](#run-scrubbing) | ✅ | ✅ | — | — |
| [Plays a scene's audio in Play, and mixes it for the author](#play-audio) | — | ✅ | — | ✅ |
| [Timeline / cutscenes](#timeline) | ✅ | ✅ | ✅ | ✅ |
| [Profiles the editor's frame, and benchmarks against the shipped host](#profiler) | — | ✅ | — | — |
| **Workspace** | | | | |
| [A workspace the user arranges](#workspace) | — | ✅ | — | — |
| [One field that finds anything](#command-palette) | — | ✅ | — | — |
| [Console and History](#console-and-history) | — | ✅ | — | — |
| [Project browser: import, rename, copy, delete, preview](#project-browser) | — | ✅ | — | ✅ |
| **Local assistant** | | | | |
| [The local assistant sets itself up inside the editor](#assistant-setup) | — | ✅ | — | — |
| [Compile errors and a proposed fix for a previewed script](#assistant-repair) | — | ✅ | ✅ | — |
| **Not yet** | | | | |
| [Editing several scenes side by side](#editor-multi-scene) | — | ❌ | — | — |
| [Build and export controls](#editor-export) | — | ❌ | — | — |
| [Versioned editor protocol](#editor-protocol) | — | ❌ | — | — |
| [In-editor script editing](#in-editor-scripts) | — | ❌ | — | — |
| [Reloading an edited scene file](#scene-file-reload) | ❌ | ❌ | — | — |
| [Viewport authoring of polygon vertices](#polygon-handles) | — | ❌ | — | — |
| **3D foundation** | | | | |
| [Model asset field metadata and picker](#model-asset-picker) | ✅ | ✅ | — | 🟡 |

## Projects and scenes

<a id="projects"></a>

### Opens a project, not only a scene

A welcome window lists recent projects, makes new ones and opens folders; a scene opened anywhere finds its `sindri.toml`.

Engine — · Editor ✅ · Decay — · Proof ✅

**Tested by:** [`editor/src/project/tests.rs`](../../editor/src/project/tests.rs), [`editor/src/project/recent.rs`](../../editor/src/project/recent.rs), [`editor/src/project/launch.rs`](../../editor/src/project/launch.rs), [`editor/src/project/manifest/tests.rs`](../../editor/src/project/manifest/tests.rs), [`editor/src/native/welcome/create.rs`](../../editor/src/native/welcome/create.rs)

**Exercised in:** [`game`](../../game), [`games/orbital-baked`](../../games/orbital-baked)

**See:** [`docs/project-format.md`](../../docs/project-format.md)

<details markdown="1">
<summary>Evidence</summary>

A project is a directory holding `sindri.toml`, which carries a format version,
the project's name, and the scene it opens. The welcome window is its own
window and the editor's is hidden behind it until a project is open: it lists
the twelve most recently opened projects, marks one that has moved or been
deleted as missing rather than dropping it, makes a project — a manifest, a
scene, and the folders assets resolve from — and opens a folder that already is
one. Shipped projects are listed as samples when the editor is run from the
repository. A scene opened from anywhere walks up to the nearest `sindri.toml`,
so the browser is rooted at the whole project and headed with the project's own
name. What it *lists* is the directory asset references resolve against, which
is the open scene's own: a project's Cargo manifest and `src/` are part of the
project and are not files a component can name, and the rest of the project is
a switch in the browser's toolbar away, remembered between launches and drawn
only where the two listings differ. A launch honours the command line first,
the last project when the user asked for that, and the welcome window
otherwise. **Set as main scene** on a scene row nominates what the project
opens on, and a scene made in a project that nominates nothing claims the empty
place rather than leaving the project opening on nothing.

</details>

<a id="scene-board"></a>

### Shows a project's scenes as a board

The Scenes panel draws a card per scene with arrows for the `Scene.go` and `Scene.restart` doors between them, and edits the manifest's scene list.

Engine — · Editor ✅ · Decay — · Proof ✅

**Tested by:** [`editor/src/project/scene_board.rs`](../../editor/src/project/scene_board.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

**Limits:**

- A door named at run time (`Scene.go(next)`) is not drawn on the board.

<details markdown="1">
<summary>Evidence</summary>

The Scenes panel (a tab beside the Scene view, or beside the Game view in the
docked preset, and in the View menu) has a card for the main scene and each
scene in `[project] scenes`, with the last frame either view drew of it, or its
entity count until it has been opened. Arrows are the doors between scenes:
literal `Scene.go("…")` and `Scene.restart("…")` calls in the scripts a scene's
entities and placed prefabs run. A door to a scene the project does not carry is marked on its
card, with a menu entry to add that scene when the file is in the project.
Clicking a card opens its scene (asking first about unsaved work); **Add
scene**, **Remove from project**, **Move earlier**/**later** and **Set as main
scene** edit the manifest, which is written in place so `[web.splash]` and
comments survive. Orbital Last Stand shows two cards with a door each way, and
every shipped project's board is checked to have no missing scene and no door
to nowhere.

</details>

<a id="scene-files"></a>

### Opens, saves, makes and forks scenes

Open, save canonically, reload, discard, New scene and Save as, keeping unknown components through a save.

Engine — · Editor ✅ · Decay — · Proof ✅

**Tested by:** [`editor/src/scene_file.rs`](../../editor/src/scene_file.rs), [`editor/src/native/tests/scene.rs`](../../editor/src/native/tests/scene.rs), [`editor/src/native/tests/shortcuts.rs`](../../editor/src/native/tests/shortcuts.rs), [`editor/tests/fixture_scene.rs`](../../editor/tests/fixture_scene.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

<details markdown="1">
<summary>Evidence</summary>

Opens a scene from a command-line argument or **File → Open scene**, saves it
back canonically, reloads from disk, and discards changes — including a scene
carrying components it has never heard of, which it keeps through a save and
lists in the inspector. Reopens the scene the editor was last left in,
overridden by a path on the command line, with the file and its unsaved state
named in the window title.

New scene (Ctrl+N) asks where the file goes, writes it, and opens it through
the ordinary path; it holds one world camera, because a scene with none renders
a black Game view, and it is named from its file. Save scene as…
(Ctrl+Shift+S) is offered whether or not the scene has a file behind it, and
the project beside the scene, the remembered scene, the textures and the
scripts all move with it. A save box takes a name rather than an extension, so
`level` is written as `level.scene`.

</details>

<a id="unsaved-work"></a>

### Unsaved marker and confirmations

Unsaved means the world and file differ, and anything that would throw unsaved work away asks first.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/native/tests/scene.rs`](../../editor/src/native/tests/scene.rs)

<details markdown="1">
<summary>Evidence</summary>

An unsaved marker means the world and the file differ, so undoing back to what
was saved reads as saved again. A confirmation comes before anything that
would throw unsaved work away — opening another scene, reloading from disk,
discarding changes, and closing the window — each naming what it is about to
do and offering to save instead.

</details>

## Hierarchy and selection

<a id="hierarchy"></a>

### Hierarchy tree with world and UI groups

A live GameObject tree with folding, search, and world and UI entities listed apart by what they carry.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/native/tests/hierarchy.rs`](../../editor/src/native/tests/hierarchy.rs), [`editor/src/native/tests/hierarchy_input.rs`](../../editor/src/native/tests/hierarchy_input.rs), [`editor/src/space.rs`](../../editor/src/space.rs)

<details markdown="1">
<summary>Evidence</summary>

The hierarchy shows live runtime state as a Unity-style GameObject tree: every
entity may own children, child-bearing rows fold with state remembered across
launches, search retains and temporarily opens each match's ancestor path,
selection works anywhere on a row, and empty space or Escape clears it.

World objects and UI objects are listed apart, under a World group and a UI
group, with icons of their own. Which group a top-level entity is in is read
from what it carries — a `sindri.ui.*` component means the viewport — so
nothing has to be kept in step by hand and no entity can claim a space it is
not drawn in. A group holding only UI elements is listed with the UI. Create
GameObject makes an empty object or a UI Image directly, and the inspector
says which space the selected entity is in.

</details>

<a id="entity-verbs"></a>

### Create, delete, rename, duplicate and reparent

Every per-entity verb on its row menu and a key, each one undo step; deleting and undoing brings an entity back at the same handle.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/native/tests/hierarchy.rs`](../../editor/src/native/tests/hierarchy.rs), [`editor/src/native/tests/hierarchy_input.rs`](../../editor/src/native/tests/hierarchy_input.rs), [`editor/src/native/tests/shortcuts.rs`](../../editor/src/native/tests/shortcuts.rs), [`editor/src/native/editing/duplicate/reference_tests.rs`](../../editor/src/native/editing/duplicate/reference_tests.rs)

<details markdown="1">
<summary>Evidence</summary>

Rename, Duplicate, Create child, Frame in the Scene view, and Delete are on an
entity's own right-click menu, each also on a key — F2, Ctrl+D, F, and Delete
or Backspace. Rename happens in the row itself, focused as it appears,
committed with Enter and abandoned with Escape; a double click starts it.
Duplicate copies the whole subtree beside the original as a sibling, gives each
copy a stable ID nothing else is using, and undoes in one step.

Creating empty root or child GameObjects assigns a stable scene ID immediately,
and creating a child opens its parent. Deleting takes the whole subtree, and
**undo brings it back at the same handle** — so the selection and every
earlier edit in the history keep pointing at what they named. That works
because the history undoes in order: reaching a delete means everything after
it is already undone, so the slot it freed is free again.

Reparenting works through a **Parent** menu or by dragging onto another
GameObject or the World root. Both ask the world's cycle check before offering
or accepting a move; drag targets show whether they are legal, successful drops
select the moved entity and open its new parent, and the move is one undo step.

</details>

<a id="multi-select"></a>

### More than one entity at a time

Ctrl- and Shift-click selection in the hierarchy and Scene view; delete, duplicate, reparent and handles act on all of it.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/selection/tests.rs`](../../editor/src/selection/tests.rs), [`editor/src/native/tests/hierarchy_input.rs`](../../editor/src/native/tests/hierarchy_input.rs)

<details markdown="1">
<summary>Evidence</summary>

Ctrl-click adds and removes, Shift-click takes the range between two rows as
the hierarchy is drawing them, and Ctrl-click does the same in the Scene view.
Delete, Duplicate and a drag to a new parent then take the whole selection in
one undo step, and dragging the handles moves, turns or scales every selected
entity by what the one under the pointer was moved, turned or scaled by — from
each one's own start, so a row stays a row. The inspector stays on the last
entity pointed at and says how many the verbs outside it would take; the rest
of the selection wears a ring in the Scene view where its own handles would
have been.

</details>

<a id="sibling-order"></a>

### Sibling order, moved rather than renamed

Move up and down on a row's menu and Alt+arrows, stored in the editor-only section rather than the scene proper.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/ordering/tests.rs`](../../editor/src/ordering/tests.rs)

<details markdown="1">
<summary>Evidence</summary>

The order lives in the entity's editor-only section of the file rather than in
the scene proper, because document order is canonical and meaningless by
design and draw order is render layers and depths — so where a row sits in a
panel is a fact about the panel. A scene nobody has reordered still lists
alphabetically by stable ID.

</details>

<a id="active-switch"></a>

### Switching an entity off without deleting it

An Active switch takes an entity and everything under it out of the scene while it stays in the world and the file.

Engine ✅ · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/inspector/entities_tests.rs`](../../editor/src/inspector/entities_tests.rs)

<details markdown="1">
<summary>Evidence</summary>

Off means it takes no part in the scene — not drawn, not stepped, not scripted,
not picked — and neither does anything under it, while it stays in the world
and in the file. An Active switch on the inspector, Disable and Enable on a
hierarchy row's menu taking the whole selection, and a struck-through row for
anything switched off. The flag is per entity and never written down through a
subtree, so re-enabling a parent brings back exactly the children that were on.

</details>

<a id="clipboard"></a>

### Copy and paste entities

Copied subtrees are kept as they were, and paste at the top level, under a row, or at a clicked point.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/native/tests/clipboard.rs`](../../editor/src/native/tests/clipboard.rs), [`editor/src/native/tests/instances.rs`](../../editor/src/native/tests/instances.rs)

**Limits:**

- Copies are kept by the editor, not the system clipboard, so they do not paste into another editor window.

<details markdown="1">
<summary>Evidence</summary>

Copy (a row's menu, or Ctrl+C with the hierarchy in hand) keeps the copied
subtrees as they were, so a copy pastes whole after the original was changed or
deleted; Paste (Ctrl+V, the empty space, or the Scene view) lands at the top
level, "Paste as child" under a row, and "Paste here" at the point clicked. A
row's menu also moves an entity out to the top level.

</details>

<a id="context-menus"></a>

### A right-click menu wherever one is expected

Hierarchy, project browser, component headings, fields, the Scene view and console lines each have their own menu.

Engine — · Editor ✅ · Decay — · Proof —

> ⚠️ **No automated test names this.** Exercised by hand; the menus' verbs are tested individually but no test opens the menus themselves.

<details markdown="1">
<summary>Evidence</summary>

The hierarchy's empty space creates an empty, a UI image or a light, and
pastes; the project browser's makes a folder, script, profile, physics material
or block set in the folder being looked at, imports, and refreshes; a
component's heading copies its values, pastes them onto another component of
the same kind, resets it to its blank and removes it; a field's name copies its
value and resets it to the schema's; the Scene view frames the selection or
everything and creates or pastes at the point clicked; a console line copies,
selects the entity it names, and clears. A project row has a menu of its own
for what the browser can already do, plus the asset path a component field
wants. Component order is not authored, so there is no Move up or down for a
component.

</details>

## Inspector

<a id="inspector-basics"></a>

### Name, transform, stable ID and the scene itself

Edits name, full transform and Z lock, shows and renames stable IDs, and shows the scene when nothing is selected.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/native/tests/inspector.rs`](../../editor/src/native/tests/inspector.rs), [`editor/src/native/inspector_panel/entity_tests.rs`](../../editor/src/native/inspector_panel/entity_tests.rs)

<details markdown="1">
<summary>Evidence</summary>

Inspector edits of name and the complete transform: position, Euler-degree
rotation backed by the stored quaternion, scale, and the Z lock, which takes
away movement off the current layer.

An entity's stable ID is shown and editable. It is what the file keys an entity
by, what a parent link names, what sibling order is derived from and what
`sindri.grid.occupant` points at. Renaming one carries every occupant that
names it along in the same undo step; one that is blank or already taken is
refused at the field rather than written.

With nothing selected the panel shows the scene's own name — a real field that
round-trips through a save — along with its file and how many entities it
holds. The rename is an ordinary undoable edit.

</details>

<a id="component-fields"></a>

### Editing any component's fields

Every component, known or not, is edited from its payload, and every edit is checked against its schema before it is written.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/native/tests/inspector.rs`](../../editor/src/native/tests/inspector.rs), [`editor/src/components/tests.rs`](../../editor/src/components/tests.rs)

<details markdown="1">
<summary>Evidence</summary>

The inspector is driven by the stored payload rather than by hand-written rows:
numbers get drags, booleans get checkboxes, text gets a field, and a short
numeric array gets a labelled row. A component the engine has never heard of is
editable too, which is what the preserve policy promises. Every edit goes
through `SetComponent`, so it undoes; every edit is checked against the
component's own schema first, so one that would stop it decoding is refused and
said aloud rather than written into a scene that then will not open.

</details>

<a id="field-meanings"></a>

### Controls that know what a field means

Choices are menus from the engine's own list, variants reshape their payload, and assets, colours and ranges get real controls.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/tests/inspector_field_meanings.rs`](../../editor/tests/inspector_field_meanings.rs), [`editor/src/inspector/choices.rs`](../../editor/src/inspector/choices.rs)

**See:** [`docs/component-schema-registry.md`](../../docs/component-schema-registry.md)

<details markdown="1">
<summary>Evidence</summary>

A component draws every field it has, filled out from the registry's own blank,
so two of one component show the same rows and a field nobody wrote down is
still visible at what it means; only a field actually changed is written back.
A value that is one of a few names is a menu — a camera's projection, a UI
anchor, a tilemap's projection, a rigid body's kind — taken from the engine's
own list. Some of those names decide what else the thing holding them consists
of: switching a camera to orthographic drops the vertical field of view, keeps
the near and far planes, and gains a vertical size, and switching a collider
piece from a box to a circle drops the half extents and gains a radius. Both
are one edit through one path, at whatever depth the field sits, and the
registry proved at startup that each variant decodes.

A field naming a project asset offers what the project holds while staying
typeable — spelled the way the open scene resolves it — and the texture list
includes every sprite cut from a loaded sheet (`blocks.png#stone-0`). A tint
opens a colour picker, with three channels or four, a declared range
(`FieldMeaning::Range`) is a control that cannot leave it, a fixed set of whole
numbers (`FieldMeaning::OneOf`, such as a shadow map size) is a menu,
whole-number vectors drag in whole steps, and a row that is only a readout says
on hover why it is one. `sindri.environment` and `sindri.voxel_world` declare
every colour, range, choice, and texture they hold, with a test that each
declared range is one validation accepts.

</details>

<a id="apply-modes"></a>

### When an inspector edit applies

Fields apply instantly, once settled, or only on Apply, so an expensive component regenerates once rather than per keystroke.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/inspector/held_tests.rs`](../../editor/src/inspector/held_tests.rs)

<details markdown="1">
<summary>Evidence</summary>

A component declares, per field or as a whole, how an edit to it reaches the
scene (`ApplyMode`, registered with `apply_when` beside its meanings and
checked against its fields the same way): `Instant`, as it is made, which is
every field that says nothing; `Settled`, once the person stops, after a short
pause in typing, on Enter or leaving the field, or on letting go of a drag; and
`Manual`, only when Apply in the component's header is pressed, with Revert
beside it. A held edit shows in the field as typed while the scene keeps what
it had, and another change to the same component meanwhile, an undo or a
script, is kept rather than overwritten. A voxel world is `Manual`.

</details>

<a id="add-components"></a>

### Adding and removing components

Add Component groups what the entity's space accepts and disables, with a reason, what cannot be added yet.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/native/tests/inspector_add.rs`](../../editor/src/native/tests/inspector_add.rs)

<details markdown="1">
<summary>Evidence</summary>

Add Component groups every type the entity's space accepts under Rendering, UI,
Physics, Grid or Behaviour — by an authored table rather than by the type
name's namespace — and a family holding one offer is listed at the top level.
Only the components that *place* something are exclusive to a space, so a UI
element can be given the script that drives it. It disables the ones that
cannot be added yet, each saying why: no font in the project, no sliced sprite
to build a clip from, no tilemap on this entity to navigate. Camera is among
them on a scene that already has a world camera. Text and sprite animation are
completed at the editor boundary: a project font gives UI Text a valid visible
default, while a Sprite component whose texture has named sheet sprites gives
Sprite Animation its first one-frame clip. Both are undoable.

</details>

<a id="script-exports"></a>

### `@export` properties discovered and authored

A script's exported fields are drawn from its declaration, without running it; only fields the author sets are written.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *`@export` properties discovered and authored*

**Tested by:** [`editor/src/scripts/tests.rs`](../../editor/src/scripts/tests.rs)

**Exercised in:** [`games/platformer`](../../games/platformer), [`games/orbital-baked`](../../games/orbital-baked)

<details markdown="1">
<summary>Evidence</summary>

The inspector draws the field's name, its type, and its default, without
running anything. A field the scene has not set shows its default and says so,
and setting one is what puts it in the scene — so a scene records an author's
choices rather than a copy of every default. This is the capability that
justified a statically typed language.

</details>

<a id="text-authoring"></a>

### Text authoring

UI text gets a multiline editor and a picker of the project's fonts; a missing font is called out, not replaced.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/typeface.rs`](../../editor/src/typeface.rs), [`editor/src/native/tests/inspector_add.rs`](../../editor/src/native/tests/inspector_add.rs)

<a id="profile-authoring"></a>

### Creates and edits reusable profiles

A project-row menu makes a `.profile`; a structured inspector edits its nested values, groups and lists.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/profile.rs`](../../editor/src/profile.rs)

<details markdown="1">
<summary>Evidence</summary>

Selecting a profile opens a structured inspector for its name, game-defined
type, nested values, groups, and lists. List entries can be appended,
duplicated, and removed, and Save writes canonical profile JSON. An exported
Decay `Profile` field uses a project asset picker rather than a free-form
string.

</details>

## Scene view

<a id="viewport-camera"></a>

### Viewport camera, axis indicator and 2D view

Orbit, pan, zoom, focus selection, an axis indicator drawn from the real camera, and a 2D toggle.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/native/camera/tests.rs`](../../editor/src/native/camera/tests.rs), [`editor/src/native/tests/viewport.rs`](../../editor/src/native/tests/viewport.rs)

<details markdown="1">
<summary>Evidence</summary>

A live viewport with orbit, pan, zoom, reset-to-authored-camera, and **Focus
selection** (F), which centres the view on the selected entity; the zoom spans
a factor of four hundred and moves proportionally, and the orbit cannot be
driven onto the pole. An axis indicator in the scene view's corner is drawn
from the same camera view the frame under it was drawn through, foreshortening
and reordering its arms as the camera turns.

Perspective, orthographic and **2D** toggle. 2D looks straight at the XY plane
without perspective, and any drag that would orbit pans instead. A scene whose
camera is orthographic and faces straight down -Z opens in 2D, framed on what
that camera frames.

</details>

<a id="viewport-picking"></a>

### Everything the Scene view draws can be clicked

Meshes, sprites, tile cells, cameras, UI and text are hit-tested from their own geometry through the exact rendered camera.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`crates/sindri-scene/src/picking/tests.rs`](../../crates/sindri-scene/src/picking/tests.rs), [`editor/src/occlusion/tests.rs`](../../editor/src/occlusion/tests.rs)

<details markdown="1">
<summary>Evidence</summary>

Meshes, world sprites, filled tilemap cells, authored cameras and UI images are
hit-tested from their own geometry; a string's box is measured by the text
renderer that draws it, at the resolution the view renders at. Picking inverts
the exact camera used to draw the frame, uses the renderer's unit quad and cube
dimensions, resolves transparent overlaps by layer and depth, honours opaque
occlusion, and clears selection on empty space. A fully transparent element is
skipped: a thing drawn as nothing is not a thing to click. Camera drags and
tile painting retain the pointer when active.

</details>

<a id="transform-tools"></a>

### Select, Move, Rotate and Scale tools with snapping

Q/W/E/R handles through the rendered camera, local or world, with snapping increments that can be set.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/gizmo/tests.rs`](../../editor/src/gizmo/tests.rs)

<details markdown="1">
<summary>Evidence</summary>

Handles use the exact rendered camera, support local/world orientation and
optional translation/angle/scale snapping, respect Z lock, and merge a whole
drag into one undoable command-history step. Snapping increments are set from a
right-click on the snap button and remembered across launches; a step of zero
means that one does not round.

</details>

<a id="simulation-gizmos"></a>

### Gizmos for what simulates without drawing

Joints, 3D colliders, character footing, effect reach and 2D lights are drawn, and selectable or draggable where it makes sense.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/native/collider_gizmo.rs`](../../editor/src/native/collider_gizmo.rs), [`editor/src/native/light2d_gizmo.rs`](../../editor/src/native/light2d_gizmo.rs), [`editor/src/native/shape_gizmo.rs`](../../editor/src/native/shape_gizmo.rs)

<details markdown="1">
<summary>Evidence</summary>

A 2D joint is drawn between its anchors as physics places them, with a rope's
reach, a hinge's allowed swing, a slider's travel and a spring's rest length,
and a click on its line selects it; a 3D collider's pieces as wireframes; a
selected character's footing, from its collider's lowest point: the steepest
slope it walks, how high it steps and how far down it snaps; and a selected
effect burst's reach, from its speed, spread, life and drag. Audio has no gizmo
because a source plays at one volume wherever it is.

</details>

<a id="scene-and-game-views"></a>

### Scene and Game views

The Game view renders through the authored camera with no editor chrome; both views are live at once.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/tests/fixture_scene.rs`](../../editor/tests/fixture_scene.rs)

<details markdown="1">
<summary>Evidence</summary>

Scene and Game views, the latter rendering through the authored camera with no
editor chrome painted over it — both live at once in the `Docked` arrangement.
A deterministic full-window screenshot is captured in CI.

</details>

## Play

<a id="play-runs-the-build"></a>

### Play runs the session a shipped game runs

Editor Play steps `sindri_runtime::Session` on a fixed clock, and a test requires the same world as a build after the same input.

Engine ✅ · Editor ✅ · Decay — · Proof ✅

**Tested by:** [`editor/tests/play_matches_the_build.rs`](../../editor/tests/play_matches_the_build.rs), [`editor/src/native/tests/runtime.rs`](../../editor/src/native/tests/runtime.rs), [`crates/sindri-platform/tests/game_loop.rs`](../../crates/sindri-platform/tests/game_loop.rs)

**Exercised in:** [`games/platformer`](../../games/platformer), [`games/scorchball`](../../games/scorchball), [`games/low-tide`](../../games/low-tide), [`games/orbital-baked`](../../games/orbital-baked)

<details markdown="1">
<summary>Evidence</summary>

Play runs **the session a shipped game runs**: `sindri_runtime::Session`, the
one the browser host, `sindri-player` and every game's test harness step. The
editor owns a `FixedStepClock`, advances it with the real frame delta, and
steps the session a whole number of times per frame, on a world styled by the
project's stylesheets as a build's is. `play_matches_the_build.rs` opens the
platformer, Scorchball, Low Tide and Orbital the way the editor does and the
way a build does, steps both with the same scripted input, and requires the
same world at the end.

**An edge belongs to exactly one fixed step.** A key going down is one event
and gameplay runs in the fixed step, so the edge is spent once a step has seen
it — a 30 Hz display driving a 60 Hz simulation earns two steps a frame and
would otherwise fire a button twice. It also survives a frame that earned no
step at all, which at 144 Hz is most of them, so a click is never dropped
before gameplay sees it. Accumulated pointer motion follows the same rule.

One button enters and leaves play mode, labelled Play or Stop by which of the
two pressing it will do; one icon pauses and resumes; and a word beside them
reads Editing, Playing, or Paused. Ctrl+P and Ctrl+Shift+P are the same two
actions. Stop puts back everything playing changed, from the world as it was
when Play was pressed rather than from the authored file, so pressing Play
never costs an unsaved edit. A separate **Discard changes** returns the world
to the file. A scene at rest shows its clips' first frames, so a broken clip is
reported without anyone pressing anything.

</details>

<a id="live-editing"></a>

### A scene can be edited while it plays

Edits act on the running world, are recorded against the run, and Stop offers to keep them as undoable entries.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/run_edits/tests.rs`](../../editor/src/run_edits/tests.rs)

**Limits:**

- Saving, new scenes, prefab files and undo/redo wait for Stop.

<details markdown="1">
<summary>Evidence</summary>

Inspector fields, the hierarchy's create, rename, duplicate and delete, gizmos,
and tile and block painting act on the running world at once, so a jump can be
tuned and tried in the same run. Each edit is recorded against the run rather
than the scene's history (`editor/src/run_edits.rs`). Stop restores the
snapshot from Play and lists them; the ones kept are applied to the restored
scene as ordinary undoable entries, by the component fields and transform parts
each one changed, so a value the game itself wrote while running does not come
back with them. An edit to something the run spawned is explained rather than
dropped.

</details>

<a id="run-scrubbing"></a>

### A run can be scrubbed and stepped

Play records two minutes of input and snapshots; the Timeline's Run strip replays to any step, and a held scene steps once.

Engine ✅ · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/tests/scrub_a_run.rs`](../../editor/tests/scrub_a_run.rs), [`editor/src/native/tests/runtime.rs`](../../editor/src/native/tests/runtime.rs)

<details markdown="1">
<summary>Evidence</summary>

Play records each fixed step's input, which steps the Game view was drawn
after, and a copy of the world and session every second, keeping the last two
minutes. The Timeline's Run strip takes the run to any recorded step: it
pauses, restores the copy at or before it and replays the recorded input, which
a restored session steps exactly as it did the first time. Playing on from
there carries on and lets the recorded future go.

**A held scene can be stepped once**, which is what a debugger's step button is
for. It runs the same body a played frame runs, so a scene stepped sixty times
is a scene that played for a second.

</details>

<a id="play-audio"></a>

### Plays a scene's audio in Play, and mixes it for the author

Play performs every audio request through the build's mixer; the Audio panel trims, mutes and solos buses without changing the game.

Engine — · Editor ✅ · Decay — · Proof ✅

**Tested by:** [`editor/src/play_audio/tests.rs`](../../editor/src/play_audio/tests.rs), [`editor/src/native/audio_view.rs`](../../editor/src/native/audio_view.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

**Limits:**

- No level meters, and buses are not yet declared by the project.

<details markdown="1">
<summary>Evidence</summary>

Play starts a scene's autoplay `sindri.audio.source`s and performs every
request its scripts make through `Audio`, through the same `AudioMixer` a build
uses; clips load from the scene's folder on first use and a missing one is a
console error. The Audio panel lists each bus the run names, master first, with
a trim fader, mute and solo, and every sound playing now with its bus, volume
and whether it loops, each stoppable. The monitor changes only what is heard:
`Audio.volume` still answers what the game set, and a new run starts from the
game's own mix while the monitor carries over. Pause holds every voice and Stop
ends them. Without an audio device Play carries on silently.

</details>

<a id="timeline"></a>

### Timeline / cutscenes

`sindri.sequence` drives tracks and cues for any entity, edited and previewed in the Timeline panel.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Timeline / cutscenes*

**Tested by:** [`editor/src/timeline/tests.rs`](../../editor/src/timeline/tests.rs), [`editor/src/native/timeline_view/tests.rs`](../../editor/src/native/timeline_view/tests.rs), [`crates/sindri-decay/tests/a_script_plays_a_sequence.rs`](../../crates/sindri-decay/tests/a_script_plays_a_sequence.rs), [`game/tests/the_sequence_demo_works.rs`](../../game/tests/the_sequence_demo_works.rs)

**Exercised in:** [`examples/sequence`](../../examples/sequence)

**Limits:**

- No curve editor, blending, or tracks beyond numbers and cues.

<details markdown="1">
<summary>Evidence</summary>

A `sindri.sequence` component holds named sequences, which one is `playing`,
and a `speed`, as a sprite animation holds clips. A sequence has a duration,
may loop, and carries tracks and cues. A track names a target (empty for the
entity, a path of child names such as `Ship/Flame`, or, with a leading slash, a
path from the scene's top level such as `/camera`), a property
(`position.x|y|z`, `rotation` in degrees about z, `scale`, `scale.x|y|z`, or a
component and a path into it such as `sindri.shape/stroke.3`), and keys in
time order, each with a CSS easing to the next. A cue names a moment and may
play a sound on a bus. `Sequences` advances every playing sequence after
scripts, holding each track's first key before it and last key after it,
wrapping a loop and reaching every cue crossed; a finished sequence lets go of
what it moved. The shared game session and the editor's Play advance them and
play cue sounds; Decay reads them through `Sequence.play`, `stop`, `restart`,
`is_finished`, `time`, `cued`, `playing` and `set_speed`.

The editor's Timeline panel shows the selected entity's sequences as a ruler, a
cue lane and one lane per track; clicking the ruler moves the playhead, and
with preview on the Scene view shows the sequence posed there without touching
the document. Keys and cues are picked, dragged on a 0.05 s grid and edited; a
track is added for any target and property and keyed at the playhead from the
value the scene holds. Every edit is one undoable step. Sequence Stage's intro
is authored this way, and its director hears the end and moves the stage on to
its idle loop. A cue's
sound is found by the export's walk of the scene, so a project need not list
it.

</details>

<a id="profiler"></a>

### Profiles the editor's frame, and benchmarks against the shipped host

The Profiler keeps 300 frames of per-phase CPU timing and per-script costs; `--benchmark` compares with the shipped host.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/profiler/tests.rs`](../../editor/src/profiler/tests.rs), [`editor/src/benchmark.rs`](../../editor/src/benchmark.rs)

**See:** [`docs/editor-update.md`](../../docs/editor-update.md)

**Limits:**

- GPU time and a breakdown inside a script are not measured.

<details markdown="1">
<summary>Evidence</summary>

The Profiler panel keeps the last 300 frames of Play, each timed on the CPU
from one call of the editor's frame to the next: upkeep, the fixed steps'
effects, physics, screen UI, scripts, sprite animation and cameras, then each
view's presentation, extraction and encoding, the panels laid out around them,
egui painting the window, and the wait until the next frame. Bars are drawn
against the 60 fps budget; pointing at one shows that frame and a click pins
it. Each script's time and runs per frame are listed slowest first. A toolbar
toggle records frames while editing too.

`sindri-editor <project> --benchmark <report.json>` waits for the project's
assets, records frames at rest and then of Play with vsync off, writes every
frame's phases and exits; `project-benchmark` plays the same project offscreen
the way the browser host does. `scripts/frame-benchmark.py` runs both and
compares the work per frame.


The fixed [3D Benchmark](../../examples/benchmark3d/README.md) supplies 100 cubes,
100 inline pyramid surfaces and 100 references to one shared GLB, plus a floor.
Its native asset-queue/Session regression verifies ready, static poses, actual
extracted commands and shared model resources; real Chromium fetches all six
assets and draws the workload. Native/browser player and offscreen capture paths
now apply authored shadow settings; on/off captures verify delivery alongside
editor benchmarks. Imported models now receive the same directional map. Shared shadow/fog WGSL,
map-resize/disable/cache GPU regressions and native/browser green-receiver controls
exercise reception on imported geometry. Renderer fog supports distance, density
and height terms; authored fog is still forwarded only by editor viewports.
Models now cast material-aware shadows from their cached primitive geometry,
with retained hierarchy, both index widths and glTF mask/facing GPU regressions.
Native/browser imported-only controls prove casting/reception; Explorer adoption
remains open.
CPU phases and GPU completion wait are available. Standalone reports also record
physical textured 3D/model draws and passes, uniform queue writes/bytes, transient
geometry pairs and separate persistent cache upload events. Shadowed/unshadowed
consecutive-frame CI checks verify deltas; these counters exclude 2D, post and
initial resource uploads. Editor benchmark frames now carry matching primary-view
counters and separate extra Scene-view work. Frame capture/reset and missing
versus measured-zero views have regressions; native editor shadow controls prove
Game/standalone agreement. Native standalone reports optionally record actual
GPU timestamp intervals separately from CPU completion wait, including adapter,
period and scope; unsupported devices retain null timing. Real GPU query/reuse
and baseline-device regressions plus shadow-control CI exercise this.
Native editor benchmarks now request supported timestamp features without
altering eframe's device limits; each view owns its own reusable query pair.
Primary and extra Scene intervals accumulate independently, serialize in
microseconds and stay outside CPU phases. Capture/reset/null regressions and
real native shadow-control/two-view runs exercise delivery. Whole-frame
counters, browser timestamps and performance targets remain pending.
`BatchDrawStats` now records physical sprite/shape/glyph batches and queue
buffer payloads, excluding initialization/atlas uploads. Standalone and editor
reports carry primary and extra Scene scopes. Real GPU empty/reuse tests,
Platformer reports and rebuilt browser smoke exercise the shared path; the
fixed 3D CI workload records measured zeroes for 2D batches. The
[baseline record](../3d-benchmark-baseline.md) records the hardware/profile context,
short editor/standalone sample and native/browser shadow comparisons.

</details>

## Workspace

<a id="workspace"></a>

### A workspace the user arranges

Every panel is a tab that docks to an edge or floats in a corner; arrangements persist and are repaired on load.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/dock/tests.rs`](../../editor/src/dock/tests.rs), [`editor/src/ui/widgets/panel.rs`](../../editor/src/ui/widgets/panel.rs), [`editor/src/preferences.rs`](../../editor/src/preferences.rs)

<details markdown="1">
<summary>Evidence</summary>

Every panel is a tab, and every tab is dragged into any of eleven places: seven
docks — two columns down each side, one along the bottom, the centre split in
two — and four overlays, one anchored to each corner of the scene view. **A
dock takes room from the scene; an overlay covers it.** Dropping a tab on a
window edge docks it there; dropping it in a corner floats it. A place with
nothing in it is not drawn. Middle-click closes a tab and **View → Panels**
brings it back; **View → Arrangement** offers `Canvas`, `Docked`, and `Wide`.
Where every panel sits and how big it is survives a restart, as do the other
preferences.

Overlays anchor and stack rather than floating freely, roll up to their strip,
and are drawn opaque. A panel lives in exactly one slot, and the centre is
never empty. An arrangement read back from settings is repaired rather than
trusted. Every slot claims its full width before drawing, and
`editor/src/ui/widgets/panel.rs` carries a headless test that fails if the
claim is removed.

</details>

<a id="command-palette"></a>

### One field that finds anything

Ctrl+K searches panels, entities, files, scenes, arrangements and verbs with a tested scored-subsequence ranking.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/palette/tests.rs`](../../editor/src/palette/tests.rs), [`editor/src/palette/score.rs`](../../editor/src/palette/score.rs)

<details markdown="1">
<summary>Evidence</summary>

Ctrl+K opens a palette over everything else, searching the panels, the scene's
entities, the project's files and scenes, the arrangements, and the editor's
verbs at once; each row says which kind it is. Matching is a scored
subsequence rather than a substring, so `oscn` finds `orbital.scene`, and the
query is split into terms matched in any order, so a file is reachable by its
folder as well as its name. The ranking is a pure function in
`palette/score.rs` tested against the orderings a person would expect.

</details>

<a id="console-and-history"></a>

### Console and History

A filtered, bounded console that names entities, and a History dock that travels the undo stack.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/console.rs`](../../editor/src/console.rs)

<details markdown="1">
<summary>Evidence</summary>

The Console holds what the editor has actually said — every failure, what each
scene turned out to be when it opened, and every texture it names that nothing
has bound — bounded, with a repeated message collapsed into a count, and
feeding the error and warning counts in the status bar. It is filtered by
level, remembered across launches. Clear empties it. An entry about an entity
ends in that entity's name and selecting it goes there.

The History dock draws the undo stack: "Scene opened", every step in the order
it happened, the step the world is at marked, and the undone steps still listed
under it. Clicking one travels there, by calling the same undo and redo the
keys call, one step at a time.

</details>

<a id="project-browser"></a>

### Project browser: import, rename, copy, delete, preview

Lists the scene's directory with list/grid, search and previews of images, text, audio and fonts.

Engine — · Editor ✅ · Decay — · Proof ✅ — parity row *Project browser: import, rename, copy, delete, preview*

**Tested by:** [`editor/src/native/tests/project_browser.rs`](../../editor/src/native/tests/project_browser.rs), [`editor/src/project/ops/tests.rs`](../../editor/src/project/ops/tests.rs), [`editor/src/preview/tests.rs`](../../editor/src/preview/tests.rs), [`editor/src/audition.rs`](../../editor/src/audition.rs)

**Exercised in:** [`games/platformer`](../../games/platformer)

<details markdown="1">
<summary>Evidence</summary>

A Project dock lists the real contents of the directory the open scene lives
in, with a list/grid toggle, a search that filters it, a refresh, and a double
click on a scene row to open it. An image opens the slicer, a text file is read
in a monospace column, a clip plays on demand, and a font draws a sample in the
face itself. The clip plays through the editor's own audio device rather than
the scene's, so auditioning one needs no running world and cannot leave a
voice behind in it.

The textures a scene names load from the directory the scene lives in, through
the real asset pipeline; each load or failure is named in the console. Saving a
texture the open scene uses shows the edit within about a second, without
restarting, and without blinking through the missing checker.

</details>

## Local assistant

<a id="assistant-setup"></a>

### The local assistant sets itself up inside the editor

One button downloads, verifies, starts and tests a pinned local model; features are tested afterwards and switched on only if they pass.

Engine — · Editor ✅ · Decay — · Proof —

**Tested by:** [`editor/src/assistant/tests.rs`](../../editor/src/assistant/tests.rs), [`editor/src/assistant/fetch/tests.rs`](../../editor/src/assistant/fetch/tests.rs), [`editor/src/assistant/install/tests.rs`](../../editor/src/assistant/install/tests.rs), [`editor/src/assistant/managed/tests.rs`](../../editor/src/assistant/managed/tests.rs), [`editor/src/assistant/server/tests.rs`](../../editor/src/assistant/server/tests.rs), [`editor/src/assistant/catalogue/tests.rs`](../../editor/src/assistant/catalogue/tests.rs)

**See:** [`docs/local-ai-architecture.md`](../../docs/local-ai-architecture.md)

**Limits:**

- Windows and Intel Macs are told the assistant is not available yet; nothing is pinned for them.
- No real model runs in CI.

<details markdown="1">
<summary>Evidence</summary>

The Assistant panel opens on one card: what the assistant does, that it runs
only on this computer, the whole download (4.7 GB) stated before anything
starts, and whether the model fits this machine's memory. One button, "Set up
the assistant", does everything: no download page, no terminal, no installer
and no password.

It fetches the pinned llama.cpp server and the pinned Qwen2.5 Coder 7B file
(`editor/assets/ai-runtime.json`, `ai-model-files.json`, commit-pinned URLs
with SHA-256 and size) into the person's own data folder through the platform's
curl, refuses any file whose hash does not match, unpacks the runner with `tar`
beside its libraries, starts it on a free loopback port, waits for its
`/health`, and tests that the model works with a question whose answer is
known. The panel shows the steps as they happen, Stop works at every step, and
an interrupted setup resumes. Failures name their step and say what to do.

What it can do in the editor is tested afterwards, feature by feature; a
feature that did not pass is shown as off, with "Test again". Each result is
saved against the exact model hash and runner build. Tested without a network
by an end-to-end run: real curl fetching local files, real `tar`, and a
stand-in runner that answers `/health` and fixes the cases. The real pinned
runner has been downloaded, verified, unpacked and started on Linux.

</details>

<a id="assistant-repair"></a>

### Compile errors and a proposed fix for a previewed script

A previewed `.decay` file shows its errors; the local model proposes a fix that must compile and keep every script it declared.

Engine — · Editor ✅ · Decay ✅ · Proof —

**Tested by:** [`editor/src/assistant/repair/tests.rs`](../../editor/src/assistant/repair/tests.rs)

**Limits:**

- It rewrites whole files; semantic edits wait on the language tooling.

<details markdown="1">
<summary>Evidence</summary>

Selecting a `.decay` file shows each error with its line and column, from
`sindri_decay::check_source` — the analysis and host environment Play compiles
with — or says it compiles. Once a model has passed the repair check, **Propose
a fix** sends the file, its errors and the generated host API to the model on a
worker thread.

The answer is untrusted. `assistant/repair.rs` takes the longest fenced block
from it, compiles it, and refuses a candidate that does not compile, is the
file unchanged, is larger than 32 KB, or no longer declares a script or
component the original did. A refused candidate goes back to the model with
only its own errors, at most twice. What survives is shown as a diff; Accept
writes it in one step, refusing a file that changed on disk since the fix was
asked for, and "Put the previous version back" undoes it. The loop is proved
against scripted models and the transport against a stand-in runner on a real
socket.

</details>

## Not yet

<a id="editor-multi-scene"></a>

### Editing several scenes side by side

The editor edits one scene at a time; there are no project settings beyond the scene list and main scene.

Engine — · Editor ❌ · Decay — · Proof —

<a id="editor-export"></a>

### Build and export controls

Static web export is a CLI and CI workflow; the editor has no build controls.

Engine — · Editor ❌ · Decay — · Proof —

<a id="editor-protocol"></a>

### Versioned editor protocol

The editor and runtime are one process with no versioned protocol between them.

Engine — · Editor ❌ · Decay — · Proof —

<a id="in-editor-scripts"></a>

### In-editor script editing

Decay source opens as a read-only preview; editing, completion and diagnostics live in the VS Code extension.

Engine — · Editor ❌ · Decay — · Proof — — parity row *In-editor script editing*

<a id="scene-file-reload"></a>

### Reloading an edited scene file

Hot reload covers assets, not the scene file: editing an open scene on disk is not noticed.

Engine ❌ · Editor ❌ · Decay — · Proof —

<a id="polygon-handles"></a>

### Viewport authoring of polygon vertices

Irregular polygon vertices work in the runtime and Decay, but have no point handles or purpose-built inspector.

Engine — · Editor ❌ · Decay — · Proof —

## 3D foundation

<a id="model-asset-picker"></a>

### Model asset field metadata and picker

Core model asset meaning describes render/static-collision references and is generated into the catalogue

Engine ✅ · Editor ✅ · Decay — · Proof 🟡 — parity row *Model asset field metadata and picker*

**Tested by:** [`editor/tests/inspector_field_meanings.rs`](../../editor/tests/inspector_field_meanings.rs)

**Exercised in:** [`examples/model-collision`](../../examples/model-collision)

**See:** [`docs/model-collision-editor-review.md`](../../docs/model-collision-editor-review.md)

<details markdown="1">
<summary>Evidence</summary>

`ModelAssetDecoder` parses self-contained binary GLB into reusable CPU geometry,
node local matrices/children, materials and embedded base-color textures. The
tiny deterministic fixture exercises both index widths, multiple primitives,
mesh reuse, non-indexed triangles and malformed/unsupported diagnostics.
`AssetLoader` reuse is covered. A separate renderer resource and GPU path retain
normal/UV geometry, hierarchy, textures and materials, composing node and entity
matrices with inverse-transpose normals. GPU readback coverage checks colors,
depth and upload reuse; renderer resource tests check malformed data. Scene
`ModelComponent` references validate asset paths and extract shared resources
with entity world transforms. Disabled models are discovered without drawing;
missing bindings name the asset. The public facade converts decoded models
without cross-coupling foundational crates. The editor catalogue assigns the
registered component an existing icon/family. Render and static-collision asset
fields now declare model metadata, use a GLB-only project picker and complete
Add Component with an actual model. Independent widget/component history scopes
keep the two selections separate; native review verifies Undo/Redo, save/reopen
and Play with an alternate wall. Broader model authoring remains deferred. Model manifests and export retain original GLB bytes and discover
scene/prefab
references, including inactive entities. Native project capture and the WebGPU
project host prepare/bind models once through the existing asset queue and
apply scene lighting. The exact Low Tide cutaway crawler was visually inspected
in both runtimes at 1200 × 1000, correctly oriented and visibly recognizable.
Exported bytes match the source hash. Full editor authoring,
animation and skinning remain deferred. This external proof does not claim
the repository's in-tree game completion surface. Model Collision Lab now also
has native player-window visual proof on X11/software Vulkan, after editor asset
selection and save/reopen; the imported wall blocks keyboard movement and the
walker renders on the goal pad.
See [imported models](../imported-models.md).

</details>
