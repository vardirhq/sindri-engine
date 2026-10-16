# Project format

A Sindri project is a directory containing `sindri.toml`.

That file is the whole of the format. Its presence is what makes a folder a
project rather than a folder with a scene in it, which is the distinction the
editor had no way to draw: it opened a scene file, showed whatever directory
that file happened to sit in, and called it the project.

## The file

```toml
format_version = 1

[project]
name = "Gather"
main_scene = "assets/gather.scene"
scenes = ["assets/house.scene"]
```

Four fields, and each is read by something today.

- `format_version` describes the file rather than the project, which is why it
  sits above the table — TOML puts bare keys before the first table, so a
  version inside `[project]` would be a field order that serializes into a file
  it cannot read back. A manifest whose version is higher than the editor
  understands is **refused**, not guessed at: opening it would mean ignoring
  whatever the newer field said, and a project that quietly loses a setting is
  worse than one that says it needs a newer editor.
- `name` is what the project is called. Stored rather than taken from the
  directory name, because the two are not the same thing: the companion game is
  called Causeway and lives in a folder called `game`. It is what the welcome
  window lists and what the project browser's header shows.
- `main_scene` is the scene opening the project opens, relative to the root and
  written with forward slashes so a checkout on another platform still finds it.
  Set from the project browser — **Set as main scene** on any scene row that is
  not already it — and claimed automatically by a scene made in a project that
  nominates none, which is never an overwrite: a project that already opens on
  something has been decided about.
  Optional, because a project can legitimately have no obvious first scene, and
  a nominated scene that has been deleted opens **nothing** rather than some
  other scene that happens to be nearby — standing one in for the other reads as
  though the named one loaded.

