#!/usr/bin/env python3
"""Assemble the AI knowledge pack: Markdown files for a chat model's knowledge.

A model that has never seen Sindri (a custom GPT, a project in a chat app, an
agent with only file search) gets these files and nothing else. Every fact in
them comes from a document or project already in this repository; the only
hand-written page is `site/knowledge-pack/START-HERE.md`. A document listed
here that no longer exists fails the build rather than leaving a gap.

    python site/build_knowledge_pack.py [OUT]     # default: target/knowledge-pack
"""

from pathlib import Path, PurePosixPath
import json
import math
import posixpath
import re
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[1]
OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "target/knowledge-pack"
REPOSITORY = "https://github.com/vardirhq/sindri-engine/blob/main"

# Pack files built from documents, in the order START-HERE lists them. Each
# entry: file name, title, what it answers, source documents.
DOC_FILES = [
    (
        "01-decay-language.md",
        "Decay language reference",
        "How Decay is written: syntax, types, fields, functions, events, state, "
        "enums, structs, lists, maps, what is surprising and what does not exist.",
        ["decay/LANGUAGE.md"],
    ),
    (
        "02-decay-api.md",
        "Decay host API (generated)",
        "Every name a Decay script may use to reach the engine: `this`, globals "
        "such as `World`, `Input`, `Physics`, `Audio`, `Scene`, and every type and "
        "member, with exact signatures. Generated from the engine.",
        ["docs/generated/decay-api.md"],
    ),
    (
        "03-scripting-guide.md",
        "Scripting guide",
        "How Decay scripts drive a Sindri game: attaching a script to an entity, "
        "exported properties, other entities, spawning, physics, input, audio, "
        "UI, timers, events, state, lifecycle, and the agent checklist.",
        [
            "docs/decay-agent-guide.md",
            "docs/scripting.md",
            "docs/camera-runtime-scripting.md",
        ],
    ),
    (
        "05-projects-scenes-prefabs.md",
        "Projects, scenes, prefabs and assets",
        "What a project is (`sindri.toml`), how a `.scene` file is shaped and "
        "versioned, how asset paths resolve, prefabs, profiles, and the component "
        "registry.",
        [
            "docs/project-format.md",
            "docs/scene-serialization.md",
            "docs/component-schema-registry.md",
            "docs/prefabs.md",
            "docs/profiles.md",
            "docs/asset-foundation.md",
            "docs/runtime-environment.md",
        ],
    ),
    (
        "06-2d.md",
        "2D: cameras, sprites, tiles, layers, lights, particles, text",
        "How 2D works in Sindri: the 2D model, depth and draw order, cameras, "
        "grids, tilemaps and terrains, sprite shaders and masks, 2D lights, "
        "particles, nine-slice, world text and transparency.",
        [
            "docs/2d-model.md",
            "docs/2d-depth-and-placement.md",
            "docs/cameras.md",
            "docs/grid.md",
            "docs/tile-system-2.md",
            "docs/terrains.md",
            "docs/sprite-shaders.md",
            "docs/sprite-masks.md",
            "docs/2d-lights.md",
            "docs/particles.md",
            "docs/nine-slice.md",
            "docs/world-text.md",
            "docs/rendering-transparency.md",
        ],
    ),
    (
        "07-physics-and-movement.md",
        "Physics and character movement",
        "2D and 3D physics: bodies, colliders, sensors, layers and masks, joints, "
        "materials, collision events, and character controllers.",
        ["docs/physics.md", "docs/character-movement.md"],
    ),
    (
        "08-weave-ui.md",
        "Weave UI",
        "Screen UI and HUDs: Weave stylesheets, responsive layout, and the full "
        "Weave language reference.",
        ["docs/weave.md", "docs/weave-reference.md"],
    ),
    (
        "09-3d.md",
        "3D: lights, models, materials",
        "3D lighting and shadows, imported GLB models, materials, and day/night.",
        [
            "docs/3d-lights.md",
            "docs/imported-models.md",
            "docs/rendering-materials.md",
            "docs/daynight.md",
        ],
    ),
    (
        "10-export-and-web.md",
        "Exporting and the web",
        "Exporting a project to a static web page and how the browser host runs it.",
        ["docs/export.md", "docs/browser.md"],
    ),
    (
        "11-what-works.md",
        "What works today, and what does not",
        "The capability inventory: for each feature, whether the engine, editor "
        "and Decay support it, and its known limitations. Check here before "
        "building on a feature.",
        [
            "docs/capabilities/core.md",
            "docs/capabilities/runtime.md",
            "docs/capabilities/projects.md",
            "docs/capabilities/assets.md",
            "docs/capabilities/decay.md",
            "docs/capabilities/input-and-audio.md",
            "docs/capabilities/physics.md",
            "docs/capabilities/rendering.md",
            "docs/capabilities/ui.md",
            "docs/capabilities/grids-and-voxels.md",
        ],
    ),
]

