<p align="center">
  <img src="https://github.com/vardirhq/sindri-engine/blob/main/docs%2Ffile_000000005bd882118094c913acb7257c.png" alt="Sindri Engine" width="680">
</p>

<p align="center"><strong>A Rust-powered 2D + 3D game engine with a native visual editor, the Decay gameplay language, and responsive Weave UI.</strong></p>

Sindri Engine is a from-the-foundation rebuild of the original Sindri. It is built around one runtime model, one scene format, one asset pipeline, and one renderer targeting native desktops and WebGPU browsers. Games are meant to be authored through the editor, Decay, and Weave; Rust is the implementation language underneath rather than a requirement for ordinary gameplay work.

> **Status:** pre-alpha and under active development. Public APIs, Decay, Weave, editor workflows, and serialized formats may change while the engine is proven through real games.

## The authoring model

Sindri is deliberately more than a Rust framework. Its primary authoring stack is developed together:

```text
                 Sindri Editor
                      |
              live World / Scene
               /             \
          Decay               Weave
        gameplay                UI
               \             /
                 Sindri Engine
                      |
                     Rust
```

- **Editor** authors the same world and scene representation used by the runtime.
- **Decay** is the statically typed gameplay language: game state, behaviour, events, input, physics, spawning, animation, audio, UI interaction, persistence, and engine-facing logic live here rather than in ordinary Rust gameplay code.
- **Weave** is Sindri's responsive presentation system. Styles and composition resolve into ordinary Sindri UI data and can adapt one authored interface across different viewport shapes.
- **Rust** implements the engine, renderer, platform layer, editor, and language/runtime integrations beneath those authoring surfaces.

The goal is that somebody can build and ship a Sindri game without needing to understand the Rust implementation underneath.

## What Sindri is trying to be

- **2D and 3D in one engine**, without forcing ordinary 2D work through needlessly complicated 3D abstractions.
- **A real native visual editor**, not a separate mock representation of the runtime.
- **A purpose-built gameplay language.** Decay is Rust-inspired, statically typed, project-aware, and designed around game authoring rather than general-purpose systems programming.
- **Responsive game UI through Weave**, with presentation rules separated from gameplay and authored for multiple viewport shapes.
- **Native and web from the same foundation**, using Rust, `wgpu`, `winit`, WebGPU, and WASM where appropriate.
- **Portable, review-friendly projects** with canonical scene files, logical asset IDs, reusable prefabs and profiles, and no hidden sidecar identity system.
- **Actionable diagnostics and tooling**, including changed-scope preflight, structured CI diagnostics, and project-aware Decay checking.
- **Features proved by games.** An API, component, editor control, or language feature is not considered complete merely because it exists.

Sindri is not trying to win a feature-count contest with mature engines. The goal is a coherent engine and authoring environment that is pleasant to build games with and difficult to accidentally lie about.

## Games prove the engine

Sindri is developed vertically: runtime capability, editor authoring, Decay/Weave access, and a real project evolve together where a feature applies.

- **Causeway** is the flagship showcase: a voxel builder that pushes the engine's world, rendering, authoring, and asset capabilities by trying to become a game worth playing rather than a disposable demo.
- **Orbital Last Stand** is the forcing function: a recreation of a complete game built through the editor and Decay. Gaps it exposes are closed as general Sindri capabilities rather than game-specific hacks.
- **Genre showcases** are small complete games that prove ordinary workflows. The platformer exercises the plain 2D path; **Scorchball** exercises local multiplayer, player-slot input, prefab-driven players, animation, typed Decay communication, events, state, and increasingly game-like AI and mechanics.
- **Feature examples and labs** isolate larger systems such as cameras, Weave, graphics, and voxels when isolation is the honest way to test them.

The exact proof status for each capability lives in [`docs/parity.md`](docs/parity.md), and the detailed evidence lives in [`docs/capabilities.md`](docs/capabilities.md).

## Decay

Decay is Sindri's Rust-inspired, statically typed gameplay language. The language implementation lives in its own workspace and is engine-agnostic; `sindri-decay` is the one-way binding layer that exposes Sindri's world and host APIs.

