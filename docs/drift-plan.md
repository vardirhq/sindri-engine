# DRIFT

A plan for **DRIFT**, a first-person survival and exploration game in zero
gravity: a damaged ship in an unknown stretch of space, a safety tether that is
the player's lifeline, and wreckage outside worth the risk of reaching. This
document is the design and the order of work; it moves with the game, and a
stage is checked off here when its acceptance criteria are met and its tests
pass.

## New direction: the compartment and the ruins

The player wakes with almost no memory in a small, cell-like compartment. It
floats apart from a much larger ruined structure, connected only by cables;
what happened there and the compartment's original purpose are unknown. The
compartment is the player's first refuge and, through salvage and construction,
becomes their own small ship. The separate flyable shuttle in the current build
is a prototype to retire from the playable scene once its proof tests and
capabilities have a new home.

The sections below describe the current build and earlier stages. They are
historical and implementation evidence where they conflict with this direction,
not a claim that the new opening already exists.

### Intended player journey

1. **Wake and survive.** Learn movement and the airlock in the compartment;
   communicate that air and power are fragile. Its original purpose is unclear.
2. **Reach the ruin.** Cables visibly lead from the refuge to the structure.
   Make a safe, readable route before making cable climbing or power transfer
   promises; these are future mechanics, not current behavior.
3. **Make salvage matter.** Recover a recognizable component and use it for a
   visible first repair at home. Show what was recovered and what it can repair.
4. **Build a ship.** Unlock tools, structural repairs, a seat and thrusters in
   stages; fit them to the starting compartment. Detaching and flying are a
   later milestone, after the first rescue and repair loop is enjoyable.
5. **Uncover the past.** Environmental details and fragments gradually raise
   questions about the structure and the player. No answer is committed yet.

### First implementation slices

- **Block art exploration:** prototype a small family of detailed, half-meter
  modular blocks and inspect them together as walls, corners, and floors before
  building a level. Compare texture-only blocks with actual modeled details.
- **Opening and connection:** establish the compartment as a separate refuge,
  place a navigable ruined structure and visible connecting cables, and verify
  the route with collision, lighting, native/browser captures and a playtest.
  *Started:* the old cargo section now floats five meters from the refuge's
  airlock, with visible service cables crossing the gap. An open, broken
  block-built frame extends the wreckage field. The cables are visual only;
  the existing tether is anchored in the cargo section, not the refuge. The
  route, lighting, performance and first-person readability still need a
  graphical playtest, and the separate shuttle remains a prototype.
- **Useful salvage:** choose and communicate a specific part and repair; test
  pickup, carrying, installation, feedback, and repeated interaction. Until
  then the existing canister counter has no crafting use.
- **Emergency return:** prototype a deliberate oxygen-powered emergency push,
  readable before use and constrained by the remaining oxygen. Verify that
  direction, cost, zero oxygen, and recovery feel fair on desktop and touch.
- **Retire the shuttle:** remove it from the playable scene while keeping the
  general engine features. Move or replace the existing flying/riding tests so
  they continue to prove those capabilities without an unexplained second ship.
- **Home becomes flyable:** implement construction and ship controls only after
  the preceding loop works. A seated view must inherit the ship's rotation;
  disembarking must preserve sensible velocity relative to it and provide a
  way back. Test translating and rotating ships and playtest the camera.

Art target: a worn, readable industrial ruin built from half-meter blocks, with
a broken large-scale silhouette, exposed ribs, open rooms and restrained lights.
Blocks should have convincing depth and engineered detail: recessed panels,
seams, fasteners, vents and occasional pipes or lights. The image reference is
an inspiration for material and depth, not a requirement to put all those
features on every block. Keep ordinary hull blocks relatively quiet and reserve
strong features for accent blocks so repeated walls do not become noisy.
Adjacent blocks must meet cleanly; stripes, trim and edge details should line up
across a row, and corner and edge variants should work as a set. Check repeated
patterns and first-person scale on a small test wall and room.