COMPONENTS_FILE = (
    "04-components.md",
    "Scene components (generated)",
    "Every component a `.scene` or `.prefab` entity may carry: its exact "
    "`sindri.*` name, its fields with default values, what each field means, "
    "and a real use from an example project.",
)

# Feature examples: each shows one capability, is small, and is exported to the
# site by CI, so it is known to load. They are the patterns to copy.
EXAMPLE_FILES = [
    (
        "12-examples-2d.md",
        "Feature examples: 2D, input, audio, UI, physics",
        "Complete, working example projects, each showing one capability. "
        "Every file is shown whole, with its path.",
        [
            "examples/input",
            "examples/audio",
            "examples/camera",
            "examples/tween",
            "examples/sequence",
            "examples/spatial",
            "examples/ui",
            "examples/physics",
            "examples/sprite-masks",
            "examples/sprite-shaders",
        ],
    ),
    (
        "13-examples-3d.md",
        "Feature examples: 3D",
        "Complete, working 3D example projects: transforms, orbit camera, "
        "character movement, lighting, model collision, day/night.",
        [
            "examples/transform",
            "examples/orbit",
            "examples/character3d",
            "examples/lighting3d",
            "examples/model-collision",
            "examples/daynight",
        ],
    ),
    (
        "14-example-wiring-flappy.md",
        "Wiring reference: one small complete game",
        "Flappy, the smallest complete game in the repository, shown whole. Use "
        "it to see how a scene, several scripts, shared state and a HUD fit "
        "together, not as a model of game design.",
        ["games/flappy"],
    ),
]

# Text files of a project, shown whole. Everything else is listed by name.
PROJECT_TEXT = {
    ".scene", ".prefab", ".decay", ".weave", ".profile", ".actions", ".sheet",
    ".tileset", ".wgsl", ".toml", ".md",
}
JSON_TEXT = {".scene", ".prefab", ".profile", ".actions", ".sheet", ".tileset"}
# A JSON file larger than this is written one entity per line instead, which
# keeps every entity while dropping most of the lines.
DENSE_JSON_BYTES = 40_000
# A file still larger than this is a level's worth of data rather than a
# pattern: it is named, and the project's scripts are still shown.
MAX_FILE_BYTES = 120_000
SKIP_DIRS = {"src", "tests", "target", "pkg", "web", "__pycache__"}

LINK = re.compile(r"(!?)\[([^\]]+)\]\(([^)\s]+)\)")
FENCE = re.compile(r"^\s*(```|~~~)")
HTML_COMMENT = re.compile(r"<!--.*?-->\s*", re.S)


def fail(message):
    print(f"build_knowledge_pack: {message}", file=sys.stderr)
    sys.exit(1)


def read(path):
    full = ROOT / path
    if not full.is_file():
        fail(f"{path} is listed but does not exist; update site/build_knowledge_pack.py")
    return full.read_text(encoding="utf-8")