```rust
script Player {
    @export
    var speed: f32 = 6.0;

    fn update(dt: f32) {
        let movement = Input.axis("ArrowLeft", "ArrowRight");
        this.transform.position.x += movement * speed * dt;
    }
}
```

Decay now goes well beyond isolated callbacks and stringly host calls. Current language and Sindri integration include:

- typed editor-visible `@export` fields;
- `Vec2` and `Vec3` values and vector operations;
- typed script references, so scripts can find and communicate with other script types without property-name strings;
- typed script messages;
- project-wide typed events with `event`, `emit`, and `on`;
- typed shared project state;
- arrays, maps, bounded loops, entity/prefab/profile references, and operation budgets;
- transforms, sprites, animation, input, physics/collisions, spawning, UI, audio, effects, persistence, grids, and other checked host APIs;
- project-aware checking through `decay-lsp`, including structured diagnostics and completion work that is evolving alongside the language.

A misspelled host member, wrong event payload, invalid script relationship, or incompatible type should be caught as a Decay diagnostic rather than becoming a mysterious runtime fallback.

Decay intentionally does **not** inherit Rust's ownership, borrowing, lifetimes, `unsafe`, or systems-programming machinery. Rust inspires the syntax, static typing, and tooling quality; Decay remains focused on gameplay.

See [`decay/LANGUAGE.md`](decay/LANGUAGE.md), [`docs/scripting.md`](docs/scripting.md), and [`docs/decay-direction.md`](docs/decay-direction.md).

## Weave

Weave is Sindri's responsive UI/presentation system. It exists so game interfaces do not have to become piles of viewport-specific coordinates or gameplay code manually rearranging rectangles.

Weave presentation rules resolve into ordinary Sindri UI data, allowing one authored interface to recompose for different viewport shapes while remaining part of the same scene/runtime model. The system is developed alongside Decay and the editor rather than as a separate UI stack bolted on afterward.

The responsive showcase and current language/limitations are documented in [`docs/weave.md`](docs/weave.md).

## What works today

Sindri is pre-alpha, but the working surface is already broad. Highlights include:

### Engine and runtime

- deterministic lifecycle and fixed-step simulation;
- generation-checked entities, hierarchies, and composed 2D/3D transforms;
- canonical, versioned scene serialization and reversible world commands;
- keyboard, pointer, touch, and local gamepad/player-slot input;
- logical asset IDs, asynchronous loading, manifests, and hot reload;
- native-windowed, headless, and WebGPU browser hosts;
- textured/layered sprites, sprite sheets and animation;
- 3D meshes, perspective/orthographic cameras, lighting and bounded directional shadows;
- tilemaps, grids, occupancy, walls, placement validation, and pathfinding;
- voxel world storage, generation/streaming, meshing, and editor/runtime rendering work;
- Rapier2D physics with authored bodies/colliders, sensors, masks, events, velocity, and impulses;
- native/browser audio;
- prefabs and reusable profile data;
- responsive Weave presentation;
- static web export;
- deterministic rendering/capture paths exercised in CI.

### Editor

The native editor works on the same scene/world model as the runtime. It supports scene/project browsing, hierarchy authoring, transforms and component editing, asset selection, sprite-sheet slicing and animation preview, Decay `@export` fields, undo/redo, Scene and Game views, camera navigation and selection, Play/Pause/Stop against the real engine lifecycle, console output, preferences/layout persistence, and asset/script hot reload.

This summary is intentionally not exhaustive. [`docs/capabilities.md`](docs/capabilities.md) is the evidence-based inventory; [`docs/parity.md`](docs/parity.md) records Engine / Editor / Decay / proof status and the important gaps against a conventional game-engine baseline.

## Architecture

Sindri keeps engine concerns in focused Rust crates while Decay remains its own workspace.

```text
                         Sindri Editor
                              |
                       live World / Scene
                      /       |        \
                 Assets    Rendering   Decay host
                    |          |           |
              sindri-assets  sindri-render  sindri-decay
                               |           |
                           sindri-gpu   decay-runtime
                               |           |
                              wgpu      decay-ir
                               |           |
                       Native / Web    decay-semantic
                                           |
                                      decay-syntax
```

