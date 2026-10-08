# Editor direction

What the Sindri editor is for, what it should become, and the order the work
goes in. This is the document `docs/editor-architecture.md` sits under: that one
says how the editor is built, this one says what it is trying to be.

## The problem this starts from

The editor is complete enough to be judged and is not yet one anybody reaches
for. Its parts are individually right — a hierarchy, a generic inspector driven
by the schema registry, a project browser, gizmos, play/pause/step with snapshot
restore — and using it is still work rather than pleasure. That is not a
shortfall of features. It is what the editor is *shaped to do*.

It is shaped as a data-entry tool. The primary surface is a column of fields,
and the primary gesture is typing a number into one of them. That was the right
shape for an editor in 2012, when the alternative was editing the file by hand.
It is the wrong shape now, and the reason is not fashion: entering structured
values is the one job at which a person is slowest and least interested, and a
tool whose main offer is doing that job is a tool people route around.

What a person is uniquely good at — and what no amount of generated content
replaces — is **feeling** whether a game is right, and **judging** what to change
next. The editor should be built around those two, and should treat data entry
as the fallback it is.

## What the editor is for

1. **Closing the loop between a change and its consequence.** Play, feel that
   something is wrong, change one thing, feel it again. Every part of the editor
   should be measured by what it does to the length of that loop. This is the
   first principle and the others are downstream of it.
2. **Making the scene legible.** What is in this world, what is it doing, and
   why did it just do that. A hierarchy of five hundred rows is a file listing,
   not an answer.
3. **Manipulating the thing rather than its numbers.** A radius is dragged in
   the viewport. A curve is scrubbed. A speed is changed while the game runs and
   the change is visible immediately.
4. **Saying plainly when something is wrong**, once, with the thing it is about.

## What follows from that

### Live editing is the centre, not a mode

Stop restores the world as it was when Play was pressed, so tuning used to mean
play, stop, guess, play again — and the feel you were tuning against was gone by
the time you were editing. That was the single largest cost in the loop above.

The capability is: **edit while playing, and decide what to keep when the run
ends**, and it is built. A change made during a run is a command like any other,
applied to the running world and recorded against the run (`run_edits.rs`); Stop
lists them and applies the kept ones to the restored scene as ordinary history
entries. What comes back is what the person changed — the component fields and
transform parts an edit wrote — not the values the game wrote beside them, and
an edit to something the run spawned is explained rather than dropped.

### Direct manipulation before fields

The inspector should be where a value is read and corrected, not where it is
found. Anything with a shape belongs in the viewport: collider bounds, camera
frustums, spawn radii, the reach of an effect. Sindri has transform gizmos and
nothing else, and each missing one sends an author back to typing numbers and
pressing Play to see whether they were right.

### Time is a dimension the editor can move in

The engine steps at a fixed rate and the simulation is deterministic. That makes
recording a run and scrubbing it a genuinely available capability rather than an
ambition, and it answers the question authored fields cannot: *why did that
feel unfair?* The History panel is command history; this is world history, and
it is where the editor has the most room to be better than its baselines rather
than level with them.

### Errors are the editor's failure, not the project's

Opening Orbital Last Stand reported dozens of errors, most of them the same
error recorded again every frame. Counting repeats fixed the volume; what is
left is the harder half. Forty entities failing because one script did not load
is one problem, and the console should say so — group by cause, name the
entities under it, and where the editor knows the fix, offer it.

### The arrangement belongs to the person using it

Implemented. See below; it is the first slice, because an editor whose panels
cannot be moved is one that has decided what you are working on.

## The canvas-first question

A mockup put the proposition plainly: one full-window scene view, with every
panel floating over it as an instrument. It is worth writing down what that
gets right, what it gets wrong, and which of the two this repository is acting
on, because the next person to see a picture like it will ask again.

### What it assumes

Canvas-first is the document-editor model — Figma, Illustrator: one canvas,
tools arranged around it. That model works for Figma because of a property
games do not share. **In Figma, what you see is the entirety of what you are
making.** Nothing in the document is invisible.

A game scene is not like that. Much of what is authored has no appearance at
all: script bindings, collision layers, spawn tables, tags, audio triggers,
prefab inheritance, the director that decides when the enemies come. The scene
view shows the set dressing and hides the machine. So a canvas-first editor
optimises hard for the part of the work that was never the bottleneck — placing
visible things — and puts the genuinely difficult part, understanding the
invisible machine, behind a summon.

That is the reservation. It is not a refusal, because the editor as it stood
gave the scene no primacy at all: the viewport was the rectangle left over when
five panels had taken theirs.

### What this repository does about it

**Canvas-first as a resting posture, not as an architecture.** The editor rests
looking like that picture and becomes an IDE when the work is IDE work. Which
is a question about default state and about how a hidden surface is summoned,
not about whether panels dock.

