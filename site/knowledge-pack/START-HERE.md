# Start here: making a game with Sindri

Sindri AI knowledge pack, file `00-start-here.md`. Read this file first, all of
it. It tells you what Sindri is, how a game is laid out, how to write its files
the way the Sindri editor writes them, which file in this pack answers which
question, and the mistakes models most often make.

This pack describes Sindri engine {{ENGINE_VERSION}}, scene format version
{{SCENE_FORMAT_VERSION}}. Sindri is pre-alpha and changes quickly. When this
pack and the user's own project disagree, the project wins: copy what its
files already do. When you are unsure and can browse, the source of truth is
<https://github.com/vardirhq/sindri-engine>.

## What Sindri is

Sindri is a 2D and 3D game engine with a visual editor. A game is **not** Rust
code. A game is a folder of text files:

- **Scenes** (`.scene`) and **prefabs** (`.prefab`): JSON describing entities and
  the components on them (sprites, shapes, cameras, physics bodies, text, UI).
- **Decay scripts** (`.decay`): gameplay code in Decay, Sindri's own statically
  typed language. Decay looks a little like Rust but is **not** Rust, and not
  JavaScript, C# or GDScript.
- **Weave stylesheets** (`.weave`): CSS-like styling and layout for screen UI.
- **`sindri.toml`**: the project file.
- Assets: images (`.png`), sounds (`.wav`, `.ogg`, `.mp3`), fonts (`.ttf`),
  models (`.glb`).

The editor is one way to write those files. You writing them is another, and it
is supported: the files are the interface.

## Which file answers which question

| File | What it answers |
| --- | --- |
{{FILE_INDEX}}

How to search this pack:

- **The exact name of anything a script can call** (`World.spawn`,
  `Input.just_pressed`, `Physics.set_velocity`, `this.transform`): search
  `02-decay-api.md`. It is generated from the engine, so a name that is not
  there does not exist. Never use an API from memory.
- **The exact name and fields of a component** (`sindri.sprite`,
  `sindri.physics2d.collider`): search `04-components.md` for the type name.
- **How to do a thing** ("jump", "spawn", "score", "sound", "button"): search
  the examples (`12-`, `13-`, `14-`) for a script that already does it, then
  `03-scripting-guide.md`.
- **Whether something is supported at all**: search `11-what-works.md`. A ❌
  means it does not exist; do not build on it.

## How a project is laid out

This is the folder the editor's **New Project** writes:

```text
my-game/
├── sindri.toml        the project file
├── main.scene         the first scene
├── scripts/           .decay files
├── textures/          .png files
└── fonts/             .ttf files
```

`sindri.toml`, as the editor writes it:

```toml
format_version = 1

[project]
name = "My Game"
main_scene = "main.scene"
```

Optional fields: `scenes = ["level2.scene"]` lists every other scene the game
can go to with `Scene.go(...)` (a scene not listed is not exported), and
`[assets] include = ["audio/jump.wav"]` lists files a script uses by name that
no scene mentions, such as sounds played from code. Details:
`05-projects-scenes-prefabs.md`.

**The rule that breaks most hand-written projects:** every path inside a scene
(a texture, a script, a font, a sound) is relative to **the folder the scene
file is in**, not to the project root. If the scene is `assets/level.scene`
and the image is `assets/textures/hero.png`, the scene names it
`"textures/hero.png"`. Writing `"assets/textures/hero.png"` loads nothing.

Some projects keep their scene in an `assets/` folder (the examples in this
pack do); then `main_scene = "assets/level.scene"` and every path inside the
scene is relative to `assets/`.

## Writing files the way the editor does

The editor writes every file in one canonical form. A file you write does not
have to be byte-identical (whitespace does not matter to the loader), but it
must have the same shape, and following these rules means the editor opens it
cleanly and its next save changes nothing important.

### A new scene

The editor's **New Scene** writes a world camera and a sun. Built from the
engine's registered defaults, that is:

```json
{{NEW_SCENE}}
```