def pack_file_for(repo_path):
    for name, _, _, sources in DOC_FILES:
        if repo_path in sources:
            return name
    if repo_path == "docs/generated/sindri-capabilities.json":
        return COMPONENTS_FILE[0]
    return None


def rewrite_links(text, source):
    """Points a link at the pack file holding its target, or at the repository."""
    base = PurePosixPath(source).parent

    def replace(match):
        image, label, target = match.groups()
        if image or target.startswith(("http://", "https://", "mailto:")):
            return match.group(0)
        path, _, anchor = target.partition("#")
        if not path:
            return label
        resolved = posixpath.normpath(str(base / path))
        pack = pack_file_for(resolved)
        if pack:
            return f"{label} (in `{pack}`)"
        return f"{label} (`{resolved}` in the Sindri repository)"

    return LINK.sub(replace, text)


def demote(text, levels):
    """Pushes every heading outside a code fence down by `levels`."""
    out, fenced = [], False
    for line in text.splitlines():
        if FENCE.match(line):
            fenced = not fenced
        elif not fenced and re.match(r"#{1,5} ", line):
            line = "#" * levels + line
        out.append(line)
    return "\n".join(out)


def compact_json(value, indent=0):
    """JSON with scalar arrays on one line: the same data in far fewer lines."""
    pad = "  " * indent
    if isinstance(value, dict):
        if not value:
            return "{}"
        items = [
            f'{pad}  {json.dumps(k)}: {compact_json(v, indent + 1)}' for k, v in value.items()
        ]
        return "{\n" + ",\n".join(items) + f"\n{pad}}}"
    if isinstance(value, list):
        if all(not isinstance(v, (dict, list)) for v in value):
            return json.dumps(value)
        items = [f"{pad}  {compact_json(v, indent + 1)}" for v in value]
        return "[\n" + ",\n".join(items) + f"\n{pad}]"
    return json.dumps(value)


def headings(text, level):
    out, fenced = [], False
    for line in text.splitlines():
        if FENCE.match(line):
            fenced = not fenced
        elif not fenced and line.startswith("#" * level + " "):
            out.append(line[level + 1:].strip())
    return out


def header(name, title, answers, contents):
    lines = [
        f"# {title}",
        "",
        f"Sindri AI knowledge pack, file `{name}`. {answers}",
        "",
        "Start with `00-start-here.md` if you have not read it. "
        "Every fact here is copied from the Sindri repository; "
        "nothing in this file is a guess.",
        "",
        "## Contents of this file",
        "",
    ]
    lines += [f"- {entry}" for entry in contents]
    return "\n".join(lines) + "\n"


def build_doc_file(name, title, answers, sources):
    parts, contents = [], []
    for source in sources:
        text = HTML_COMMENT.sub("", read(source)).strip()
        text = rewrite_links(text, source)
        doc_title = (headings(text, 1) or [source])[0]
        sections = headings(text, 2)
        contents.append(
            f"**{doc_title}** (from `{source}`)"
            + (": " + "; ".join(sections) if sections else "")
        )
        parts.append(f"\n---\n\nSource: `{source}`\n\n" + demote(text, 1))
    return header(name, title, answers, contents) + "\n".join(parts) + "\n"


def project_files(project):
    root = ROOT / project
    if not (root / "sindri.toml").is_file():
        fail(f"{project} is listed as an example but has no sindri.toml")
    shown, listed = [], []
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if not path.is_file() or SKIP_DIRS & set(relative.parts[:-1]):
            continue
        if relative.name == "Cargo.toml":
            continue
        if path.suffix in PROJECT_TEXT:
            shown.append(path)
        else:
            listed.append(path)
    return root, shown, listed