All eight shuttle voxel block types now use cell-sized GLB models. The
set uses low-profile enameled hull panels, a continuous oxide stripe,
stamped steel deck treads, reinforced framing, ceramic service hatches,
amber hazard bands, recessed diffusers and louvered vents. Colored relief
helps identify each piece at half-meter scale. Normal and roughness maps give
enameled plates, brushed steel, primer, ceramic and light diffusers distinct
surface detail under the same lighting. Ceiling work lights make the deck
and service panels readable from the pilot's seat.
`art/blocks.py` and `art/block_models.py` generate the set and its face art.
There are 14 model batches across two sections, with 50 material draws and
about 99,000 triangles for the shuttle blocks when both sections are visible.

### Modeled voxel blocks: implementation track

Sindri's 3D voxel blocks can use an opaque static GLB confined to one cell.
The block set carries the model reference, and the renderer bakes repeated
blocks into section-local indexed batches by material. They move and scale
with the ship and update when a cell is edited or the model is rebound.
Neighboring cube faces stay visible around recessed geometry. The vent uses
the same full-cell collision as a hull block.

The loading and editing contract is documented in
[Voxel terrain](voxel-terrain.md). Tests cover section boundaries, repeated
models, scaled ownership, removal, model reload and export discovery.
DRIFT tests that every shuttle block takes the model path and stays within
the draw and geometry budgets. The complete block set has been inspected
on the shuttle exterior and at first-person scale inside its cabin.
The explorable ruins are still to come. Connected trim should select
visual variants from the neighborhood while preserving
the placed block's identity. Rotation, transparent materials and modeled
collision are later work.

## What kind of project this is

DRIFT is **a game rather than a showcase**, on the same terms as Lamplighter
(`AGENTS.md`): held to the standard of a game somebody would choose to play,
with gaps it finds closed in the engine generally and never shaped around it.
It is where **first-person 3D, zero-gravity rigid-body movement and 3D
constraints** are proven at the scale of a real game. It belongs to the broad
family of Subnautica (isolation, a safe base, venturing out against a
consumable) without copying it: names, places, creatures, items, art and rules
are DRIFT's own.

It overlaps with the planned third-person `games/explorer` genre showcase
(`docs/3d-update.md`, slice 7) on imported models, model collision, lights and
shadows, but not on terrain, foliage, skeletal animation or an orbit camera,
which DRIFT does not need. Where both need a capability (batching, culling,
transparency, mipmaps), whichever reaches it first builds it, generally, and the
other adopts it.

It would live in `games/drift`.

## The game

The player wakes aboard the *Tern*, a small survey craft adrift and damaged in a
region no chart names. Lights flicker; the oxygen recycler is failing; outside
the viewport is dark, broken by the slow tumble of wreckage catching a distant
star. The ship is the only safe place there is. Everything needed to keep it
alive is out there.

Core vision: *your spacecraft is your sanctuary; your tether is your lifeline;
beyond it lies the unknown.*

### How it plays

The loop is **explore → salvage → craft → upgrade → venture farther**.

- **Zero-g movement.** Full six degrees of freedom with real inertia: what moves
  keeps moving. Suit thrusters translate and turn the player, but burn
  propellant. Grabbing a surface stops the player relative to it; pushing off
  launches them along their look. Handholds on hulls make routes. Good movement
  is grabbing and pushing; thrusters are for correction and for when things go
  wrong.
- **The tether.** A physical line from the player to the ship or to an anchor.
  It has a length, it goes taut, it snags on geometry and drags along hulls, and
  it can reel the player home. Portable anchors extend range by re-rooting it;
  upgrades lengthen it. Unclipping is allowed and is the biggest risk in the
  game: untethered, a missed push-off with dry thrusters is a drift into the
  dark.
- **Survival.** Oxygen during EVA, suit power, thruster propellant. Hazards:
  radiation near certain wrecks, heat and cold in and out of a star's light,
  tumbling debris. Ship systems (recycler, power, lights, airlock) break and
  are repaired.
- **Exploration and progression.** Derelicts, stations, asteroid clusters and
  structures that are not anyone's. Salvage yields components and recovered
  technology; crafting makes tools, suit upgrades and ship equipment; later,
  pressurized modules are bolted on to grow the ship into a station. A signal,
  and what is found following it, tells the story without cutscenes.

