# The End of Pre-Alpha

## Sindri's passage from experiment to game engine

> *Pre-alpha ends when Sindri stops asking whether it can become a game engine and starts proving that it is one.*

There will come a commit after which the words **PRE-ALPHA** disappear from Sindri.

This document defines that commit.

Not because every feature imaginable has been implemented.

Not because every bug has been slain.

Not because the editor has achieved enlightenment.

And certainly not because somebody changed a version number and hoped nobody would ask difficult questions.

Sindri leaves pre-alpha when its foundations are strong enough that another developer can download it, build a real game, publish that game, and reasonably expect the engine beneath them to hold.

The experiments end here.

The engine begins.

## I. The engine must be whole

Sindri does not need everything.

It does need enough.

The major systems required to build complete games must exist as coherent, documented, tested parts of one engine.

### Rendering

Sindri must be capable of presenting complete 2D games and a credible 3D baseline.

Rendering must behave consistently across supported targets. Cameras, materials, sprites, meshes, environments, effects and the surrounding rendering architecture must feel like parts of one system rather than artifacts from twelve separate expeditions into WebGPU.

### Physics

**The Physics Patch must be complete.**

2D physics must provide the machinery expected of a serious game engine: rigid bodies, colliders, materials, forces, rotation, contacts, CCD, joints, motors, character movement and accelerated spatial queries.

3D must have a useful physics foundation rather than merely possessing a third coordinate for decorative purposes.

Voxel terrain must participate in real collision.

The Physics Showcase must demonstrate the system as something worth playing with, not merely a collection of rectangles politely falling onto other rectangles.

The Physics Showcase is `examples/physics`, the Physics Playground; [`physics-playground.md`](physics-playground.md) records what it covers and the gaps it found.

### Decay

Gameplay belongs in Decay.

By Alpha, this must be a practical architectural rule rather than an aspiration written sternly in `AGENTS.md`.

A developer must be able to implement meaningful game logic without modifying Sindri's Rust source.

Decay must have coherent APIs, useful diagnostics, documentation and authoring support. When something is wrong, the engine should explain what is wrong.

"Parse failed somewhere around here, good luck" does not qualify.

### Weave

Sindri needs a real UI system.

Games must be capable of building menus, HUDs, settings screens, inventories, dialogs and responsive interfaces without fighting the engine.

Layout, input, focus, navigation, text, clipping, scrolling, styling and animation must form a coherent whole.

Mouse, keyboard, controller and touch must be considered first-class citizens where appropriate.

### Audio

Games make noises.

Sindri must therefore also make noises.

Reliably.

Audio playback, spatial behavior where applicable, volume control, buses or equivalent organization, looping and runtime control must be sufficient for complete games.

### Scenes, prefabs and assets

Large games are not one scene.

The scene and prefab systems must survive reuse, nesting, duplication, references, serialization, save/reopen and runtime spawning without developing opinions about causality.

Assets must have predictable identity and lifecycle.

The editor must understand what the project contains. The runtime must understand what the editor produced. Neither should require divine intervention when a file moves.

## II. The editor must become a product

Opening Sindri Alpha should not feel like opening an internal debugging application accidentally shipped to the public.

The editor must have a coherent workflow from beginning to end:

**Create Project -> Build -> Play -> Debug -> Save -> Reopen -> Export**

A developer should not need Cargo. They should not need to compile Sindri. They should not need to know which Rust crate owns scene synchronization. They should absolutely not need to clone the engine repository just to make a platformer.

The editor must provide dependable:

- project creation
- scene creation and management
- hierarchy editing
- inspector editing
- component authoring
- prefab workflows
- asset browsing
- undo and redo
- Play and Stop
- useful diagnostics
- project settings
- saving and reopening
- export

Crashes, corrupt saves and destructive editor operations are release blockers.

Mild ugliness is negotiable.

Eating someone's game is not.

## III. Sindri must survive scale

Tiny examples prove capabilities.

They do not prove an engine.

Before Alpha, Sindri must face something intentionally unreasonable.

### The Scale Trial

Construct a synthetic game substantially larger than the examples used during development.

It should contain enough scenes, entities, assets, prefabs, scripts and persistent state to expose architectural weaknesses that small projects politely conceal.

Think:

- roughly 100 rooms or equivalent world partitions
- multiple environments or biomes
- reusable enemies and objects
- deeply reused prefabs
- persistent switches, doors and collected objects
- substantial asset libraries
- transitions between areas
- save/load state
- shared UI
- audio zones
- enough content to make the project panel regret its career choices

