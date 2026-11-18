//! The tools `sindri mcp` lists: each one's name, what it answers and the
//! arguments it takes, as an agent reads them.

use serde_json::{Value, json};

pub(super) fn tools() -> Value {
    let mut tools = command_tools();
    // The questions go after the check and the run, before the references
    // and the picture, which is the order an agent meets them in.
    let rest = tools.split_off(2);
    tools.extend(query_tools());
    tools.extend(language_tools());
    tools.extend(rest);
    Value::Array(tools)
}

/// The tools that read the project as a whole: lookups, not runs.
fn query_tools() -> Vec<Value> {
    let Value::Array(tools) = json!([
        {
            "name": "project_overview",
            "description": "What a Sindri project holds, without reading it: its scenes and prefabs with their entity counts (and any that do not read), each Decay file and the scripts it declares, its stylesheets, how many entities use each component, and its other files by kind. Start here in a project you have not seen. Answers `sindri project overview --format json`.",
            "inputSchema": { "type": "object", "properties": { "project": project_property() }, "required": [] }
        },
        {
            "name": "scene_outline",
            "description": "A scene or prefab as a tree of entities: each one's ID, name, components (a script as sindri.script(Name)), the prefab it is an instance of, and whether it is switched off. Far smaller than the file; read it before editing a scene, then read or edit only the entities you need. Answers `sindri scene outline --format json`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project_property(),
                    "scene": { "type": "string", "description": "The scene or prefab: its file name (wilds, wilds.scene, lamp.prefab) or asset ID. The main scene unless given." }
                },
                "required": []
            }
        },
        {
            "name": "find_entities",
            "description": "Entities across every scene and prefab matching all the filters given, each with its scene, ID, name, parent, component keys and prefab. Answers `sindri entity find --format json`; lists at most 200 and says how many more matched.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project_property(),
                    "component": { "type": "string", "description": "A component type, such as sindri.sprite; matches its named parts too." },
                    "name": { "type": "string", "description": "Part of the entity's name or ID, ignoring case." },
                    "script": { "type": "string", "description": "A script it runs, by container name (Hero) or source file (hero.decay)." },
                    "prefab": { "type": "string", "description": "A prefab it is an instance of, by file name or asset ID." },
                    "scene": { "type": "string", "description": "Only this scene or prefab." }
                },
                "required": []
            }
        },
        {
            "name": "show_entity",
            "description": "One entity exactly as its file has it (every component and field), the scene it is in, and the IDs of its children. Answers `sindri entity show --id ID`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project_property(),
                    "id": { "type": "string", "description": "The entity's stable ID." },
                    "scene": { "type": "string", "description": "The scene or prefab it is in, when the ID is in more than one." }
                },
                "required": ["id"]
            }
        },
        {
            "name": "asset_uses",
            "description": "Everything that names a file: each entity and component field (as a path, like sindri.sprite.texture) and each prefab instance, across every scene and prefab. Ask before renaming, moving or removing a file. Answers `sindri asset uses --asset FILE --format json`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project_property(),
                    "asset": { "type": "string", "description": "The file, by asset ID (textures/hero.png) or with assets/ in front." }
                },
                "required": ["asset"]
            }
        }
    ]) else {
        unreachable!("a literal array")
    };
    tools
}

/// The tools about the two languages: how Weave styles an element, and
/// whether a Decay draft compiles.
fn language_tools() -> Vec<Value> {
    let Value::Array(tools) = json!([
        {
            "name": "style_show",
            "description": "How one UI element is styled and why: every Weave rule that reaches it, strongest first, with its file, line, selector and specificity, each declaration marked as applying or overridden, and the final computed values. Use when an element does not look as its stylesheet says it should. Answers `sindri style show --id ID --format json`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project_property(),
                    "id": { "type": "string", "description": "The element's entity ID." },
                    "scene": { "type": "string", "description": "The scene or prefab it is in, when the ID is in more than one." }
                },
                "required": ["id"]
            }
        },
        {
            "name": "style_match",
            "description": "Which UI elements, across every scene and prefab, a Weave selector reaches, matched by the engine's own cascade. Check a selector before writing a rule with it. Elements are matched at rest: :hover and :focus never match here. Answers `sindri style match --selector S --format json`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project_property(),
                    "selector": { "type": "string", "description": "A Weave selector, such as `.hud > .button` or `#title`." }
                },
                "required": ["selector"]
            }
        },
        {
            "name": "style_unused",
            "description": "The Weave rules no element of any scene or prefab reaches at rest. A rule for a class a script adds while the game runs shows here too, so read it as a list to look at, not of mistakes. Answers `sindri style unused --format json`.",
            "inputSchema": { "type": "object", "properties": { "project": project_property() }, "required": [] }
        },
        {
            "name": "check_script",
            "description": "Compile a Decay script you have not written yet against this project: the real host API and every other script in the project, exactly as project_check would once it is saved. Give its full text, and the file it will be (replacing what is there). Answers with each error's code, line and column, and the scripts it declares. Check a draft before writing it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": project_property(),
                    "source": { "type": "string", "description": "The script's full text." },
                    "file": { "type": "string", "description": "Its asset ID, such as scripts/hero.decay; a draft name otherwise." }
                },
                "required": ["source"]
            }
        },
        {
            "name": "decay_api",
            "description": "One name in the Decay host API, instead of the whole reference: a global (`Physics`, with every member and its signature), a type (`Entity`), or a member (`Physics.mask`, `this.entity`). Suggests the nearest names when there is no such name. Answers `sindri script api NAME --format json`.",
            "inputSchema": {
                "type": "object",
                "properties": { "name": { "type": "string", "description": "Such as World, World.spawn or this.entity." } },
                "required": ["name"]
            }
        }
    ]) else {
        unreachable!("a literal array")
    };
    tools
}