Concretely, and in the order the items are worth doing:

1. A **placement** beside `docked` and `closed`: an overlay anchored to a corner
   of the scene view, which draws over the world rather than taking room from
   it. `Canvas` is then a preset rather than a second editor.
2. ~~**A command palette.**~~ Done: `editor/src/palette/`. Ctrl+K, and one
   field that finds panels, entities, project files, scenes, arrangements and
   verbs. It is the safety net that makes hiding a panel something other than
   losing it, which is why it came before anything else here.
3. **The Game view demoted to an anchored camera thumbnail**, promotable to
   full. Most of the time the question is *is the player in frame*, which a
   thumbnail answers and half a window does not.
4. **Contextual verbs at the selection**, and the properties they belong to
   staying at an edge. Verbs travel well; thirty schema-driven fields do not.
5. **The Add palette populated from the project's own prefabs**, not from a
   fixed list. Orbital gets Drifter and Charger in it without the engine having
   heard of either.

### What it gets wrong, and is not being copied

- **A floating inspector.** It moves on every selection so no muscle memory
  forms, it covers the neighbours a value is being judged against, and Sindri's
  inspector is schema-driven — a real entity is thirty fields, not five.
  Properties stay at an edge.
- **Everything floating at once.** Six translucent surfaces do not overlap in a
  drawing because someone placed them; they overlap constantly in use. Overlays
  here are anchored to corners and stack, so they cannot be piled on each other
  by accident.
- **Top-level modes for editors that do not exist.** A `Script` or `Audio` tab
  teaches someone the tool is bigger than it is and disappoints them on the
  second click. An absent mode is better than a hollow one.
- **A design tuned for an empty scene.** The mockup's status bar says three
  thousand entities and its hierarchy shows six. What has to survive is a
  hierarchy of three thousand rows, an inspector on a forty-field body, and a
  tileset dense enough that a translucent panel over it is unreadable. Design
  for hour three, not minute one.

### The two things a mockup cannot show

**Liveness.** What makes an editor feel like this decade is not floating panels;
it is that nothing stalls, everything is live, and the distance between a change
and its consequence is zero. A canvas-first editor that hitches on selection
feels worse than a plain docked one that never does. This cannot be drawn, which
is exactly why it gets under-invested, and why editors designed from mockups
tend to look current and feel ten years old.

**The loop.** A picture shows an editor at rest. The reason this one is not
reached for is not that its panels have borders — it is that tuning a value
means stop, edit, play, and try to remember what it felt like ninety seconds
ago. Canvas-first is a good surface answer to a structural problem. It does not
replace items 4 and 5 of the order of work below, and it must not be allowed to
reorder them.

### Getting the docked editor back

Two ways, and neither is a folder of dead code kept beside the live one. A
second copy of an editor is a second editor to compile, test, and keep honest,
and it rots from the first week.

**As a menu click.** The docked arrangement survives as the `Docked` preset in
**View -> Arrangement**. It is the same panels in the same places; nothing about
it was deleted. Changing your mind about the default costs one click, not a
revert.

**As a commit.** The editor exactly as it stood before this direction was
adopted is commit `8880d96` on `claude/bold-goodall-s1xveg`, tagged
`editor-docked-v0` locally. (The tag is not on the remote: this environment's
GitHub token pushes branch refs only, and refused the tag with a 403. The commit
is pushed, so the archive is real either way -- the tag is just a friendlier
name for it.)

```bash
git show 8880d96:editor/src/dock/mod.rs     # read one file
git diff 8880d96 -- editor/                 # what the canvas work changed
git checkout 8880d96                        # run it whole
```

### Warmth is a requirement

The one thing in the mockup to take without qualification. It is warm, it has
texture, it does not look like a dashboard, and the editor it was drawn against
is competent and cold. "I should want to use the editor" is a specification, not
a mood — and an editor nobody opens has failed however correct its panels are.

## Order of work

Dependency order, not priority order — each item is cheaper once the ones above
it are done.

1. ~~**A workspace the user arranges.**~~ Done: `editor/src/dock/`. Every panel
   is a tab, every tab is draggable into any slot, and slots resize freely.
   Extended since with overlay placement and the `Canvas` preset; see the
   canvas-first section above.
2. ~~**Error grouping in the console.**~~ Done: a repeat is counted against any
   matching entry rather than only the one before it. What is left is grouping
   by *cause* — forty entities failing for one missing script are still forty
   lines, each counted correctly.
3. **Gizmos for the shapes that have none** — colliders, cameras, effect radii.
   Each one removes a round trip through Play.
4. **Edit while playing, with a decision at Stop.** Built in the editor
   update: the largest single improvement to the loop.