Most rooms may be generated or templated. The purpose is not to secretly develop Hollow Knight before Alpha.

The purpose is to discover whether Sindri would survive someone trying.

Measure editor startup, project loading, scene switching, runtime transitions, memory consumption and recovery, prefab propagation, Decay checking, asset discovery, save/reopen correctness, export time and size, and editor responsiveness.

Sindri does not need infinite scale.

It needs evidence that its architecture does not collapse immediately after the twentieth room.

## IV. The Great Code Audit

Then we clean the forge.

Before Alpha, the entire repository receives a deliberate code-quality audit.

Not an AI prompt reading:

> "Make code better."

Civilizations have fallen for less.

The audit proceeds subsystem by subsystem. Every major area is examined for:

- architectural responsibility
- readability
- unnecessary complexity
- duplicated implementations
- inconsistent APIs
- ownership and lifetime clarity
- error handling
- dead code and obsolete compatibility paths
- performance hazards
- unnecessary allocation and cloning
- test quality and missing tests
- documentation quality
- unsafe code and concurrency assumptions
- dependency hygiene
- native/WASM divergence

The existing **600-line Rust source limit remains absolute**.

Approximately 400 lines remains the preferred target where sensible. Functions should remain understandable without requiring excavation equipment.

Complex algorithms must explain their invariants and reasoning. Public APIs must explain their contracts. Comments should explain **why**.

```rust
counter += 1; // Increment counter
```

will not be regarded as a meaningful contribution to human knowledge.

Where an audit discovers a rule that can reasonably be enforced automatically, CI should enforce it.

The repository must carry its own institutional memory.

That matters particularly because Sindri has been constructed with enormous assistance from AI systems capable of producing individually excellent modules while occasionally possessing the architectural memory of a startled pigeon.

Local quality is not enough.

The whole must make sense.

## V. The engine must build games

Sindri's examples are valuable.

They are no longer sufficient evidence.

Before Alpha there must be at least **one complete game** built with Sindri.

Not a technology demonstration.

Not a laboratory.

Not a rectangle capable of jumping.

A game.

It needs a beginning, gameplay, progression or meaningful state, UI, audio, saving where appropriate, failure/success states where appropriate, enough content to expose real development friction, and an exported build someone else can play.

The game does not need to be large.

It does need to be finished.

Every time developing it requires changing Sindri itself, that friction should be examined. Sometimes the engine needs another feature. Sometimes its API is wrong. Sometimes the documentation is wrong. Sometimes the game developer is attempting something profoundly questionable.

All four are useful discoveries.

## VI. Strangers must touch it

This one is non-negotiable.

Someone who did not build Sindri must use Sindri.

Preferably several someones.

They receive the downloadable editor, documentation, examples and starter templates.

They do **not** receive a Sindri developer standing behind them explaining which button secretly means what.

Their mission:

> Create a small game and export it.

Observe where they struggle.

Every question beginning with "How was I supposed to know that...?" is evidence. Every undocumented workaround is evidence. Every trip into the engine repository is suspicious. Every engine modification required for ordinary game development is extremely suspicious.

The engine must survive contact with people who do not already know how it works.

Software has historically found this requirement deeply offensive.

It remains a requirement.

## VII. Sindri must ship

Alpha cannot require building the engine from source.

There must be versioned downloadable releases.

At minimum, Windows must have a normal downloadable editor build. Linux must have a practical distributable build. Additional platforms may follow as support matures.

Projects must be exportable without reconstructing Sindri's internal build pipeline by hand. Web export must produce something suitable for static hosting and platforms such as itch.io.

A successful workflow should resemble:

**Download Sindri -> Create Project -> Make Questionable Game -> Press Play -> Fix Questionable Game -> Export -> Upload -> Inflict Game Upon Internet**

This is the desired technological achievement.

## VIII. Formats must stop moving under people's feet

Alpha does not mean permanent API stability.

It does mean we acknowledge that other humans may now possess Sindri projects.

Serialized formats therefore require versions and migrations where appropriate. Breaking changes require documentation. Project upgrades must be deliberate.

A developer opening a project created three releases ago should encounter either a working project or a clear migration path.

They should not encounter an obscure deserialization error followed by a contemplative evening.

## IX. Documentation must be trustworthy

Documentation is part of the engine.

Before Alpha:

- major systems are documented
- Decay APIs are documented
- editor workflows are documented
- project structure is documented
- export is documented
- examples point toward current practices
- generated documentation is current
- obsolete guidance is removed

A developer should be able to learn Sindri primarily from Sindri.

