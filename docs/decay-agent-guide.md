# Decay agent guide

Use this checklist before an AI agent edits a `.decay` script, a scripted prefab,
or the Sindri host surface. The full language and host contracts remain in
[`decay/LANGUAGE.md`](../decay/LANGUAGE.md),
[`docs/scripting.md`](scripting.md), and
[`docs/generated/decay-api.md`](generated/decay-api.md).

## Before writing code

1. Read the existing script, its scene or prefab, and the host API entry it uses.
2. For repository work, run `scripts/preflight.py`; it automatically includes every
   changed `.decay` file in the typed batch check alongside applicable Rust gates.
   To run only the Decay checker, compile the changed scripts against Sindri's real
   typed environment:

   ```bash
   cargo run --quiet --package decay-lsp -- --check path/to/changed.decay
   ```

   Multiple files and directories are accepted. Directories are searched
   recursively; `.git`, `target`, and `node_modules` are skipped.
3. Run the gameplay regression that observes the behavior, not only the compiler.
4. Run `cargo fmt --all --check` when Rust, generated files, or workflow code
   also changed.

The preflight exits nonzero for syntax and semantic errors. Runtime-contract
reminders are warnings because some valid uses require context the checker
cannot prove.

## Authored properties are not live state

This is the most important Sindri/Decay distinction.

| API | What it means | Valid timing |
| --- | --- | --- |
| `World.set_property(entity, name, value)` | Authors an exported field on the target script component | After spawning and before that script starts |
| `World.property_number(entity, name)` | Reads the numeric value authored on the script component | Any time, but it does not read the script's current field |
| `World.send_signal(entity, name, value)` | Sends runtime data to a running script | During gameplay |
| `World.take_signal(name)` | Consumes accumulated runtime signal data on the receiving script | During gameplay |
| `Name.on(entity).field` | Reads or writes another script's live field, checked when it compiles | Any time; before it starts, a write sets its starting value |
| `Name.on(entity).message(args)` | Sends a typed message, delivered after the pass | During gameplay |
| `Event.emit(args)` / `on Event(args) { }` | Emits a declared event to every script handling it, delivered after the pass | During gameplay |
| `Timer(seconds)` in a field, read with `.done` / `.left` / `.progress` | A countdown that runs down on its own before each `update` | Any time |
| `shared fn name(...)` at the top of any `.decay` file (a plain top-level `fn` is that file's only) | A helper every script calls by name, with no `this` | Any time |
| `const LIMIT: f32 = 3.0;` / `shared const VIEW_SIZE: f32 = 11.0;` at the top of a file | A value worked out when the project compiles, the same everywhere; a scene cannot tune it | Any time |
| `struct Card { name: String, weight: f32 = 1.0 }`, `Card(name: "Arc")`, `card.weight` | Values that belong together, as one; copied where assigned; a field with a default may be left out | Any time |
| `fn heavier(other: Card) -> bool { return this.weight > other.weight; }` after a struct's fields, `card.heavier(best)` | A function asked of one value; `this` is a copy, so return a changed one rather than writing `this.x` | Any time |
| `const HOT: Color = Color("#ff8800");`, `this.sprite.tint = HOT.lerp(c, t)`, `this.sprite.tint.a = 0.5` | A colour, held and assigned whole or by channel; the engine's tints, fills and strokes are this type | Any time |
| `fn find() -> f32? { ... return null; }`, `x ?? 0.0`, `if x == null { return; }` | A value that may be missing, instead of `-1`; used as its type only after a fallback or a check on a `let`/parameter | Any time |
| `var m: Map<String, f32> = [:];`, `m["arc"] = 1.0`, `m.get(k, 0.0)`, `for k in m.keys()` | Values found by key, in the order keys were first set; `m[k]` for a key it must have | Any time |
| `var xs: List<f32> = [];`, `xs.push(v)`, `for i in 0..n` | A list the script owns, changed in place; a range walked without building one | Any time |
| `"Score " + score`, `s.contains("x")`, `s.slice(0, 3)` | Joins text with numbers, flags, vectors and variants; asks text by character | Any time |
| `n.fixed(2)`, `n.padded(2)` | A number as text with exactly 2 decimals (`"12.50"`), or whole and led with zeros (`"05"`) | Any time |
| `enum Phase { Lobby, Play }` in any file, `match p { Phase.Lobby => { } _ => { } }`, `let s = match p { Phase.Lobby => "wait", _ => "go" };` | A named set of values in place of numbers; `match` must cover every variant or end with `_`, and gives a value where one goes | Any time |
| `Game.field` (declared with `state Game { var field: f32 = 0.0; }`) | Reads or writes a value the whole game shares, checked | Any time |

Prefer the typed forms in the last sixteen rows: a misspelt name or a wrong value
is a compile error rather than a silent fallback.

Do not use `set_property` as a setter for a running script:

```decay
// Wrong once part's script has started: the host rejects the write.
World.set_property(part, "leader_x", x);
```

Configure a newly spawned entity before it is allowed to start:

```decay
let part = World.spawn("prefabs/body-part.prefab.json");
World.set_property(part, "group", group);
World.set_property(part, "leader", this.entity);
```

For later changes, write the field through the script's type, or send it a
message and let it update its own state:

```decay
BodyPart.on(part).leader_x = x;
BodyPart.on(part).follow(x);
```

A read has the same boundary. If a script changes its own `health` field,
`World.property_number(entity, "health")` still describes authored component
data; it does not inspect that live field.

## Runtime behavior the compiler cannot prove

- Script updates and physics integration happen at defined frame boundaries.
  Check the host contract before assuming a transform or velocity write is
  visible in the same update.
- Signals accumulate until consumed. Confirm whether the receiver should sum,
  clamp, or treat the value as an event.
- A spawned script does not automatically inherit the spawner's runtime state.
  Author its exported startup fields before it starts, then communicate through
  explicit runtime APIs.
- A follower should consume the leader's sampled positions over time. Copying
  the leader's current translation every frame produces rigid simultaneous
  movement, not a trailing body.
- Splitting or re-parenting a chain must rebuild ownership and history for both
  resulting chains. Do not reuse one mutable history buffer for two leaders.
- Boundary movement needs a gameplay route, not merely a legal coordinate.
  A hazard that patrols only the screen edge may be technically active and still
  pose no threat.

These are behavior contracts, so add or update a deterministic runtime test when
they matter. Static checking cannot establish that a boss route is threatening
or that a segmented body follows the intended historical path.

## Syntax habits that avoid common failures

- Copy the shape of a compiling nearby Decay script before inventing syntax.
- Treat `decay/LANGUAGE.md` as authoritative; Decay is not Rust, JavaScript,
  or GDScript.
- Use only names and signatures listed in
  `docs/generated/decay-api.md`. Remembered APIs are not evidence.
- Keep changes small enough that the preflight diagnostic points at one idea.
- Do not repair only the first compiler message. Re-run the complete changed
  script set after every fix.

## Definition of done

A Decay change is ready to push only when:

- the batch preflight passes for every changed `.decay` file;
- runtime-contract reminders have been consciously checked;
- the relevant gameplay/runtime regression passes;
- the final diff contains the required scene, prefab, documentation, and
  generated API updates; and
- the behavior was observed on the surface it is meant to prove.