def render_project_file(path):
    text = path.read_text(encoding="utf-8")
    language = {".decay": "rust", ".toml": "toml", ".wgsl": "wgsl", ".weave": "css"}.get(
        path.suffix, "json"
    )
    if path.suffix in JSON_TEXT:
        try:
            document = json.loads(text)
        except json.JSONDecodeError:
            return text, language
        text = compact_json(document)
        if len(text.encode("utf-8")) > DENSE_JSON_BYTES and isinstance(document, dict):
            text = dense_json(document)
    return text, language


def dense_json(document):
    """A document with each item of its lists on one line of its own."""
    lines = ["{"]
    keys = list(document)
    for index, key in enumerate(keys):
        value = document[key]
        comma = "," if index < len(keys) - 1 else ""
        if isinstance(value, list) and any(isinstance(v, dict) for v in value):
            items = [json.dumps(v, separators=(", ", ": ")) for v in value]
            body = ",\n    ".join(items)
            lines.append(f"  {json.dumps(key)}: [\n    {body}\n  ]{comma}")
        else:
            lines.append(f"  {json.dumps(key)}: {json.dumps(value)}{comma}")
    lines.append("}")
    return "\n".join(lines)


def build_example_file(name, title, answers, projects):
    parts, contents = [], []
    for project in projects:
        root, shown, listed = project_files(project)
        readme = root / "README.md"
        project_title = (
            headings(readme.read_text(encoding="utf-8"), 1) or [project]
        )[0] if readme.is_file() else project
        contents.append(f"**{project_title}** (`{project}`)")
        block = [f"\n---\n\n## Example: {project_title} (`{project}`)\n"]
        block.append(
            "Every text file of the project follows, whole, under its path "
            "relative to the project folder. JSON files are shown with short "
            "arrays on one line; whitespace does not matter to Sindri.\n"
        )
        if readme.is_file():
            body = rewrite_links(readme.read_text(encoding="utf-8"), f"{project}/README.md")
            block.append(demote(body.strip(), 2) + "\n")
        for path in shown:
            if path == readme:
                continue
            relative = path.relative_to(root).as_posix()
            text, language = render_project_file(path)
            if len(text.encode("utf-8")) > MAX_FILE_BYTES:
                block.append(
                    f"### `{relative}`\n\nOmitted: {len(text) // 1000} KB of level "
                    "data, too large to be a useful pattern.\n"
                )
                continue
            block.append(f"### `{relative}`\n\n```{language}\n{text.rstrip()}\n```\n")
        if listed:
            names = ", ".join(f"`{p.relative_to(root).as_posix()}`" for p in listed)
            block.append(f"### Binary files in this project\n\n{names}\n")
        parts.append("\n".join(block))
    return header(name, title, answers, contents) + "\n".join(parts) + "\n"


def scene_uses():
    """The smallest real payload of each component in the example projects."""
    uses = {}
    projects = [p for _, _, _, ps in EXAMPLE_FILES for p in ps]
    for project in projects:
        for path in sorted((ROOT / project).rglob("*")):
            if path.suffix not in (".scene", ".prefab"):
                continue
            try:
                document = json.loads(path.read_text(encoding="utf-8"))
            except json.JSONDecodeError:
                continue
            for entity in document.get("entities", []):
                for type_name, payload in (entity.get("components") or {}).items():
                    rendered = compact_json(payload)
                    where = f"{project}/{path.relative_to(ROOT / project).as_posix()}"
                    if type_name not in uses or len(rendered) < len(uses[type_name][1]):
                        uses[type_name] = (where, rendered)
    return uses


def describe_meaning(path, meaning):
    kind = meaning.get("kind")
    detail = {k: v for k, v in meaning.items() if k not in ("kind", "variants")}
    text = f"- `{path}`: {kind}"
    if detail:
        text += " (" + ", ".join(
            f"{k}: {json.dumps(v)}" for k, v in detail.items() if v is not None
        ) + ")"
    for word, fields in (meaning.get("variants") or {}).items():
        text += f"\n  - when `{word}`, the object also has: `{json.dumps(fields)}`"
    return text


