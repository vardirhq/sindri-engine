<!-- Generated from docs/capabilities/runtime.toml by `cargo run -p sindri-capabilities -- --write`. Edit the source, not this file. -->

# Effects, saves and randomness

Short-lived visual effects kept out of the entity world, persistent game saves,
and the seeded random stream every script shares.

Each entry links its tests and the projects that exercise it. Surface marks match the row in [parity](../parity.md) where one is named: ✅ works and is exercised · 🟡 a useful slice, with the gap named · ❌ absent · — not applicable.

| Capability | Engine | Editor | Decay | Proof |
| --- | :-: | :-: | :-: | :-: |
| **Effects** | | | | |
| [Pooled fleck bursts, batched with sprites](#fleck-bursts) | ✅ | 🟡 | ✅ | ✅ |
| [Particle system (emitters, curves, shapes)](#particle-emitters) | ✅ | 🟡 | ✅ | ✅ |
| **Game saves** | | | | |
| [Game saves](#game-saves) | ✅ | ✅ | ✅ | ✅ |
| **Randomness** | | | | |
| [Seeded, portable randomness](#seeded-randomness) | ✅ | ✅ | ✅ | ✅ |
| **Not yet** | | | | |
| [Authored parallax](#parallax) | ❌ | ❌ | ❌ | — |

## Effects

<a id="fleck-bursts"></a>

### Pooled fleck bursts, batched with sprites

Short-lived flecks live as plain values in one bounded pool, drawn in the same batches as sprites.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Pooled fleck bursts, batched with sprites*

**Tested by:** [`crates/sindri-scene/src/effects/pool.rs`](../../crates/sindri-scene/src/effects/pool.rs), [`crates/sindri-decay/tests/a_script_throws_flecks.rs`](../../crates/sindri-decay/tests/a_script_throws_flecks.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

**See:** [`docs/effect-scaling.md`](../../docs/effect-scaling.md)

<details markdown="1">
<summary>Evidence</summary>

`Effects2d` holds short-lived visual flecks as plain values in one array. A
fleck has no identity a script can hold, no components, no place in the
hierarchy, and nothing can collide with it — everything an entity is *for* is
given up, and `docs/effect-scaling.md` is why: 8,000 flecks-as-entities cost
5.25 ms a frame, a third of a 60 Hz budget, against 0.018 ms pooled. Over half
the entity cost was `serde_json` re-reading each one's payload, every entity,
every frame.

What a burst looks like is authored on the entity as `sindri.effect.burst` —
count, speed, spread, lifetime, size, tint, fade, drag — because those are a
designer's numbers and a call naming all of them would be unreadable. Flecks
draw their directions from a stream of their own, never the run's, so turning
an explosion up cannot change which enemies spawn. The pool is bounded, the
oldest fleck makes way for the newest, and overflow is counted rather than
hidden.

They batch with ordinary sprites by layer and texture, so a burst is one draw
call rather than a second rendering path.

</details>

<a id="particle-emitters"></a>

### Particle system (emitters, curves, shapes)

`sindri.particles` emitters throw into the fleck pool at an exact rate, with shapes, ramps and world or local simulation.

Engine ✅ · Editor 🟡 · Decay ✅ · Proof ✅ — parity row *Particle system (emitters, curves, shapes)*

**Tested by:** [`crates/sindri-scene/src/effects/emitter_tests.rs`](../../crates/sindri-scene/src/effects/emitter_tests.rs), [`crates/sindri-scene/src/effects/particles.rs`](../../crates/sindri-scene/src/effects/particles.rs), [`crates/sindri-decay/tests/a_script_drives_particles.rs`](../../crates/sindri-decay/tests/a_script_drives_particles.rs), [`games/platformer/tests/sparkle_and_dust.rs`](../../games/platformer/tests/sparkle_and_dust.rs)

**Exercised in:** [`games/platformer`](../../games/platformer), [`games/low-tide`](../../games/low-tide)

**See:** [`docs/particles.md`](../../docs/particles.md)

<details markdown="1">
<summary>Evidence</summary>

`sindri.particles` is an emitter that keeps throwing into the same pool while
it plays: a rate and a cap, lifetime, speed, direction and spread with
variation, a point, circle, box or ring shape at an offset, gravity, drag, size
over life, a color ramp, rotation and spin, and world or local simulation. It
owes its rate times each step and carries the remainder, so a fractional rate
is exact at any frame rate, and it draws from the pool's stream. Decay plays,
stops and emits a handful at once (`Effects.play`, `Effects.stop`,
`Effects.emit`); the editor runs the selected emitter in the Scene view while
nothing plays. The platformer's coins sparkle and its hero trails dust; Low
Tide's treads kick up dust and its engine smokes while it drives.

</details>

## Game saves

<a id="game-saves"></a>

### Game saves

A flat, versioned key/value save written atomically to disk or `localStorage` by the host, never by a script.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Game progress saves (numbers, flags and text)*

**Tested by:** [`crates/sindri-core/src/save.rs`](../../crates/sindri-core/src/save.rs), [`crates/sindri-platform/src/saves/file.rs`](../../crates/sindri-platform/src/saves/file.rs), [`crates/sindri-platform/src/saves/mod.rs`](../../crates/sindri-platform/src/saves/mod.rs), [`crates/sindri-decay/tests/a_script_remembers.rs`](../../crates/sindri-decay/tests/a_script_remembers.rs), [`games/orbital-baked/tests/the_mixer.rs`](../../games/orbital-baked/tests/the_mixer.rs), [`games/orbital-baked/tests/the_complete_game.rs`](../../games/orbital-baked/tests/the_complete_game.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

**Limits:**

- Decay's store holds numbers, truths and text; it has no world snapshot operation.

<details markdown="1">
<summary>Evidence</summary>

`sindri_core::SaveStore` is a flat, versioned key/value document. Flat because
Decay holds numbers, truths and text and nothing else: a structure a script
could not build is a structure nothing could write, and the file stays
something a person can read and repair. Ordered keys, so the same saved state
is the same bytes and a save can be diffed.

Three absences are distinguished, because they call for different things: a
first run starts cheerfully, a **damaged** save is worth telling someone about
before their progress is written over, and one written by a **newer** build is
reported without being read — a reader that guessed at a format it does not
know would corrupt it the moment it wrote back.

`sindri_platform::SaveBackend` is where the bytes go. `MemorySaves` is the
default so a headless run and a test have somewhere to write without choosing a
path; `DamagedSaves` exists so the unreadable path can be proven without
corrupting a real file; `FileSaves` writes beside the target and renames over
it, because a save half written is a save destroyed at the exact moment
someone's machine lost power mid-run; `BrowserSaves` uses `localStorage`,
chosen over every larger browser store because the alternatives are
asynchronous and a game should not have to ask whether its progress has landed
yet.

Decay reaches Unicode text through typed `Save.text` and `Save.set_text`, with
explicit wrong-type/missing-key fallbacks. Orbital uses those calls for pilot
names and a bounded flight log alongside its numeric career and flag unlocks.

Nothing in Decay writes to storage. The store is in memory, marked dirty on
change, and the host writes it out on a cadence and before it stops — how often
someone's disk is touched is a decision about their machine. Writing the same
value again is not a change, so a game storing its volume every frame does not
keep a disk busy.

Editor Play keeps a save in memory for the editor's lifetime and never writes
it to disk: `Save.*` calls work and round-trip inside a session so persistence
can be play-tested, but putting a file into someone's project because they
pressed Play would be a side effect they did not ask for. Editor preferences
are a separate thing and stay separate.

</details>

## Randomness

<a id="seeded-randomness"></a>

### Seeded, portable randomness

A written-out PCG-XSH-RR generator: the same seed is the same run on every host, and Play restarts it.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅

**Tested by:** [`crates/sindri-core/src/random.rs`](../../crates/sindri-core/src/random.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

**Limits:**

- One stream is shared by every script, so a number drawn early shifts every number after it.
- Not a source of secrets: a handful of outputs reveals the state.

<details markdown="1">
<summary>Evidence</summary>

`sindri_core::Rng` is a **PCG-XSH-RR 64/32** generator written out rather than
depended on. Every general-purpose crate reaches the operating system for a
seed, which on `wasm32-unknown-unknown` means `getrandom` and a target that
refuses to compile without an opt-in — and more to the point, entropy is the
opposite of what this is for. A run that cannot be replayed from its seed is
not seeded at all.

Everything in it is integer arithmetic and the one division is by a power of
two, which is what makes a seed mean the same thing on every host. Fractions
are built from the top 24 bits, so `[0, 1)` is never `1.0`; bounded integers
reject the draws that would make the low values slightly more likely, because
modulo bias on a drop table over a long run is exactly the kind of wrongness
that gets blamed on the game design. Seeding takes two steps around the state,
so neighbouring seeds do not produce neighbouring first outputs.

The engine never asks the platform for entropy and does not pretend to: a host
that seeds nothing gets a fixed stream. The editor puts the stream back to its
seed on every fresh Play, so pressing Play twice gives the same run twice and a
bug found once can be found again; resuming from a pause deliberately does
not, since that would replay numbers the scene has already acted on. A game
that wants variety calls `Random.seed` with something it knows. A run's seed is
worth storing while a frame's numbers are not.

</details>

## Not yet

<a id="parallax"></a>

### Authored parallax

There is no authored parallax system.

Engine ❌ · Editor ❌ · Decay ❌ · Proof —