The important boundary is intentional:

```text
decay/*                 knows the language, not Sindri
crates/sindri-decay     knows both halves
sindri engine crates    do not depend on Decay
```

That keeps the language embeddable and prevents Sindri implementation details from leaking into Decay core. The renderer-independent engine core likewise has no dependency on a window, GPU, browser, editor, physics engine, scripting runtime, or async executor.

## Native and web

The web is a first-class runtime target rather than a separate edition. Native and browser hosts share the same engine concepts, scene extraction, assets, GPU abstraction, renderer, Decay gameplay model, and Weave presentation policy.

Static WebAssembly/WebGPU exports are exercised through repository projects and browser smoke tests. A browser needs WebGPU support.

## Important gaps

Sindri is still pre-alpha. Among the meaningful gaps and incomplete areas are:

- richer asset import, preview, and project-settings workflows;
- deeper prefab authoring/linked-instance workflows;
- polished external Decay authoring, formatting, debugging, and the remaining language-server/IDE features;
- richer Weave editor tooling and accessibility work;
- native packaging/release tooling beyond the current web exporter;
- a mature 3D content pipeline, including broader materials, model import, skeletal animation, and more complete lighting/physics features;
- the accumulated stability, documentation, platform coverage, and real-world usage expected of a mature engine.

For the current outside-in audit rather than an aspirational feature list, see [`docs/parity.md`](docs/parity.md).

## Try the editor

The workspace currently requires Rust 1.95.

```bash
cargo run --package sindri-editor
```

The repository includes games, feature examples, fixtures, deterministic captures, and regression tests that exercise the same implementations used by the editor and runtime.

## Development

Engine/editor workspace:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace --all-features
cargo check --workspace --all-features --target wasm32-unknown-unknown
```

Decay is intentionally a separate Cargo workspace:

```bash
cd decay
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For changed Decay scripts, use the typed project-aware preflight rather than treating syntax alone as proof:

```bash
cargo run --quiet --package decay-lsp -- --check path/to/script-or-project
```

Repository guidance and the complete required checks live in [`AGENTS.md`](AGENTS.md).

## Documentation

Start here:

- [`docs/parity.md`](docs/parity.md) — outside-in Engine / Editor / Decay / proof audit and important gaps
- [`docs/capabilities.md`](docs/capabilities.md) — detailed evidence for what demonstrably works
- [`decay/LANGUAGE.md`](decay/LANGUAGE.md) — Decay language reference
- [`docs/scripting.md`](docs/scripting.md) — Decay-to-Sindri host surface
- [`docs/decay-direction.md`](docs/decay-direction.md) — why Decay exists and how its authoring model evolved
- [`docs/weave.md`](docs/weave.md) — responsive UI/presentation model
- [`docs/project-format.md`](docs/project-format.md) — project structure and `sindri.toml`
- [`docs/diagnostics.md`](docs/diagnostics.md) — diagnostics and CI/preflight model
- [`ROADMAP.md`](ROADMAP.md) — development plan
- [`AGENTS.md`](AGENTS.md) — architecture, contribution, proof, and verification rules for coding agents

Subsystem contracts live under `docs/` beside the code they describe.

## Why Sindri?

Sindri's central bet is that a Rust engine does not have to make Rust the everyday game-authoring experience.

The editor owns the world. Decay owns gameplay. Weave owns responsive presentation. Rust provides the engine underneath. Those surfaces are developed together and proved through actual games rather than treated as independent checkboxes.

The project has a simple rule: **working code beats plausible architecture**. A capability is not complete because an API exists or an editor control is visible; it is complete when the relevant authoring surface can use it and a real project proves it.

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) for checks, conventions, and commit style, and [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) for participation guidelines. The dependency policy is documented in [`docs/dependency-policy.md`](docs/dependency-policy.md).

## License

Dual-licensed under either the [Apache License 2.0](LICENSE-APACHE) or the [MIT license](LICENSE-MIT), at your option.

Unless you state otherwise, any contribution intentionally submitted for inclusion in Sindri is dual-licensed as above, with no additional terms or conditions.
