# Sindri Mobile: product and technical direction

Status: product direction and proposals, not an implementation or a claim of Android support.

## The idea

Sindri Mobile is a native Android game editor for making a complete game on a phone or tablet: create a project, build a world, author gameplay, test, debug, and export a playable game without a computer. It is part of Sindri, not a separate engine or a remote control for the desktop editor. Offline creation is the intended baseline; services must not be required to edit or play a local project.

Desktop and mobile work on compatible Sindri projects and share engine systems. Their interfaces are adapted to their devices. Starting on Android and continuing on desktop, or the reverse, should be possible without conversion into a separate mobile format or losing data the other editor does not understand.

The first complete creator journey should target a small 2D game. This is a first milestone, not a permanent restriction on 3D or on the engine's capabilities. Supporting existing projects and approaching desktop capability parity are continuing goals, measured per feature rather than assumed from shared dependencies.

## The experience

The interactive Sindri Mobile prototype is the visual and interaction reference. It uses Lamplighter's real art and the desktop editor's Forge palette and icon vocabulary to explore a canvas-first editor. It demonstrates workflows; its simulated behavior is not evidence that those capabilities are implemented.

Preserve what makes it compelling:

- The world is the focus. The interface is calm and makes room for creation.
- Contextual tools, bottom sheets, and clear selection surfaces reveal the next useful action without covering the canvas permanently.
- Controls are designed for touch, not miniature desktop panels; common tasks require neither a mouse nor a keyboard.
- Create, play, pause, inspect, and return to editing form one fast, understandable loop.
- Forge colors, typography, spacing, icons, and interaction states form a consistent design system, adapted to mobile rather than replaced with generic controls.
- Portrait and landscape work on real devices; tablets and external keyboard, mouse, or stylus are useful additions, not prerequisites.
- Accessibility, readable text, sufficient contrast, usable touch targets, system insets, keyboard handling, and screen-reader semantics are part of the design.

The app is built to this design first and given functionality second; [design.md](design.md) tracks each screen against the prototype and what is wired. Use the prototype as an acceptance reference: retain its assets and interaction intent, document its states and gestures, and compare implementation captures with it on actual phones. Exercise dense scenes and long names, not just curated sample content. The prototype is tracked in [`prototype/`](prototype/index.html). Open `mobile/prototype/index.html` in a browser at a phone-sized viewport; it loads Lamplighter's textures directly from `games/lamplighter/assets/textures/` rather than keeping copies. It needs network access for its React, Babel, and Google Fonts dependencies. `prototype/support.js` is the generated runtime from the design tool that exported the prototype: do not edit it by hand. Treat the prototype as a design reference. It is not part of the engine, editor, or build, and passing through it does not test any implementation.

## Full creation journey

A creator should be able to:

1. Install and open the app, create a project from a blank starting point or a template, and find their recent projects.
2. Import and organize assets, preview them, slice sprites and animations, create prefabs, and use appropriate creation tools as they become available.
3. Create and navigate scenes; paint tilemaps; place, select, transform, and inspect objects; configure cameras, components, lighting, physics, audio, and UI.
4. Author gameplay with visual Decay, using variables, events, functions, branches, and reusable logic; use text Decay where appropriate.
5. Play the actual game on the device, pause and inspect it, diagnose errors, and keep or discard edits made during Play through the same semantics as desktop.
6. Save and reopen safely, undo and redo meaningful changes, and transfer or synchronize a project without silent data loss.
7. Export and share a playable game. Browser builds and Android packages are both candidates; the first supported export target and on-device versus remote build requirements remain to be established. Editing and playing a game on Android, exporting a web game, and building a distributable Android game are distinct capabilities.

A first complete 2D game should exercise this journey end to end, including a title and UI, interaction, sound, multiple scenes, saving progress, and a shareable build. Define concrete acceptance criteria and supported device profiles before implementation.

## Visual Decay

Visual scripting is a first-class way to write Decay, not a separate, lesser gameplay language. The desired model resembles Blueprint-style graphs:

- Execution wires show the order in which events and actions run.
- Separate, typed value wires carry numbers, booleans, text, entity references, and other supported Decay values.
- Nodes represent real Decay constructs and registered Sindri host APIs. Events, branches, variables, functions, loops, and reusable graph sections are part of the intended scope.
- Invalid connections and missing references produce actionable diagnostics; compatible choices are easy to discover.
- During Play, users should be able to follow execution and inspect values without leaving the graph, subject to implementation and performance validation.

For example, an interaction event flows to a branch. A value connection supplies whether Wick has enough light. The true path lights a lantern and opens a bridge; the false path displays a message. This is illustrative behavior, not a claim that these specific Decay calls exist.

Touch interaction is a product requirement: tap a socket to search for and connect compatible nodes; allow dragging as an alternative; provide pan, zoom, fit-to-graph, navigation between events, and collapsible named groups. Avoid making fine wire dragging mandatory. A simple contextual When/Then flow may offer a shortcut into the same underlying graph, so it can grow into a full graph without being rewritten.

### Representation and code

**Decided (2026-11-01): visual scripting is Decay, but visual.** The `.decay` file
is the only thing stored. There is no graph asset beside it and nothing is
converted: the blocks are how an editor shows a script, and a block edit is a
change to that script's text. A script written by hand on desktop opens as
blocks on the phone, and one edited as blocks reads on desktop exactly as if it
had been typed there.

This replaces the earlier proposal of a versioned graph asset compiled to Decay.
How it works, in `sindri_authoring::visual_decay`:

- Blocks are read through Decay's own parser (`decay-syntax`). Handlers are
  "When …" blocks, with the lifecycle taken from `sindri_decay::LIFECYCLE`.
  Statements are blocks, and expressions sit in slots. Comments between
  statements, and above values and handlers, are shown. Anything with no block
  shape yet is shown as its exact source, so nothing is hidden.
- Every node carries its span in the file. An edit replaces exactly the text of
  the slot it changed, then the result is parsed and refused if it no longer
  reads. Every other byte, comments and formatting included, stays as it was.
  A script with syntax errors is fixed as text before its blocks edit.
- The same reading is drawn as Blueprint-style nodes (`visual_decay::graph`):
  handlers are event nodes, statements a chain joined by execution wires,
  and calls and operators value nodes wired into pins. Literals sit on their
  pins.
- Node positions are the one thing a script does not hold. They live beside
  it in `<script>.layout`, never in it (`visual_decay::layout`). A stored
  position finds its node by its place in the code and by a fingerprint of
  its text, so moving code on desktop keeps nodes where they were left. A
  node nobody placed is laid out from the code. Fields a newer editor wrote
  are kept, and a newer file is not overwritten.

- Structure changes from a node's menu (`visual_decay::change`): delete,
  move, add after or first inside. Whole lines are cut, swapped or added at
  the code's own indentation, with a statement's comment travelling with it.
  The result must parse and must not add a problem the checker
  (`decay-semantic`, against the project's scripts and the engine's API)
  finds.

The look follows Unreal's Blueprints (execution and value wires, Branch with
True and False). The interaction follows Unity's Visual Scripting (flow and
value ports), and a searchable finder for engine calls is next. Wiring values
between nodes and following execution during Play build on this. Each new kind of edit is a span edit checked the same way. The round
trip is tested on every script in the repository.

## Architecture direction

One engine and project model, with device-appropriate authoring surfaces:

- Reuse Sindri's scene and project formats, component schemas and validation, assets, rendering, physics, Decay host, and `sindri-runtime::Session`.
- Route editor changes through the same checked, undoable commands and preserve the existing Play, stop, and run-edit semantics. Do not maintain a second world model or duplicate fixed-step gameplay.
- Incrementally separate reusable authoring logic from desktop UI where current code couples them. Establish a real dependency boundary before adding a crate; engine crates do not depend on an editor.
- Adapt Android lifecycle, graphics surfaces, input, audio, file access, and app state behind platform-specific boundaries. A shared crate alone does not establish Android support.
- Maintain capability evidence per platform. A feature earns parity only when it is implemented and exercised on that surface.