### Open design choices

- Whether the ship interior has gravity (spin, plating) or is zero-g throughout.
  The plan assumes **gravity inside, none outside**, which makes the airlock a
  real threshold; the engine supports either.
- Whether death is a reload of the last save or a cost carried forward.
- How much of the tether is simulated rope versus a constraint with a drawn
  line: a fully colliding rope is the goal, but feel decides how far it goes.
- Whether the signal source is ever reached in the first release.

## How it looks

Stylized realism: hard, readable silhouettes, worn mechanical detail, and
strong contrast between the warm, lit interior of the ship and the vast dark
outside. Light is placed, not ambient: a far star as the directional light,
work lamps and the suit's helmet light as spot lights, emissive panels and
status lights inside. Bloom and exposure carry the glare of the star; fog is
used inside smoke-filled or depressurized spaces, not in vacuum.

The opening now has a visible emissive star aligned with the directional
starlight and a low ambient fill to keep unlit silhouettes from disappearing
completely. The cabin, airlock and helmet retain their local lights. This is an
initial lighting pass: local point and spot lights do not yet cast shadows or
respect occluders, so a lamp can illuminate through walls. The apparent star
is a scene object at a fixed distance, not an astronomical sky; its visual
alignment and visibility, dark routes and cabin readability still need an
in-game capture and playtest.

Models are made in Blender and exported as GLB, with AI-assisted modelling
where it helps; following Causeway's lead, generation scripts and sources live
in tree (`games/drift/art/`) so assets are reproducible. Asset complexity is set
by the target of running well on modest hardware: modest triangle counts,
shared material profiles, repeated debris pieces rather than unique ones.

## What the engine has, and what to verify

Present and proven elsewhere, and used directly:

- **Imported GLB models** rendering in native, browser and editor, with shared
  material factors and profiles, emission and sidedness (Day/Night Lab,
  Model Collision Lab).
- **Collision from models:** static triangle colliders for ship interiors and
  hulls, convex hulls for dynamic salvage (Model Collision Lab).
- **3D physics world and bodies:** `sindri.physics3d.world` gravity and per-body
  `gravity_scale`; Decay `velocity`, `set_velocity`, `angular_velocity`,
  `set_angular_velocity`, `apply_impulse`; contact and sensor events; raycasts,
  overlaps and sphere/box/capsule casts.
- **3D transforms in Decay:** quaternion `rotation`, `forward`/`right`/`up`,
  `look_at`, `rotate_around`; perspective FOV.
- **Mouse look:** `Pointer.delta` and pointer lock, native and browser.
- **Lights:** directional, point and spot lights that ride moving parents (a
  helmet lamp), model shadows, the post stack (exposure, tone mapping, bloom,
  vignette).
- **Screen UI** in Weave, **saves**, **audio** and the Sound Lab.

To verify before relying on it:

- **Impulse-driven thrust feels continuous.** Thrusters as a small impulse every
  fixed step, plus angular impulse or velocity for turning; check for jitter at
  the fixed step and at frame rates that do not divide it.
- **Changing gravity at the airlock** (by world gravity or per-body
  `gravity_scale`) is immediate, wakes the player and loose props, and replays
  deterministically.
- **First-person camera on a rotating body:** the camera follows the body's full
  orientation without lag, and look is applied relative to the body in zero-g
  and relative to the floor in gravity.
- **Raycast filters** cleanly exclude the player's own body and tether for
  interaction and grab probes.

## The gaps

None of these is large, and each is general engine work that other 3D projects
need too. Each is closed in the stage that first needs it, with its parity row,
capability evidence and tests.

1. **3D joints.** Sindri has 2D distance, rope, revolute and prismatic joints;
   the 3D world has none. DRIFT needs at least scene-authored and Decay-owned
   **3D distance/rope and spherical joints** on the Rapier 3D world, mirroring
   the 2D contract (`docs/physics.md`): owner entities, enable/suspend, tuning,
   editor gizmos. The tether is then a chain of small bodies on rope joints,
   with the player's end a joint to the player body, so tension, snagging and
   collision come from the solver. Reeling in is shortening the rest lengths.
