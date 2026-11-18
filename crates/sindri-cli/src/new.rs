//! `sindri project new`: a project an agent can start work in.
//!
//! The project is the editor's — the same `sindri.toml`, folders and blank
//! scene (`sindri_authoring::blank_scene`) — with what a coding agent needs
//! beside it, because a game made with Sindri does not carry this repository:
//!
//! - `AGENTS.md`, a short guide: where things belong, how to learn the shapes
//!   from the binary, and the check-and-look loop. Discovery and checks, not
//!   API facts, which it points at rather than restates, so it cannot go
//!   stale as the engine moves (`docs/agent-native-direction.md`).
//! - `CLAUDE.md`, which says to read `AGENTS.md`, and the same guide as a
//!   skill in `.claude/skills/sindri/`, for agents that load skills.
//! - The scene and prefab JSON Schemas in `.sindri/`, and VS Code settings
//!   that validate and complete scenes against them.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::report::SCHEMA_VERSION;

/// What `project new` made.
#[derive(Serialize)]
pub struct NewReport {
    pub schema_version: u32,
    pub project: String,
    pub name: String,
    /// Every file written, relative to the project.
    pub wrote: Vec<String>,
}

impl NewReport {
    #[must_use]
    pub fn human(&self) -> String {
        let mut text = format!("Created {} in {}\n", self.name, self.project);
        for file in &self.wrote {
            text.push_str("  ");
            text.push_str(file);
            text.push('\n');
        }
        text.push_str("\nNext: sindri project check, then sindri render capture --out shot.png\n");
        text
    }
}

/// The folders a new project has, as the editor makes them.
const FOLDERS: [&str; 3] = ["textures", "scripts", "fonts"];

/// The scene a new project opens on.
const SCENE: &str = "main.scene";

/// Makes a project called `name` in `root`.
///
/// # Errors
/// `root` already holds a project, or a file could not be written.
pub fn new(root: &Path, name: &str) -> Result<NewReport, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("a project needs a name".to_owned());
    }
    if root.join("sindri.toml").exists() {
        return Err(format!("{} is already a project", root.display()));
    }
    let extractor = sindri_scene::SceneExtractor::new().map_err(|error| error.to_string())?;
    let scene = sindri_authoring::blank_scene(extractor.components(), "Main")
        .to_canonical_json()
        .map_err(|error| error.to_string())?;
    let schema = |name: &str| {
        sindri_capabilities::reference(name)
            .ok()
            .flatten()
            .and_then(|schema| serde_json::to_string_pretty(&schema).ok())
            .map(|text| text + "\n")
            .ok_or_else(|| format!("the {name} schema would not generate"))
    };
    let files: Vec<(&str, String)> = vec![
        (
            "sindri.toml",
            format!(
                "format_version = 1\n\n[project]\nname = {}\nmain_scene = \"{SCENE}\"\n",
                serde_json::to_string(name).map_err(|error| error.to_string())?
            ),
        ),
        (SCENE, scene),
        ("AGENTS.md", guide(name)),
        ("CLAUDE.md", "Read and follow `AGENTS.md`.\n".to_owned()),
        (".claude/skills/sindri/SKILL.md", skill(name)),
        (".sindri/scene.schema.json", schema("scene")?),
        (".sindri/prefab.schema.json", schema("prefab")?),
        (".vscode/settings.json", VSCODE.to_owned()),
    ];
    for folder in FOLDERS {
        create(&root.join(folder))?;
    }
    let mut wrote = Vec::new();
    for (path, contents) in files {
        let full = root.join(path);
        if let Some(parent) = full.parent() {
            create(parent)?;
        }
        // Never over something someone wrote: a folder that was not a
        // project may still hold their AGENTS.md or editor settings.
        if full.exists() {
            continue;
        }
        std::fs::write(&full, contents)
            .map_err(|error| format!("{} could not be written: {error}", full.display()))?;
        wrote.push(path.to_owned());
    }
    Ok(NewReport {
        schema_version: SCHEMA_VERSION,
        project: root.display().to_string(),
        name: name.to_owned(),
        wrote,
    })
}

fn create(folder: &Path) -> Result<(), String> {
    std::fs::create_dir_all(folder)
        .map_err(|error| format!("{} could not be made: {error}", folder.display()))
}

/// The folder `sindri project new PATH` makes its project in, and the name
/// it gives it: `--name`, or the folder's own name.
pub fn target(path: Option<&str>, name: Option<&str>) -> Result<(PathBuf, String), String> {
    let root = PathBuf::from(path.ok_or("project new needs the folder to make it in")?);
    let name = name.map(str::to_owned).or_else(|| {
        root.file_name()
            .map(|name| name.to_string_lossy().into_owned())
    });
    Ok((root, name.ok_or("give the project a --name")?))
}