- `scenes` are the other scenes the project can reach, written the same way.
  Optional and usually absent: most projects have one place in them.

  Declared rather than discovered, which is the only list the exporter takes and
  is worth justifying. A scene reached by `Scene.go("house")` is a string inside
  a program, and the exporter deliberately does not look for strings that
  resemble paths — a field declared `String` is text however much it looks like
  one, which is the same reason prefabs are found through a script's declared
  types instead. Nor is `[assets] include` the place for them: that ships raw
  bytes, so a scene listed there would arrive without its own textures, scripts
  or prefabs, which is an export that looks complete and opens a door onto
  nothing.

  Listing the main scene here as well is allowed and changes nothing; it ships
  once.

  The editor's **Scenes** panel is this list as a board: a card for the main
  scene and one for each scene here, in order, with the doors between them
  drawn as arrows from the `Scene.go("…")` calls in the scripts each scene
  runs. Its **Add scene** menu, and a card's **Remove from project**, **Move
  earlier**/**later** and **Set as main scene**, write this field. Setting a
  listed scene as main puts the old main scene at the head of the list, so no
  scene stops shipping because another one was chosen. A script that goes to a
  scene this list does not carry is marked on its card, because a build would
  have nowhere to go.

  The editor writes the manifest by editing the file in place: a table it does
  not model, such as `[web.splash]`, and comments survive any of these.

`PROJECT_OVERVIEW.md` sketches a larger file: window size, feature flags, an
asset root, a web canvas selector. None of that is here, and the sketch itself
says why — "avoid designing an enormous configuration schema before features
require it". The next field arrives with the feature that reads it.

`sindri.toml` is not `sindri.manifest`. The manifest is an asset ledger of
bytes and hashes, written by a build and verified by a loader; this is project
metadata, written by a person or by the editor.

### `[web.splash]`

A browser build's own brand, shown after the Sindri loading screen while the
game loads: `image`, `title`, `caption`, `background` and `seconds`.
`docs/export.md` has what each means and how long it shows.

## What creating one makes

New Project writes a directory holding the manifest, a scene, and the folders
assets are put in:

```text
my-first-game/
├── fonts/
├── scripts/
├── textures/
├── main.scene
└── sindri.toml
```

The asset folders sit beside the scene rather than under an `assets/` root
because that is where asset references actually resolve today:
`SceneTextures::for_scene` roots the loader at the scene's own directory. A
layout that looked tidier and loaded nothing would be worse than a flat one. A
project laid out differently — Gather keeps its scene under `assets/` — is
opened exactly the same way; this is what the editor *creates*, not what it
requires.

The scene it writes is the one New Scene writes: one world camera, from the
component registry's own default payload. A second copy of that answer living
beside the project format is a copy that drifts.

A directory that already holds a `sindri.toml` is refused rather than
overwritten. A directory holding other files is allowed, and the form says so
before the button is pressed.

## File extensions

Every file the engine authors has an extension of its own, named for what it
is rather than for the notation inside it:

| Extension | What it is |
| --- | --- |
| `.scene` | a scene document |
| `.prefab` | a prefab |
| `.profile` | a data profile |
| `.sheet` | the sprite sheet that slices the texture of the same stem |
| `.tileset` | a tile set |
| `.actions` | an input action map |
| `.isobake` | an isometric baker recipe |
| `.decay` | a Decay script |
| `.weave` | a Weave stylesheet |
| `sindri.manifest` | the asset ledger a build writes |

Most of these hold JSON today, and their loaders still parse them as JSON, but
the extension is the format's name: the project browser, the export gatherer,
the asset manifest and decay-lsp all decide what a file is by it, and a plain
`.json` in a project is just a file. There is no fallback to the old
`*.scene.json`-style names; a project written before the change is migrated by
renaming its files (`level.scene.json` to `level.scene`) and the references to
them.

## The project root and the asset root are two directories

A project is rooted at its `sindri.toml`. Asset references are not: a scene
names its textures, scripts, fonts, and clips relative to **the directory the
scene file itself is in**, because that is where `SceneTextures::for_scene`
roots the loader.

For a project the editor creates those are the same folder, and the distinction
never shows. For Gather they are two folders apart:

```text
game/                       <- the project root: sindri.toml, Cargo.toml, src/
└── assets/                 <- the asset root: where references resolve
    ├── gather.scene
    └── textures/orb.png    <- the scene names this "textures/orb.png"
```

`assets/textures/orb.png` is that file's path from the project root and is not a
reference to it. Writing it into a texture field names a file the loader will
look for at `game/assets/assets/textures/orb.png`, which is nothing, and the
sprite draws the missing checker.

So the browser knows both. `ProjectEntry::relative` is the path below the root,
which is what a search result shows to tell two files of the same name apart;
`ProjectEntry::reference` is how a scene names the file, which is what every
picker offers, what **Copy asset path** copies, and what the inspector checks a
typed reference against. A file outside the asset root has no reference at all —
Gather's `src/main.rs` is a real file that no component can name — and is
offered by nothing rather than offered under a path that will not resolve.

The browser lists the asset root by default for the same reason. A project's
Cargo manifest and its `src/` are part of the project and are not part of what
an editor field can point at, and a panel two thirds full of rows whose paths
mean nothing is a directory listing rather than an asset browser. The rest of
the project is one control away in the browser's toolbar, and the choice is
remembered; the control is drawn only where the two listings actually differ.

## Which project a launch opens

In order of how deliberately it was asked for, which is the ordering
`scene_io` already applied to scenes:

1. **A path on the command line.** A directory holding a manifest opens as a
   project; anything else opens as a scene, including a path to nothing — a
   named file that is missing is a failure the editor reports, never a reason to
   open something else.
2. **The last project**, when the user asked for that. The welcome window's
   footer is the only place that preference is set.
3. **The welcome window**, which asks.

A scene carries its project with it. Opening one — from the command line, from
a file dialog, from a browser row — walks up from the file to the nearest
`sindri.toml`, so a scene inside a project opens *as* that project: the browser
is rooted at the project and headed with its name, and lists the assets inside
it. A scene in no project leaves the editor with none, which is the state it was
always in before projects existed and is still a perfectly good way to edit one
file.

Opening a project opens the scene the editor was last left in when that scene is
inside it, and `main_scene` otherwise. Reopening a project should put someone
back where they were working rather than at its front door.

## The welcome window

Its own window, and the editor's is hidden until a project is open. It is not
part of editing anything: the editor's window is about a scene — its title
carries that scene's name, its panels hold that scene's entities, its viewport
renders it — and a "no project open" state painted over all of that would be an
editor pretending to be a launcher.

It lists the projects that have been opened, most recent first, twelve at most.
A project that has moved or been deleted is **shown and marked missing** rather
than quietly dropped: silently pruning the list answers "where did my project
go" with an empty row where it used to be, and the editor cannot tell an
unmounted volume from a deletion. A row leaves only when someone asks it to, and
removing it touches nothing on disk.

Each row is remembered by path and shown by name, with the name stored beside
the path. Reading every remembered project's manifest to draw the list would
mean a file read per row per frame, and a project on a disconnected network
drive would hang the window rather than appear in it. The name is re-read
whenever a project is opened, so renaming a project in its manifest shows up
next time it is opened.

Each row also says when the project was last opened ("2 days ago"), recorded
when it is opened rather than read from the disk, and a search box filters the
list by name or folder. A list saved before times were recorded still reads;
its rows show no time until each project is opened again.

Beside the list are the two ways to get a project that is not on it — New and
Open — a Learn panel linking to the guides in `docs/`, and the projects this
repository ships as Examples, listed only when they are actually there.
`SHIPPED` is relative to the working directory, which is the repository root
under `cargo run` and is somewhere else entirely for an installed editor: a
sample row that fails on the click is worse than no sample row. The Learn panel
opens the local copy of a page when the repository is beside the editor and
the GitHub copy otherwise.

The banner and the tiles beside each project are drawn, not loaded: the editor
has no image loading yet, so a project's tile is its initial on a colour derived
from its name. Screenshot thumbnails need a cached capture per project and are
not done yet.

What it deliberately is not is Unity Hub. There are no editor versions to
install, no account, and no news, because Sindri has none of those things to
manage.

### How the window exists

A deferred egui viewport, which eframe's wgpu integration opens as a real second
window. Deferred rather than immediate because an immediate child viewport is
drawn by its parent, and the parent here is the hidden editor: eframe throttles
a hidden window to ten frames a second so that a `Visible` command still reaches
it, which would make the one window the user can see repaint at the rate of the
one they cannot.

The editor's window starts hidden and is revealed when a project opens. That is
also what makes closing the welcome window with no project open close the
editor: the alternative is a running process with nothing on screen and no way
back to it.

Two consequences worth knowing:

- The welcome window is titled "Sindri" and not "Sindri Editor", because
  `scripts/capture-editor.sh` finds the editor by matching a title ending in the
  latter. That script also names the demo scene explicitly now — a launch with
  nothing on its command line opens the welcome window, which is right for a
  person and wrong for a screenshot.
- Where multiple windows are unavailable, egui embeds a child viewport inside
  its parent. The editor checks for that and shows its own window, so the
  welcome window is not painted somewhere nobody can see. On Wayland,
  `set_visible(false)` is not honoured, so the empty editor window is visible
  behind the welcome window rather than hidden; nothing else changes.

## What this does not do yet

- One scene at a time. A project can hold many scenes and the Scenes panel
  shows them all, but the editor edits one of them at a time.
- Only half a settings surface. **Set as main scene** in the project browser
  nominates what a project opens on, and a scene made inside a project that
  nominates nothing claims the empty place. The project's *name* still cannot
  be changed from the editor — a project is renamed by editing the file.
- The runtime and exporter read the manifest's entry scene and asset policy,
  but there is no broader runtime settings surface yet: window policy, host
  module selection, and target-specific settings are not project fields.

## External static models

An entity can reference a self-contained GLB with
`"sindri.model": { "asset": "models/crawler.glb", "layer": 0 }`. The asset ID is
relative to the project's asset root; models remain separate files. Export finds
references in scenes and prefabs automatically, validates and preserves their
bytes, and records manifest kind `model`. Native project capture and the exported
WebGPU host load these references through the normal asset pipeline. See
[imported models](imported-models.md) for supported features and diagnostics.