2. **Continuous force and torque in 3D.** 2D has additive force and torque that
   last one fixed step; 3D exposes impulses only. If per-step impulses prove
   adequate (see above) this stays optional; if not, add `apply_force` and
   `apply_torque` to 3D on the 2D contract.
3. **A zero-g controller, as a pattern, not a component.** The existing
   `sindri.physics3d.character` is a grounded kinematic walker and is the right
   tool inside the ship under gravity. Outside, the player is a dynamic capsule
   body driven by thrust, grab and push-off from Decay. The handover between the
   two at the airlock (kinematic walker ↔ dynamic body, keeping velocity) is the
   part to prove; if it needs engine support, it is added generally.
4. **Rendering many repeated props.** Every 3D draw is its own pass, nothing is
   instanced and meshes/models are not frustum-culled (`docs/3d-update.md`,
   slices 2–3). A wreckage field of repeated debris needs **instancing and
   culling** before it is large.
5. **A transparent 3D pass** for viewports, the helmet visor and glass, and
   **mipmaps** so distant hull textures do not shimmer (slice 3).
6. **Authored fog and post controls from Decay** for smoke, depressurization and
   damage effects (slice 4b).
7. **CCD in 3D** if fast debris or a fast player tunnels through thin hulls.

Not needed for the first playable: terrain, foliage, skeletal animation, an orbit
camera, editor drag-in of models (useful, not blocking).

## Block-built ships

DRIFT's ships are built of blocks: half-metre voxel blocks for hull, deck,
frame and bulkhead, which the player cuts apart as salvage and places to build,
with detailed models for doors, windows, consoles, lights and thrusters
snapped to the same grid. A ship is drivable: one rigid body that flies, turns
and carries the people walking inside it, and a wreck is the same thing
adrift. It is the genre of block-built spacecraft, made as DRIFT's own: its
look aims at worn, engineered metal rather than toy cubes.

Engine work, each step general, in order:

1. **Built voxel worlds.** *Done.* A voxel world that is only its placed
   blocks, scaled by its entity (`docs/voxel-terrain.md`).
2. **Voxel bodies.** *Done.* `sindri.physics3d.voxel_body`: a built world's
   blocks merged into exact boxes as one rigid body, rebuilt on edits with its
   motion kept (`docs/physics.md`).
3. **Riding a moving ship.** *Started.* A hull is a tagged entity whose own
   coordinates define its interior and gravity. The diver's capsule aligns
   to the hull and uses a bounded, collision-resolved velocity correction to
   hold its local deck position when grounded and idle. Moving or jumping
   releases that anchor; exiting keeps the body's world velocity. Tests cover
   a full roll, pitch, walking, jumping, wall collision, and exit velocity.
   This is a gameplay-level correction, not a general moving reference-frame
   constraint; visual playtesting, moving ship interactions, and collision
   behavior under stronger acceleration still need review.

4. **Flying it.** *Started.* Sindri's 3D physics gains `apply_impulse_at`,
   an impulse through a point. The shuttle's functional parts are models on its
   grid (`art/parts.py`): twelve thrusters (mains, retros and pairs on its
   flanks, keel and roof) with flames that show while they fire, and a pilot's
   seat with a console. F at the seat sits at the controls (`scripts/pilot.decay`):
   W/S, A/D and Space/C fire every thruster whose push points the way asked,
   each through where it is mounted, X brakes against the drift with them, and
   a gyroscope turns the ship to the mouse and Q/E and otherwise holds it
   steady. A ship that loses a main engine turns when it burns. The pilot rides
   in the seat, and standing up in flight keeps the ship's speed
   (`tests/flying.rs`). Still to come: a cockpit with glass, thrusters as parts
   a player places, and flying the *Tern* itself.
5. **Realism.** *Started.* Imported models sample embedded normal and packed
   roughness/metalness maps. All eight shuttle blocks use them, with distinct
   finishes generated by `art/blocks.py`. Ambient occlusion maps, material maps
   for face-textured cubes and shadows from the helmet lamp are still open.