If documentation and implementation disagree, that is a bug.

## X. CI becomes the gatekeeper

The Alpha commit does not happen because the engine looked healthy that afternoon.

The repository must prove itself.

Tests must pass. Warnings remain denied. Formatting passes. Clippy passes. Generated documentation is current. Repository size rules pass. Native builds pass. WASM builds pass. Browser smoke tests pass. Representative exports launch. Important examples run. The Scale Trial survives. The complete game survives.

No known release-blocking data-loss bug remains.

No ritual sacrifice should be required to make CI green twice consecutively.

## XI. What Alpha does not mean

Sindri Alpha will still have bugs.

Some APIs will change. Some workflows will be awkward. 3D will not possess every feature accumulated by decades-old engines. Rendering, Weave, Decay and the editor will continue evolving. Performance work will continue. Platforms will expand. Features nobody has thought of yet will appear approximately fourteen minutes after release.

Alpha does not mean:

> **Sindri is finished.**

It means:

> **Sindri is ready to be used.**

That is a much more important distinction.

## XII. The Final Trial

Pre-alpha may end only when the following statements are true:

> **An external developer can download Sindri, create a project, build a small complete game primarily through the editor and Decay, debug it, save it, reopen it, export it, and publish it without modifying Sindri itself or understanding its internal Rust architecture.**

> **The Sindri repository is sufficiently coherent, tested and documented that an experienced contributor can enter a major subsystem, understand its responsibilities, modify it confidently and verify the change without requiring knowledge trapped inside its original author's head.**

> **Sindri has demonstrated that its architecture survives projects substantially larger than its development examples.**

When all three statements are boringly, demonstrably true, the gate opens.

## XIII. The last pre-alpha commit

The final pre-alpha build should be tagged and preserved.

Then:

- update the version
- update the README
- remove the pre-alpha warning
- publish release notes
- produce release artifacts
- archive the final qualification results
- publish the first Alpha documentation
- release the editor

And somewhere in that commit should be a very small change.

```text
Status: Alpha
```

Two words.

After physics engines, voxel worlds, rendering systems, Decay, Weave, editors, prefabs, exporters, thousands of tests, probably several horrifying regressions, and an unreasonable quantity of Rust:

two words.

## The pre-alpha exit checklist

Sindri may enter Alpha when:

- [ ] The major 2D engine stack is complete-game viable.
- [ ] A coherent 3D baseline exists.
- [ ] The Physics Patch is complete.
- [ ] Rendering has received its major production-readiness pass.
- [ ] Weave is capable of complete game interfaces.
- [ ] Decay is practical as the normal gameplay-authoring language.
- [ ] Audio supports complete games.
- [ ] Scenes, prefabs and assets survive production-style use.
- [ ] Character movement is production viable.
- [ ] Native and browser runtimes agree where promised.
- [ ] The editor supports the complete create-to-export workflow.
- [ ] Developers do not need Rust or Cargo for ordinary game development.
- [ ] Downloadable editor builds exist.
- [ ] Web export is publication-ready.
- [ ] Serialized formats have appropriate versioning/migration policy.
- [ ] The Scale Trial passes.
- [ ] At least one complete game has been built and exported.
- [ ] External developers have completed the newcomer trial.
- [ ] Their highest-impact findings have been addressed.
- [ ] The repository-wide Quality Patch is complete.
- [ ] Major architectural debt discovered during that audit is resolved or explicitly documented.
- [ ] Documentation describes the engine that actually exists.
- [ ] CI enforces the quality rules that can reasonably be automated.
- [ ] Required CI is green.
- [ ] No known release-blocking data-loss or project-corruption issue remains.
- [ ] Versioned release artifacts can be produced reproducibly.
- [ ] The project can survive development without its creators explaining it.

## And then

For years, game engines begin as experiments.

A renderer draws a triangle.

The triangle acquires input.

Input acquires entities.

Entities acquire scenes.

Scenes acquire scripting.

Scripting acquires physics.

Physics acquires twenty thousand lines in a single pull request because apparently nobody was supervising.

Eventually the collection of experiments becomes something else.

A tool.

A platform.

An engine.

That transition should not happen accidentally.

So Sindri will make it deliberately.

When every gate in this document has been passed, when the complete game ships, when strangers have built things we did not anticipate, when the repository has survived its audit, and when CI finally has nothing interesting left to complain about:

**Pre-alpha is over.**

Strike the warning from the README.

Tag the commit.

Build the binaries.

Open the gates.

# SINDRI ALPHA

## FORGE SOMETHING.