The desktop editor uses Rust, egui, winit, and wgpu, with the engine viewport and editor sharing a graphics device. A touch-specific Rust/egui interface could reuse more of that stack. A Kotlin/Jetpack Compose shell around the Rust editor and engine could offer Android-native controls while embedding a real Sindri viewport, but adds a bridge and graphics integration. **Decided (2026-10-31):** Compose plus Rust, on a Pixel 8 reference device, with Android 10 (API 29) as the minimum. The app is in `android/` and its native half, the `sindri-mobile` crate, is in `native/`. GPU surface ownership, lifecycle, input, text and IME behavior, accessibility, performance and build complexity still have to be proven on the device; [first-slice.md](first-slice.md) tracks that.

Android storage is another explicit platform boundary. A selected document tree need not behave like an ordinary filesystem path. Evaluate app-owned projects with import/export and direct document-provider access, including permission persistence, provider failures, backups, conflict handling, and recovery from interruption. Never silently overwrite a changed external project. Cloud sync is optional future work and needs its own conflict design.

Game export also needs a separate feasibility investigation: toolchain size and availability, on-device versus remote builds, signing and keys for Android packages, web asset packaging, offline requirements, and safe sharing. No export target is promised by this document.

## Proposed first technical slice

Prove the design and architecture together on a real Android phone. Progress, evidence, and blocking decisions are tracked in [first-slice.md](first-slice.md).

1. Open a real Sindri project and display a scene using the real engine renderer.
2. Select and move an entity with touch; edit one schema-validated property in a prototype-style inspector.
3. Undo and redo, save, restart the app, and reopen the project. Open the resulting project on desktop and verify the same data.
4. Start and stop a real `Session`, exercise touch input, and check that the scene returns to the expected state.
5. Exercise background/resume, graphics surface recreation, keyboard/insets, and an interrupted save; record device, performance, and limitations.

Follow with vertical slices that lead to the complete 2D game journey: assets and prefabs, tilemaps, a minimal real visual Decay graph, debugging, UI and sound, scene transitions and saving, then a proven export path. Ordering is provisional: findings from the first slice and export feasibility may change it.

## Decisions and open questions

**Established product direction**

- The Android app is Kotlin and Jetpack Compose around the Rust engine; the Pixel 8 is the reference device and API 29 the minimum.
- For the first slice, projects are app-owned copies in the app's private storage.
- Android is a standalone, start-to-finish game creation environment.
- The mobile design follows the supplied prototype's canvas-first, touch-first approach.
- Projects and engine systems are shared with desktop; visual Decay uses connected execution and typed value nodes.
- A complete small 2D game is the first end-to-end target, with broader capability to follow.

**To prove or decide**

- Graphics integration and device behavior of the chosen Compose plus Rust host, proven on the Pixel 8.
- Exact visual graph representation, compilation, diagnostics, debugging, migrations, and relationship to handwritten Decay.
- Project storage, safe transfer and recovery, and optional synchronization.
- First distributable export target and where its toolchain runs.
- Scope, interaction design, and accessibility of the first complete 2D creation journey.
- Which desktop capabilities need shared authoring extraction or Android-specific replacements; track real gaps rather than claiming blanket parity.

## Related repository documents

- [Editor direction](../docs/editor-direction.md) and [editor architecture](../docs/editor-architecture.md)
- [Project format](../docs/project-format.md) and [component schema registry](../docs/component-schema-registry.md)
- [Decay direction](../docs/decay-direction.md), [language](../decay/LANGUAGE.md), and [language-server modernization](../docs/decay-lsp-modernization.md)
- [Capability parity](../docs/parity.md), [editor capabilities](../docs/capabilities/editor.md), and [feasibility](../docs/FEASIBILITY.md)

The existing feasibility document excludes native mobile from the first release. This overview proposes a later mobile product direction; it does not silently change that release gate or claim the Android platform is already supported.