6. **The *Tern* rebuilt in blocks**, wrecks as floating voxel bodies, salvage as
   cutting blocks out and repair as placing them.

The block set is `art/blocks.py` (worn painted hull plate, hull with stripe,
tread deck, vented bulkhead, frame, hazard, light panel); ships are written by
`art/ship.py`. The first is a 12 m shuttle floating off the wreckage.

## Playing on a phone

DRIFT plays on a phone held sideways, with its controls always on screen
(`art/touch.py`, `scripts/touch.decay`): a move stick on the left half, a view
stick on the right half (a pad whose thumb `Ui.drag` reads, turning at a rate
by how far it is pulled), roll buttons, and Action, Jump and Crouch. Action is
context: tapped it works the airlock panel, takes salvage or sits; held, it
holds on. Jump is up in space, Crouch down. The touch script reads them into a
`Controls` state each script merges with its keys. Reel, clip and brake are
keys only for now (`tests/touch.rs`).

## Stages

Each stage is one or more pull requests that leave `main` working: the project
opens, its scripts compile, its scripted run reaches its goal, and this document
says what is now true.

0. **Direction.** This plan and the `AGENTS.md` description. *Done.*
1. **Floating.** *Started.* `games/drift` opens in the *Tern*'s cargo hold
   (`art/hold.py`): 14 m of plated room with no gravity, work lamps and a red
   emergency light, yellow handholds down the bulkheads, walls and overhead, and
   three crates drifting and turning in it. The diver (`scripts/diver.decay`) is
   a dynamic sphere that physics never turns; the view is the Eye under it, kept
   as a quaternion and turned about its own axes by the mouse and Q/E, so there
   is no up and no flip past vertical. W/S, A/D, Space/C thrust along the view
   on a propellant tank, X brakes against the drift. F or the right mouse holds
   on to a handhold in reach (held, it catches the next one); Space or the left
   mouse pushes off along the view. A HUD shows propellant, speed and what a
   hand can do. `tests/floating.rs` pushes off the aft handhold, drifts the
   length of the hold and catches the forward one, and checks momentum kept and
   braked away; the editor plays it the same as the build. Per-step velocity
   changes made thrust smooth enough that continuous 3D force (gap 2) is not
   needed yet. Still to come: grabbing any surface, not only handholds; a
   collision jolt and sound; and a person playing it to judge whether floating
   is fun. *This stage decides whether the game is worth making: if floating is
   not fun, nothing later saves it.*
2. **The tether.** *Started.* Sindri gains 3D distance joints (gap 1, the
   rope kind): `sindri.physics3d.distance_joint` between body-local anchors,
   owned by its own entity like the 2D one, with `Physics3d.set_distance`,
   `set_joint_enabled` and `joint_enabled` (`docs/physics.md`). The tether
   (`scripts/tether.decay`) is sixteen link bodies on seventeen of them, from
   the aft handhold to the diver: the links touch the hold and what is in it,
   never each other or the diver, so the line slackens, goes taut at 10 m and
   catches on what it is drawn across. A lanyard absorber in Decay takes out
   motion along a taut line, so coming up short is a tug rather than a
   rebound. R reels in (the line shortens, drawing the diver home, and pays out
   again when let go); U unclips, and clips back on only within reach of the
   loose end. Thin strands drawn between the links show the line, meeting the
   diver at the waist. A stanchion part way down the hold gives it something
   to catch on. `tests/tether.rs` drifts out to the line's end, is stopped
   without a rebound and reeled home; passes the stanchion, crosses behind it
   and is drawn round it when reeled; and unclips, clips back on at the loose
   end, and drifts free past the line's length. Two things it taught: a link
   scaled to size its mesh is scaled in mass too (now in the physics
   contract), and a chain of light links needs an absorber to feel like webbing
   rather than elastic. Still to come: portable anchors that re-root the line,
   longer tethers as upgrades, a tug sound and jolt, and a person flying it.
