<!-- Generated from docs/capabilities/*.toml by `cargo run -p sindri-capabilities -- --write`. Edit the sources, not this file. -->

# What Sindri can do

An inventory of what exists and what is missing. `ROADMAP.md` says what is
planned and in what order; `docs/parity.md` judges Sindri against what an
engine is expected to do; this says what is true today, and what proves it.

It exists because those are easy to confuse. A roadmap full of ticked boxes
reads like a capable engine, and an editor full of buttons looks like a working
tool. Twice now something has been described as working because it was written
down rather than because it ran.

## Keeping this current

The inventory is data: one TOML file per area in `docs/capabilities/`, from
which this index and each area's page are generated. **Edit the TOML in the
same commit as the change it describes**, then regenerate with
`cargo run -p sindri-capabilities -- --write`; `cargo test --workspace` fails
when a page is stale.

The generator refuses an entry that:

- names a test, proof project or document that does not exist;
- claims ✅ or 🟡 on any surface with no test and no proof project behind it,
  unless it says in `untested` how it was exercised instead — and then the page
  says so in a warning;
- names a `parity` row that `docs/parity.md` does not have, or whose
  Engine / Editor / Decay / Proof cells disagree with its `status`.

Update an entry when a change adds, removes or wires up a capability, turns
something under *Not yet* into something that is, or finds that an entry is
wrong. Every entry should be something someone ran, not something read in a
roadmap or inferred from a type signature. If you cannot demonstrate an entry,
correct it or delete it.

## Areas

| Area | Capabilities | Not yet | No automated test |
| --- | :-: | :-: | :-: |
| [Engine core](capabilities/core.md) | 7 | 1 | 0 |
| [Input and audio](capabilities/input-and-audio.md) | 9 | 1 | 0 |
| [Physics](capabilities/physics.md) | 36 | 8 | 0 |
| [Effects, saves and randomness](capabilities/runtime.md) | 4 | 1 | 0 |
| [Screen UI and Weave](capabilities/ui.md) | 7 | 1 | 0 |
| [Grids and voxels](capabilities/grids-and-voxels.md) | 10 | 3 | 0 |
| [Hosts and rendering](capabilities/rendering.md) | 33 | 4 | 0 |
| [Assets](capabilities/assets.md) | 5 | 0 | 0 |
| [The editor](capabilities/editor.md) | 37 | 6 | 1 |
| [Decay](capabilities/decay.md) | 23 | 8 | 0 |
| [Games, labs and tooling](capabilities/projects.md) | 9 | 0 | 0 |
| **All** | **180** | **33** | **1** |