def build_components_file():
    name, title, answers = COMPONENTS_FILE
    source = "docs/generated/sindri-capabilities.json"
    data = json.loads(read(source))
    uses = scene_uses()
    components = sorted(data["components"], key=lambda c: c["type_name"])
    contents = [f"`{c['type_name']}`: {c['display_name']}" for c in components]
    out = [header(name, title, answers, contents)]
    out.append(
        f"\nSource: `{source}`, engine {data['engine_version']}, scene format "
        f"version {data['scene_format_version']}.\n\n"
        "How to read an entry: **Fields** is the full component with every "
        "field at its default value; write it under the entity's `components` "
        "object keyed by its type name. **Meanings** says what a field is for "
        "when its shape cannot: `asset` names a file, `choice` lists the only "
        "accepted spellings, `range` bounds a number, `color` is "
        "`[r, g, b, a]` from 0 to 1, `angle` is radians, `entity` is another "
        "entity's `id`, `mask` is a 32-bit collision mask. `[]` in a path means "
        "every item of a list. A component with **no default** names an asset "
        "and must be given one.\n\n"
        "Two things an entity carries are not components here: its transform "
        "(the entity's `transform_3d`, see `05-projects-scenes-prefabs.md`) and "
        "its script (`sindri.script`, see `03-scripting-guide.md`).\n\n"
        "Registry note, verbatim:\n\n> " + data["about"] + "\n"
    )
    registered = {c["type_name"] for c in components}
    others = sorted(set(uses) - registered)
    if others:
        out.append(
            "\n## Components not in the registry\n\n"
            "The example projects also use these. They are read by their own "
            "systems rather than the registry, so they have no generated field "
            "list; the real use below is the pattern, and the named file "
            "explains them.\n"
        )
        for type_name in others:
            where, rendered = uses[type_name]
            out.append(f"\n### `{type_name}`, from `{where}`\n\n```json\n{rendered}\n```")
        out.append(
            "\n`sindri.script` is explained in `03-scripting-guide.md`; "
            "`weave.style` in `08-weave-ui.md`.\n"
        )
    for component in components:
        type_name = component["type_name"]
        out.append(f"\n## `{type_name}`: {component['display_name']}\n")
        out.append(
            "Can be added with no extra data."
            if component["addable"]
            else "Needs data the engine cannot invent (usually an asset); "
            "copy a real use rather than the defaults."
        )
        out.append(f"\n### Fields of `{type_name}`\n\n```json\n{compact_json(component['fields'])}\n```")
        if component["meanings"]:
            out.append(f"\n### Meanings of `{type_name}` fields\n")
            out += [describe_meaning(p, m) for p, m in sorted(component["meanings"].items())]
        if component["default_payload"] is None:
            out.append(f"\nNo default: `{type_name}` must name what it uses.")
        if type_name in uses:
            where, rendered = uses[type_name]
            out.append(f"\n### A real `{type_name}`, from `{where}`\n\n```json\n{rendered}\n```")
    return "\n".join(out) + "\n"


def file_index():
    rows = [("00-start-here.md", "Read first: the workflow, project layout, rules and traps.")]
    rows += [(n, a) for n, _, a, _ in DOC_FILES[:3]]
    rows.append((COMPONENTS_FILE[0], COMPONENTS_FILE[2]))
    rows += [(n, a) for n, _, a, _ in DOC_FILES[3:]]
    rows += [(n, a) for n, _, a, _ in EXAMPLE_FILES]
    return "\n".join(f"| `{n}` | {a} |" for n, a in rows)


