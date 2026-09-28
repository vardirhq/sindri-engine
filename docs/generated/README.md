# Generated capability documents

Nothing in this directory is written by hand. Every file is produced by

```bash
cargo run -p sindri-capabilities -- --write   # rewrite
cargo run -p sindri-capabilities              # check; exit 1 when stale
```

and `cargo test --workspace` fails when a file here disagrees with the code it
was generated from. Editing one by hand is therefore not a way to change what
Sindri offers; it is a way to fail CI.

| File | Answers | Read from |
| --- | --- | --- |
| `decay-api.json` | What gameplay code may I write? | `sindri_decay::environment()` |
| `decay-api.md` | The same, for a person | the same model as the JSON |
| `decay-language.json` | What is the language itself? | the lexer, parser, analyzer and runtime tables |
| `sindri-capabilities.json` | What may I author? | the built-in `ComponentSchemaRegistry` |

## Why generated

Both questions already had exactly one authority in the repository, and neither
was readable without opening Rust.

The Decay surface is described once in `crates/sindri-decay/src/surface/`
because the analyzer and the runtime host must not disagree — a path one accepts
and the other cannot answer is a clean compile followed by a failure on frame
one. `environment()` turns that description into the types the analyzer checks
against, and this is the same call, asked to write down what it found.

The components are registered once in `crates/sindri-scene/src/extract/`, with
the distinction that registry exists to keep: **fields** are what a component
has, a **default payload** is what a fresh one is, and only some types have one.

A hand-maintained reference for either would be wrong a release later, and
wrong in the direction that matters: it would describe calls that do not exist.

## The language

`decay-language.json` describes the language rather than the engine: its
keywords and contextual words, what may start an item, its attributes, every
operator with its precedence and grouping, the built-in types and how each is
spelled, `Vec2`/`Vec3`/`Timer` construction, what a vector, a timer, text and
a list can be asked (with each operation's parameters and result), the
lifecycle functions Sindri calls, and the runtime's limits.

Each list is the table the compiler reads: the lexer's `KEYWORDS` and the
parser's operator tables in `decay_syntax::vocabulary`, the analyzer's
`BUILT_IN_TYPES` and the member signatures in `decay_semantic::members` (which
decay-lsp's completion reads too), `sindri_decay::LIFECYCLE`, and the
runtime's limit constants. Its first key is `"schema": "decay-language/1"`; a
reader should refuse a schema it does not know rather than guess.

`diagnostics` lists every error the compiler can report by its stable code —
the phase and a name joined, such as `decay-semantic/unknown-name` — with a
one-line summary. The code is what `decay-lsp` reports, live and in
`--check`, and it is never reworded or reused, so a page, a test or a fix can
key on it where a message would drift. The tables are `SyntaxCode::ALL` in
`decay_syntax::codes` and `Code::ALL` in `decay_semantic::codes`, and every
diagnostic the parser or analyzer raises names one.

## Descriptions and parameter names

Every name in `decay-api.json` carries a `description`, and every call its
`parameter_names`; every host type has a `description` too. They are written
for someone who has never used Decay: what a thing is for, in plain words,
with a short example where one helps. `decay-api.md` shows them beside each
signature, as `axis(negative: String, positive: String)`.

The surface registers only types, because that is what type-checking needs,
so the prose lives beside it in `sindri_decay::reference`. It is a separate
table, but it cannot drift silently: sindri-capabilities fails when a name on
the surface has no description, when a description names something no longer
on the surface, or when a call's parameter names do not match its arity. A new
host call therefore cannot ship undescribed. `docs/scripting.md` remains where
the reasons behind the design are explained.

## What is deliberately not here

**Anything about the editor.** These files describe the engine, not one client
of it.
