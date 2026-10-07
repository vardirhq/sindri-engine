# Agent-native authoring

**Status:** Accepted direction  
**Scope:** How Sindri supports AI coding agents: external ones (Claude Code, Codex, and whatever follows them) and its own local one  
**Last updated:** 2026-10-07

## The decision

Sindri does not compete with general coding agents. People who use one bring
it, and Sindri makes sure it can work in a Sindri project without having seen
Sindri before. People who do not — no subscription, no terminal, no account —
get a smaller local agent inside the editor. Both agents stand on the same
foundation, so building it once serves both.

That foundation is three things:

1. **Docs an agent can trust** — mostly generated from the engine, so they
   cannot describe calls that do not exist.
2. **Checks an agent can run** — one command that says whether the project is
   valid, in JSON, before a person ever opens it.
3. **A way to see the result** — run the game headless and read its log and
   frames, because a valid project can still behave wrongly.

Everything else here explains why this is enough, how the two agents divide
the work, and what it deliberately
leaves out.

## Why the files are the interface

Everything a person authors in Sindri is already text: scenes and prefabs are
canonical JSON, gameplay is Decay, and interface is Weave. The editor is one
way to write those files. An agent editing them directly is another, and it is
already how agents work on this repository's own games.

So Sindri does not need a second way to change a project. A CLI or MCP tool
such as `create_entity` or `set_component` would be a narrower, slower copy of
what an agent already does with a text editor, and one more vocabulary it
would have to learn. The file is the API; git is the undo; the diff is the
proposal a person reviews.

What an agent editing text does *not* have is the knowledge the editor carries
for a person: which components exist, what a field may hold, whether a
reference resolves, and what the game does when it runs. That is the gap this
direction closes.

## Two agents, one foundation

General agents improve faster than anything Sindri could embed, and competing
with them means owning providers, keys, context management, tool loops and a
chat interface — none of which is game-engine work. An external agent also
settles the provider question for free: the model is whichever one the person
already pays for or runs.

But not everyone pays for Claude or Codex, or wants a terminal. For them the
editor hosts a **local agent**: the managed llama.cpp runner and pinned model
described in [`local-ai-architecture.md`](local-ai-architecture.md), with no
account, key or usage cost.

The two are not separate systems. The local agent is another consumer of the
foundation below: it reads the same generated documents, changes the same
files, and its work is judged by the same `sindri project check` and headless
run. The editor only hosts the loop and shows the diff. So:

- every improvement to the docs and checks helps Claude, Codex and the local
  model at once;
- there is one trust model — whatever an agent proposes, the checks decide;
- the checks are what make a small model usable. Decay repair already works
  this way: draft, compile, at most two repairs, and only an answer that
  compiles is shown. A project-wide check widens that loop from one script to
  a whole project.

### What each agent does

A 7B-class local model cannot do what an external agent does: explore a
project, plan across many files, and recover from its own mistakes. So the
local agent is deliberately narrow, as `local-ai-architecture.md` argues —
small context, one focused task, typed output:

| Task | Local agent | External agent |
| --- | --- | --- |
| Fix a Decay compile error | ✅ Exists today | ✅ |
| Write one script from a description | Next | ✅ |
| Add or change components on the selected entity | Planned, through the [proposal protocol](ai-authoring-protocol.md) | ✅ Edits the scene file |
| Explain a check failure | Planned | ✅ |
| A feature across several files ("make the player dash on Shift, with a cooldown and a trail") | ❌ | ✅ |
| Explore and refactor a project | ❌ | ✅ |

The local agent's changes are always shown as a diff and applied only on
acceptance, as one undo step. It does not become a general chat assistant; a
task that needs one is the external agent's.

### Sequencing

The local agent gets better as the foundation does, so its new work follows
the foundation rather than running beside it: writing a script from a
description needs the checks to grade it, and entity edits need the scene
checks the proposal protocol's validator would otherwise duplicate.

A note on Unity, since it prompted this: Unity is pursuing two things at once.
Unity Spark is a browser-based prompt-to-game tool for Google's Playground,
aimed at people who do not write code; Sindri should not chase that. Separately,
Unity ships a CLI with an MCP server and an official skill plugin for Claude
Code and Codex. That second bet is the one this direction resembles — with the
difference that Sindri's project files are already readable enough that the
mutation half of it is unnecessary.

## 1. Docs an agent can trust

An agent knows nothing about Sindri or Decay from training. Unity can lean on a
decade of tutorials in every model's memory; Sindri has only what is in the
project and the repository. That makes introspection the substitute for
popularity, and it makes a wrong document worse than none.

**Generated first.** What can be derived from the engine is derived, by
`sindri-capabilities`, and a test fails when it goes stale:

| Question | Answered by | State |
| --- | --- | --- |
| What may I author, and what does each field mean? | `docs/generated/sindri-capabilities.json` | Exists |
| What gameplay code may I write? | `docs/generated/decay-api.json` / `.md` | Exists |
| What is the Decay language? | `docs/generated/decay-language.json` | Exists |
| What shape is a `.scene` or `.prefab` file? | JSON Schema generated from the component registry | Missing |
| What may a Weave stylesheet say? | A generated Weave reference | Missing |

The JSON Schemas are the cheapest large win: an agent reads them, and a person
editing a scene in VS Code gets validation and completion from the same file.