5. **Record and scrub a run.** Depends on 4 having settled what a run's
   authority over the world is.

Items 4 and 5 are scheduled in the editor update, `docs/editor-update.md`,
on top of one runtime session shared with the shipped host.

## Modes

A mode is a **different document and a different canvas**, not a different set
of panels. That is the whole test, and it is what separates a mode worth having
from an arrangement someone could already have dragged into shape. Scene is the
world. UI is the screen. Audio is the mix. Script is the graph of what runs.

The dock model already carries most of this: a `Workspace` is data, persisted,
holding places and panels, so a mode is a *named* workspace plus the panels that
mode offers. Switching modes swaps the workspace. The expensive part of modes is
therefore not the modes — it is the editors inside them, which is where the
effort belongs anyway.

Ranked by what already exists to build on:

1. **UI.** The strongest case by a distance, because Weave is already here: the
   styles, the layout engine, hot reload, the screen-size chooser. A UI mode is
   a screen-shaped canvas instead of a world-shaped one, a widget tree instead
   of an entity hierarchy, and a styles inspector. Most of the runtime exists;
   what is missing is the authoring surface. It is also where Sindri could be
   plainly better than its baselines, because UI authoring is the weakest part
   of every engine it would be compared against.
2. **Audio, as an overview and a mixer.** Every `AudioSource` in the scene, the
   clip each names, its volume and falloff, and what triggers it. This is the
   part **only Sindri can do**, because only Sindri knows the scene — the audio
   equivalent of the hierarchy.
3. **Script, as the thing an external editor cannot be.** Not a code editor; it
   cannot beat VS Code at editing text and should not try, and `tools/decay-lsp`
   already makes the external editor good. What it can be is which scripts are
   attached to what, what each script's exports need authored, live values while
   the scene plays, and compile failures mapped to the entity that has the
   problem.

Two things deliberately excluded, and the reasoning is the same for both: a MIDI
editor implies a synthesis and sequencing runtime the engine does not have and
competes with tools that have had decades of work, and recording competes with
Audacity for a job done once per asset. A bad version of either inside a game
engine costs the credibility of the mode that holds it. If something specific
demands them later, that demand will be better information than the ambition is.

The discipline that makes modes safe: **ship one only when it is better than the
alternative for at least one real task.** An empty mode teaches someone the tool
is bigger than it is and disappoints them on the second click, which is the
objection to the mockup's `Script` and `Audio` tabs and remains the objection to
shipping a mode before it does something.

## What is deliberately not here

**AI in the editor is limited to what Sindri can check.** The decision this
section used to record was that the editor integrates no AI at all. It changed
once a local-model setup panel had been built and did nothing, which is the
placeholder this section warned against. The way out is one feature whose
answers are graded by the engine rather than trusted: **Decay repair**.

- A previewed `.decay` script that does not compile shows its errors, from the
  same check Play compiles with, whether or not any assistant is set up.
- Once a local model has passed Sindri's own repair cases on this machine, the
  preview offers **Propose a fix**. The model sees the file, its errors and the
  generated host API; its answer is compiled, and a failing answer is sent back
  with its own errors at most twice more. Only a candidate that compiles and
  keeps every script and component the file declared is shown, as a diff.
- Nothing is written until the person accepts it, the write refuses a file
  that changed in the meantime, and the previous text can be put back.

That keeps the reasons for the original decision. The model runs locally, so
there are no API keys, no per-use cost, and no dependency on a service being up.
The editor talks to it over loopback, with no HTTP stack or TLS client. Nothing
depends on it: with no model set up, the editor still shows every error.
`docs/local-ai-architecture.md` is the wider design this is the first slice of,
and it is still a proposal.

What the architecture already gives, at no cost and for its own reasons:

- **Every mutation is a checked, named, reversible command** rather than a world
  write. Anything proposing a change — a person, a script, a tool, a model —
  goes through the same seam, and anything that comes through it can be
  reviewed and undone.
- **Panels are data.** A new panel is one `dock::Panel` variant and one arm of
  the match in `native/workspace.rs`. Adding a surface costs no rearrangement of
  the editor around it.
- **The component schema registry describes components in machine-readable form**
  and `docs/generated/` is regenerated from it, so what the editor knows about a
  project is already written down rather than implicit in UI code. The repair
  prompt carries `docs/generated/decay-api.md` for exactly that reason.

What is **not** being built, and should not be added speculatively: cloud
providers, credential storage, a chat surface, or an AI feature whose output
Sindri cannot check before a person sees it. Writing a new script from a
description is the natural next feature, because it runs through the same
compile-and-repair loop; scene edits wait for the proposal protocol in
`docs/ai-authoring-protocol.md`, because nothing yet checks a proposed world
change the way the compiler checks a script.