/// The tools that answer a command: check, run, references, picture.
fn command_tools() -> Vec<Value> {
    let Value::Array(tools) = json!([
        {
            "name": "project_check",
            "description": "Check a Sindri project the way a run reads it: every scene, prefab, Decay script, Weave stylesheet and sindri.toml. Answers `sindri project check --format json`: every problem with a stable code, its file, and its line or entity. ok is false when there are errors.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": "The project folder, holding sindri.toml. The folder the server was started in unless given." }
                },
                "required": []
            }
        },
        {
            "name": "play_run",
            "description": "Play a Sindri project headless at 60 steps a second with scripted input, styled and laid out as it is drawn. Answers `sindri play run --format json`: the shared board (Game state), where each tracked entity ended up, and what scripts printed and failed, by frame; and each `shot` step's picture, inline.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": "The project folder, holding sindri.toml. The folder the server was started in unless given." },
                    "scene": { "type": "string", "description": "A scene to start in, by file name, instead of the main one." },
                    "steps": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": STEPS
                    },
                    "track": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Stable IDs of entities whose final position to report."
                    }
                },
                "required": []
            }
        },
        {
            "name": "capabilities",
            "description": "What this engine build can be told, as JSON, generated from the engine itself so it is never out of date. Read these instead of guessing: scene and prefab are the JSON Schemas for .scene and .prefab files; components lists every component with its fields, defaults and what each field means; decay is the script API with types; decay-language is the language; weave is every style property; decay-guide and weave-guide are how to write each, with examples, what surprises and what does not exist. Read the guide before writing your first script or stylesheet in a project. Answers `sindri capabilities NAME`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "enum": sindri_capabilities::REFERENCES.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
                        "description": "Which document."
                    }
                },
                "required": ["name"]
            }
        },
        {
            "name": "capture",
            "description": "A picture of a Sindri project as a player would see it: opened at its main scene or `scene`, played through `steps`, then photographed. Answers the picture inline, and the run's report as `sindri render capture --format json` does.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": "The project folder, holding sindri.toml. The folder the server was started in unless given." },
                    "scene": { "type": "string", "description": "A scene to start in, by file name, instead of the main one." },
                    "steps": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": STEPS
                    },
                    "track": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Stable IDs of entities whose final position to report."
                    },
                    "width": { "type": "integer", "description": "Pixels across; 1280 unless given." },
                    "height": { "type": "integer", "description": "Pixels down; 720 unless given." }
                },
                "required": []
            }
        }
    ]) else {
        unreachable!("a literal array")
    };
    tools
}

/// A tool's `project` argument.
fn project_property() -> Value {
    json!({ "type": "string", "description": PROJECT_ABOUT })
}

const PROJECT_ABOUT: &str =
    "The project folder, holding sindri.toml. The folder the server was started in unless given.";

/// What a step may be, as the tools describe it.
const STEPS: &str = "In order: wait:<seconds>; key:<Key> (a tap: Space, F, ArrowLeft); hold:<Key> and release:<Key>; set:<name>=<number> (a shared board value); click:<entity> (a laid-out UI element, by ID); type:<text>; place:<entity>=<x>,<y>; near:<entity>=<name> (in front of the first thing of that name); shot or shot:<name> (a picture, answered inline); film:<seconds>x<count> (that many frames across that span, as one picture, answered inline: a jump's arc, a walk, a fade); debug:colliders, debug:ui, debug:all or debug:off (later pictures draw every 2D collider piece, green or yellow for a sensor, and every laid-out UI element, magenta or cyan where it takes presses: why something fell through or a click missed); probe:<x>,<y> (what is at that pixel: the UI element, the one a press reaches, the world point, the colliders holding it; needs no picture); expect:<what><op><value>, op one of = != < <= > >=, checked where it stands and failing the run when it does not hold: a board value (expect:score>=1), an entity's place by ID (expect:player.x>3), the scene playing (expect:scene=level2) or a printed line (expect:printed=Game over). Write the expectations a change should meet, then play_run them: ok is false and expectations says which failed and what was there instead.";