- The camera is perspective, for 3D. **A 2D game uses an orthographic
  camera**: set `"projection": "orthographic"` and give it a
  `"vertical_size"` (how many world units tall the view is), as every 2D
  example does. A scene with no camera draws nothing.
- A 2D game made only of sprites and shapes does not need the sun; it lights 3D
  models.
- `format_version` must be exactly {{SCENE_FORMAT_VERSION}}; any other number is
  refused.
- `metadata.name` is the scene's name.
- `entities` is a list; order does not matter (the editor sorts by `id`).

### An entity

| Key | Meaning |
| --- | --- |
| `id` | Required, unique in the scene, never changed once other things refer to it. The editor names new ones `game-object-1`, `game-object-2`, …; readable ids such as `"player"` or `"coin-3"` are better and equally valid. |
| `name` | What the editor's hierarchy shows, and what `World.find("Name")` looks for. |
| `parent` | The `id` of the parent entity. The child's transform is then relative to the parent. |
| `transform_3d` | `position` `[x, y, z]`, `rotation` a quaternion `[x, y, z, w]` (no rotation is `[0, 0, 0, 1]`), `scale` `[x, y, z]`. 2D games use it too: x right, y up, z for depth. |
| `components` | An object: component type name to its data. See `04-components.md`. |
| `disabled` | `true` to switch the entity and everything under it off. Omit when false. |
| `prefab` | Instead of `components`: this entity is a copy of a prefab. See `05-projects-scenes-prefabs.md`. |

When the editor adds a component, it writes the component's **default
payload**: the "Fields" block in `04-components.md`. Do the same, then change
only the fields you need. A component with no default (it names an asset, such
as a sprite's texture) must be given that asset.

### A script on an entity

A script is attached with the `sindri.script` component:

```json
"sindri.script": {
  "source": "scripts/player.decay",
  "script": "Player",
  "properties": { "speed": 6.0 }
}
```

`source` is the file (relative to the scene's folder, as above); `script` is
the name after `script` inside it; `properties` sets the script's `@export`
fields for this entity. A property the script does not declare as `@export` is
an error, not ignored.

The matching script:

```rust
script Player {
    @export let speed: number = 6.0;
    var jumps: number = 0.0;

    fn start() {
        // once, before the first update
    }

    fn update(dt: number) {
        let move = Input.axis("ArrowLeft", "ArrowRight");
        this.transform.position.x += move * speed * dt;
    }
}
```

Decay code blocks in this pack are marked `rust` only so they are coloured;
they are Decay.

## How to make a game: the workflow

1. **Start from a working example, not from nothing.** Find the example in
   `12-`, `13-` or `14-` closest to what is wanted, and copy its files whole.
   Change one thing at a time.
2. **Look up every name.** Every host call in `02-decay-api.md`, every
   component in `04-components.md`. If it is not there, it does not exist.
3. **Write whole files.** When giving the user a file, give the complete file
   with its path, not a fragment or a diff, unless they ask otherwise.
4. **Keep scripts small and single-purpose.** One script per behaviour
   (`player.decay`, `coin.decay`, `hud.decay`), with shared game values in a
   `state` (see `03-scripting-guide.md`, "Declared state").
5. **Ask the user to check and run it.** With a copy of the Sindri repository
   and Rust installed, these work from the repository folder:

   ```bash
   cargo run --quiet --package decay-lsp -- --check path/to/my-game   # checks every script
   cargo editor path/to/my-game                                       # opens it in the editor
   cargo run --release --package sindri-player -- path/to/my-game     # plays it in a window
   ```

   The checker's errors carry a stable code such as
   `decay-semantic/unknown-name`; `01-decay-language.md` lists them all. Fix
   every error, then check again; do not stop after the first.

## Decay: things models get wrong

Copied from the Decay language reference. Read both lists before writing any
Decay.

### Surprising behaviour

{{DECAY_SURPRISES}}

### What does not exist

{{DECAY_ABSENT}}