3. **The airlock.** *Started.* The *Tern* now has a cabin and an airlock
   chamber aft of the hold (`art/hold.py`), and the hold is the outside: vacuum
   and no gravity. The diver wakes standing by the bunk. Aboard, the ship's
   gravity holds: the diver walks (W/S, A/D, Space to jump) and the view keeps
   the deck below, righting itself over a moment on coming in from outside.
   Outside, the view turns about its own axes as before. The airlock
   (`scripts/airlock.decay`) has two sliding pressure doors and is only ever
   open to one side: E at any of its three panels closes the open door, pumps
   the chamber down or up over three seconds and opens the other, with its lamp
   green, amber or red for the pressure. The suit carries oxygen and power that
   drain in vacuum and refill in the ship's air, with propellant; out of air,
   the diver blacks out and wakes in the cabin. The HUD shows all three and the
   airlock's cycle. The tether now starts unclipped at the aft handhold, just
   outside the outer door. `tests/airlock.rs` walks from the cabin into the
   chamber, cycles out, steps out weightless and drifts on at walking speed,
   thrusts back in under gravity and cycles in; checks a cycle waits for each
   door and the pressure; and runs out of air outside and wakes aboard. Gap 3
   is answered without engine work: rather than handing over between the
   kinematic character controller and a dynamic body, the diver is one dynamic
   body throughout, and the ship's gravity, footing and walking are the diver
   script's inside the hull. Stepping through the outer door is then the whole
   change, momentum and all. Still to come: a door that will not close on the
   diver, sound (the hiss of the pump, footsteps, breath), and a person
   walking it.
4. **The wreckage.** *Started.* The forward bulkhead is now an open cargo
   hatch with a safety bar across it, and beyond it is the wreckage
   (`art/wreck.py`, models made by the repository's GLB writer): a derelict
   survey module torn open at its far end, solid to fly into, with a red lamp
   still burning inside; twenty-two bent plates and truss girders drifting and
   turning, dynamic bodies on convex colliders that the diver knocks away; three
   salvage canisters; and a starfield, one model and so one draw. G takes a
   canister in reach into the pack; what the diver carries is stowed once they
   are back in the ship's air (`scripts/salvage.decay`). Sindri gains view
   culling (gap 4, the culling half): cubes and models wholly out of the
   camera's view, and out of the light's shadow view while shadows are on, are
   not drawn. Looking out of the hatch draws 32 of the scene's 165 pieces.
   `tests/wreckage.rs` takes salvage outside and stows it aboard, knocks a plate
   away and flies into the derelict; `tests/culling.rs` measures the draws.
   Still to come: batching and instancing of repeated geometry (the other half
   of gap 4: every girder is still its own draw), occlusion culling, the
   transparent pass and mipmaps (gap 5), a portable anchor so the wreck can be
   reached on a line, and carrying salvage in hand rather than in a count.
5. **First playable.** The milestone, end to end: wake aboard the *Tern*;
   discover the failing recycler; suit up and clip on; go out; recover the
   replacement component from the wreck; return; repair the recycler; detect
   the signal. A test plays it from waking to the signal.
6. **Atmosphere.** Lighting pass, emissive panels, the helmet lamp, bloom and
   exposure on the star, smoke and fog inside (gap 6), sound and score from the
   Sound Lab, saves. Screenshots and site entry; `docs/parity.md` and the
   capability pages brought level with what is played.

After the first playable, and only once it proves enjoyable: crafting and suit
upgrades, portable anchors and longer tethers, hazards (radiation, heat, cold,
debris), more derelicts, pressurized modules and the station, and the story the
signal leads to.

## Checks that hold throughout

- Every stage has a scripted run that plays it; one test plays the first
  playable from waking to the signal. A goal that cannot be reached is a failing
  test.
- `scripts/preflight.py` and the typed Decay preflight on every changed script;
  no game rules in Rust; the project has no `src` beyond its test harness.
- Physics is deterministic: the same inputs replay to the same tether and the
  same drift, and a test says so.
- Art is reproducible from the sources and scripts in tree.
- It runs well on modest hardware: the 3D benchmark counters are recorded for
  the wreckage field, and a regression in draws or passes is a failing check.
- A capability gap is closed in the engine, generally, in the stage that finds
  it, with its parity row and tests.
- Nothing is copied from another game: names, places, creatures, items, art and
  rules are DRIFT's own.
