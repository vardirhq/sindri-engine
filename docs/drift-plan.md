# DRIFT

A plan for **DRIFT**, a first-person survival and exploration game in zero
gravity: a damaged ship in an unknown stretch of space, a safety tether that is
the player's lifeline, and wreckage outside worth the risk of reaching. This
document is the design and the order of work; it moves with the game, and a
stage is checked off here when its acceptance criteria are met and its tests
pass.

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
2. **The tether.** 3D joints in the engine (gap 1), then a tether from an anchor
   to the player: length limit, taut stop, snagging on a pillar, reeling in,
   unclipping and re-clipping. A test drifts out to the tether's end, is stopped,
   and is reeled home around an obstacle.
3. **The airlock.** The *Tern*'s interior with gravity and the grounded walker,
   an airlock that cycles, and the handover to zero-g outside (gap 3). Oxygen,
   suit power and propellant in Decay, with a HUD in Weave. A test walks into the
   airlock, cycles it, floats out, returns and cycles back in.
4. **The wreckage.** A small field of derelict and debris outside, built from
   repeated GLBs with instancing and culling (gap 4). Salvageable objects to
   find, pick up and carry back; convex-hull props that tumble when struck.
   Viewports and visor through the transparent pass (gap 5).
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
