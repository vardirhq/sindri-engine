# AI knowledge pack

Markdown files a chat model is given as its knowledge so it can make a Sindri
game without having seen Sindri: a custom GPT's knowledge files, a chat
project's files, or any agent limited to file search.

```bash
python site/build_knowledge_pack.py                 # target/knowledge-pack/
python site/build_knowledge_pack.py target/pages/knowledge-pack
```

Pages builds it on every run and publishes the files, an index page and
`sindri-knowledge-pack.zip` at `https://sindri.vardir.no/knowledge-pack/`.
To use it, upload the fifteen `.md` files as the model's knowledge and tell it
to read `00-start-here.md` first.

## What is in it

`START-HERE.md` in this folder is the only hand-written page: the project
layout, how to write files the way the editor does, the workflow, and an index
of the other files. Its placeholders are filled from the engine: the scene
format version, the editor's New Scene built from registered defaults, and the
Decay reference's *Surprising behaviour* and *What does not exist* sections,
copied verbatim.

Every other file is assembled from documents already in the repository, listed
in `DOC_FILES` and `EXAMPLE_FILES` in the builder:

- the Decay language reference and the generated host API, as they are;
- `04-components.md`, rendered from `docs/generated/sindri-capabilities.json`,
  with each component's smallest real use found in the example projects;
- the subsystem documents, grouped by topic, with links rewritten to name the
  pack file that holds their target;
- the feature examples in `examples/`, every text file whole, and Flappy as the
  one complete game, for wiring rather than design.

A listed document that no longer exists fails the build, so a rename cannot
leave a silent gap. To change what the pack says, change the source document;
to change what it holds, change the lists in the builder.

## Known gaps

Most subsystem documents are engine contracts written for people working on
Sindri, so they explain Rust internals a game author does not need. Weave's
properties come from the generated `docs/generated/weave-reference.md`, but its
selectors, states and units are only as complete as the hand-written
`docs/weave-reference.md`.