const VSCODE: &str = r#"{
  "json.schemas": [
    { "fileMatch": ["*.scene"], "url": "./.sindri/scene.schema.json" },
    { "fileMatch": ["*.prefab"], "url": "./.sindri/prefab.schema.json" }
  ],
  "files.associations": { "*.scene": "json", "*.prefab": "json" }
}
"#;

fn guide(name: &str) -> String {
    format!("# {name}\n\n{GUIDE}")
}

fn skill(name: &str) -> String {
    format!(
        "---\nname: sindri\ndescription: Work on {name}, a Sindri game: scenes and prefabs are JSON, behaviour is Decay, interface is Weave. Use before changing any file in this project, to learn the shapes from the engine and to check and look at every change.\n---\n\n{GUIDE}"
    )
}

/// The guide itself. Discovery and checks only: every fact about what may be
/// written is asked of the binary, which answers for the engine it is.
const GUIDE: &str = r#"A game made with the Sindri engine. Everything in it is a file: there is no
hidden editor state, so you change the game by editing files, and the
`sindri` command tells you whether the change works and shows you what it
did. If the Sindri editor has this project open, it picks up what you
change, the open scene included; if that scene has unsaved edits there, the
editor asks its user before reading your version.

## Where things belong

- **Scenes and prefabs** (`*.scene`, `*.prefab`) are JSON: entities with a
  stable `id`, an optional `parent`, a `transform_3d` and `components`
  keyed by type name (`sindri.sprite`, `sindri.physics2d.collider`, ...).
  A second component of one kind is a named part: `sindri.light2d#halo`.
- **Behaviour** is Decay, in `scripts/*.decay`, attached to an entity by a
  `sindri.script` component naming the file (`source`) and the script in it
  (`script`).
- **Interface styling** is Weave, in `*.weave` stylesheets.
- **`sindri.toml`** names the project, its main scene and its other scenes.
- Pictures go in `textures/`, fonts in `fonts/`.

## Learn the shapes from the engine, never from memory

Sindri is not in your training data the way older engines are, and a guess
is usually a field the engine silently ignores. Ask the binary, which answers
for the engine you are building against:

```bash
sindri capabilities                 # what can be asked
sindri capabilities components      # every component: fields, defaults, meanings
sindri capabilities scene           # JSON Schema for a .scene (prefab: .prefab)
sindri capabilities decay           # the script API, with types
sindri capabilities decay-language  # the language itself
sindri capabilities weave           # every style property
sindri capabilities decay-guide     # how to write Decay, with examples: read before the first script
sindri capabilities weave-guide     # how to write Weave: selectors, cascade, layout, animation
```

`.sindri/scene.schema.json` and `.sindri/prefab.schema.json` are the same
schemas, so an editor validates scenes as you write them. After upgrading
Sindri, write them again with `sindri capabilities scene >
.sindri/scene.schema.json` (and `prefab`).

## Find your way without reading everything

Scenes are long files. Ask about them instead of reading them whole:

```bash
sindri project overview                       # scenes, prefabs, scripts, components in use
sindri scene outline --scene main             # a scene as a tree, one line an entity
sindri entity find --component sindri.sprite  # or --name, --script, --prefab, --scene
sindri entity show --id player                # one entity exactly as written
sindri asset uses --asset textures/hero.png   # everything naming a file, before moving it
```

## Check, then look

After every change:

```bash
sindri project check --format json
```

reads every scene, prefab, script and stylesheet as a run does and reports
each problem with a stable `code`, its file, and its line or entity. Fix all
errors before going on; `ok` is false while any remain.

A clean check is not a working game. Play it and look:

```bash
sindri play run --steps "wait:1 hold:D wait:0.5 release:D shot:moved" --track player --format json
sindri render capture --out shot.png --steps "wait:1"
```

`play run` plays headless at a fixed 60 steps a second with scripted input
(`sindri help` lists the steps) and reports the shared board, where each
`--track`ed entity ended up, and what scripts printed and failed, by frame.
`shot` steps and `render capture` write pictures: open them and check that
what you meant is on the screen.

## Keep

- Entity ids stable: scripts, parents and prefab overrides name entities by
  them.
- `editor` blocks as they are; they are the editor's, and runtimes ignore
  them.
- Scenes in their canonical form: the editor writes them back byte for byte,
  so your diffs and its saves do not fight.

## Without a shell

`sindri mcp install` registers it with your agent (`--client claude`,
`claude-desktop`, `cursor`, `vscode` or `codex`). `sindri mcp` serves the
same commands as read-only MCP tools
(`project_check`, `play_run`, `capture`, `capabilities`), answering with the
same JSON and with pictures inline.
"#;