def language_section(heading):
    """A `## heading` section of LANGUAGE.md, verbatim, one level down."""
    text = read("decay/LANGUAGE.md")
    match = re.search(rf"^## {re.escape(heading)}\n(.*?)(?=^## |\Z)", text, re.S | re.M)
    if not match:
        fail(f"decay/LANGUAGE.md has no '## {heading}' section")
    body = rewrite_links(match.group(1).strip().rstrip("-").strip(), "decay/LANGUAGE.md")
    return demote(body, 1)


def sun_rotation():
    """`default_sun_transform`'s rotation: yaw 30°, pitch -50°, in YXZ order."""
    yaw, pitch = math.radians(30.0) / 2, math.radians(-50.0) / 2
    # q = q_y(yaw) * q_x(pitch); the roll is zero.
    x = math.cos(yaw) * math.sin(pitch)
    y = math.sin(yaw) * math.cos(pitch)
    z = -math.sin(yaw) * math.sin(pitch)
    w = math.cos(yaw) * math.cos(pitch)
    return [round(v, 6) for v in (x, y, z, w)]


def new_scene(capabilities):
    """What the editor's New Scene writes (`editor/src/native/scene_new.rs`)."""
    defaults = {c["type_name"]: c["default_payload"] for c in capabilities["components"]}

    def rounded(value):
        if isinstance(value, float):
            return round(value, 6)
        if isinstance(value, list):
            return [rounded(v) for v in value]
        if isinstance(value, dict):
            return {k: rounded(v) for k, v in value.items()}
        return value

    document = {
        "format_version": capabilities["scene_format_version"],
        "metadata": {"name": "Main"},
        "entities": [
            {
                "id": "sun",
                "name": "Sun",
                "transform_3d": {
                    "position": [0.0, 10.0, 0.0],
                    "rotation": sun_rotation(),
                    "scale": [1.0, 1.0, 1.0],
                },
                "components": {"sindri.light": rounded(defaults["sindri.light"])},
            },
            {
                "id": "world-camera",
                "name": "World Camera",
                "transform_3d": {
                    "position": [0.0, 0.0, 9.0],
                    "rotation": [0.0, 0.0, 0.0, 1.0],
                    "scale": [1.0, 1.0, 1.0],
                },
                "components": {"sindri.camera": rounded(defaults["sindri.camera"])},
            },
        ],
    }
    return compact_json(document)


def build_start_here():
    template = read("site/knowledge-pack/START-HERE.md")
    capabilities = json.loads(read("docs/generated/sindri-capabilities.json"))
    replacements = {
        "{{FILE_INDEX}}": file_index(),
        "{{ENGINE_VERSION}}": capabilities["engine_version"],
        "{{SCENE_FORMAT_VERSION}}": str(capabilities["scene_format_version"]),
        "{{NEW_SCENE}}": new_scene(capabilities),
        "{{DECAY_SURPRISES}}": language_section("Surprising behaviour"),
        "{{DECAY_ABSENT}}": language_section("What does not exist"),
    }
    for key, value in replacements.items():
        if key not in template:
            fail(f"START-HERE.md lost its {key} placeholder")
        template = template.replace(key, value)
    return template


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for stale in OUT.glob("*.md"):
        stale.unlink()
    files = {"00-start-here.md": build_start_here()}
    for name, title, answers, sources in DOC_FILES:
        files[name] = build_doc_file(name, title, answers, sources)
    files[COMPONENTS_FILE[0]] = build_components_file()
    for name, title, answers, projects in EXAMPLE_FILES:
        files[name] = build_example_file(name, title, answers, projects)
    for name, text in sorted(files.items()):
        (OUT / name).write_text(text, encoding="utf-8")
    with zipfile.ZipFile(OUT / "sindri-knowledge-pack.zip", "w", zipfile.ZIP_DEFLATED) as archive:
        for name in sorted(files):
            archive.write(OUT / name, name)
    total = sum(len(t.encode("utf-8")) for t in files.values())
    print(f"knowledge pack: {len(files)} files, {total // 1000} KB, in {OUT}")


if __name__ == "__main__":
    main()