**Hand-written only for what cannot be generated:** where things belong
(gameplay in Decay, interface in Weave, engine capability in Rust), the
distinction between authored properties and live state, which files are
generated or canonical, and the workflow below. These live in `AGENTS.md`,
[`decay-agent-guide.md`](decay-agent-guide.md) and the subsystem contracts, and
point at generated documents rather than restating them. The legacy editor's
hand-written `ENGINE_REFERENCE.md` is the failure this rule exists to prevent.

**Shipped with projects, not only the repository.** A game made with Sindri
does not contain this repository's `AGENTS.md`. A new project needs its own
short agent guide — the workflow and where to find the generated documents —
written by `sindri new`, and packaged as a skill for the agents that support
one. It stays thin: discovery and checks, not API facts.

## 2. Checks an agent can run

An agent that cannot tell whether its edit is valid hands its mistakes to a
person. Decay is already covered: `decay-lsp --check --json` compiles scripts
against the real host environment with structured diagnostics. Everything else
is checked only when the editor or a game loads it.

The target is one command over a whole project:

```bash
sindri project check --format json
```

It reports, with a stable `code`, the file, and the JSON path or source span:

- scene and prefab structure, through `SceneDocument::validate` and
  `ComponentSchemaRegistry::validate_scene` — both already exist;
- unknown components, with the nearest registered name;
- references that do not resolve: assets, prefabs, parent and entity ids, and a
  `sindri.script` whose `script` names no declared container in its source;
- Weave stylesheets that do not parse or name nothing;
- every Decay script, through the existing checker;
- `sindri.toml` itself.

It follows [`cli-conventions.md`](cli-conventions.md): JSON is the contract,
the human form is rendered from it, and each diagnostic is raised by the crate
that found the problem rather than assembled by the CLI.

## 3. A way to see the result

A scene can pass every check and the dash still feels wrong, the sprite can sit
behind the floor, and the interface can overlap. The agent needs to run the
game and look.

Every game in this repository already carries a test that opens it, compiles
its scripts and plays a scripted run to its goal, and several games have
`*-capture` binaries that render frames offscreen. Generalizing those gives:

```bash
sindri play --headless --frames 600 --format json            # log, script prints, errors, per frame
sindri play --headless --frames 600 --capture 60,300,600     # PNG frames an agent can look at
```

with scripted input, so an agent can press Shift and see whether the player
dashed. The fixed, deterministic step makes a run reproducible, which is what
makes it a check rather than an anecdote.

## The editor and the agent at the same time

An agent will edit files while the editor has them open. Scripts, textures,
prefabs and Weave styles are already watched and reloaded. The open scene is
not: reloading is manual and discards unsaved edits along with their history
(see [`editor-architecture.md`](editor-architecture.md)).

The editor should reload a scene changed on disk when it has no unsaved edits,
and when it does, say so and offer the choice rather than letting the next save
overwrite the agent's work. That is the one editor change this direction
requires.

Because canonical serialization reproduces an untouched scene byte for byte,
the editor and an agent already produce diffs that do not fight. That property
is now an external contract and must stay true.

## The file format is now public

Scenes already declare `format_version`, and older formats are migrated rather
than refused (see [`versioning.md`](versioning.md)). What changes is who relies
on it: hand-written and agent-written files are now expected, not tolerated. A
format change is therefore a public change with a changelog entry, the
generated schemas carry the version they describe, and a migration that
cannot be done safely says what it needs rather than guessing.

## What is deliberately not built

- **A mutation CLI or MCP tool set.** `sindri scene edit --ops` in
  `cli-conventions.md` remains the agreed shape if one is ever needed, but it is
  not built ahead of evidence.
- **A live editor bridge.** Unity shipped an MCP server inside its editor and
  then deprecated it in favor of its CLI. Reading live state is the one thing
  files cannot give, and headless play covers most of it. Revisit only if the
  benchmark below shows agents failing for lack of it, and then route changes
  through the existing proposal protocol so they stay previewable and one undo
  step.
- **A general in-editor chat assistant.** The local agent stays narrow.

**The one likely exception** is data that is text but unpleasant to edit
positionally: flat tilemap cell arrays, voxel data, quaternions, sprite-sheet
slices. A few narrow commands — painting a tilemap region, setting a rotation
from Euler angles — may earn their place. Add each one when the benchmark shows
agents failing at that edit, not before.

## Measuring it

"Agent-native" is a claim, so it gets a test. `tools/ai-authoring-eval`
measures models against the proposal protocol; it gains a second mode that
gives an unmodified external agent a fresh project and a fixed task — "make the
player dash on Shift with a cooldown", "add a coin that plays a sound when
collected" — and grades the result with `sindri project check` and a headless
run. The pass rate, per agent, is the number that decides what to build next.

## Order of work

1. `sindri project check` with JSON output, over scenes, prefabs, Weave, Decay
   and `sindri.toml`.
2. JSON Schemas for scenes and prefabs, generated from the component registry.
3. `sindri play --headless` with structured logs, scripted input and capture.
4. The editor reloads a scene changed on disk.
5. A per-project agent guide written by `sindri new`, packaged as a skill.
6. The external-agent benchmark, after which its failures order the rest.
7. The local agent on the same checks: writing one script from a description,
   then selected-entity edits through the proposal protocol, then explaining a
   check failure. The benchmark gains a local-model column for the tasks the
   local agent is meant to do.

These are tracked under *Agent-native authoring* in `ROADMAP.md`.
