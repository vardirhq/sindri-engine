# Sindri Decay for VS Code

Language support for `.decay` gameplay scripts.

The extension supplies syntax highlighting and starts the repository's `decay-lsp` language server. The server uses the same `sindri-decay::environment()` as the runtime, so host members accepted by the game are the members offered by completion and checked by diagnostics.

## Development setup

Build the server from the repository root:

```sh
cargo build --package decay-lsp
```

Install this extension's pinned dependencies and run its manifest, grammar, and
project-discovery checks:

```sh
cd editors/vscode-decay
npm ci
npm test --if-present
npm run check
```

Open `editors/vscode-decay` in VS Code, press **F5**, and open a `.decay` file
in the Extension Development Host. The client walks upward from that file to
the nearest `sindri.toml` and gives that whole project to `decay-lsp`, including
when Sindri launched VS Code for one file.

The server executable is selected in this order: `SINDRI_DECAY_LSP`, the
`decay.server.path` VS Code setting, then `decay-lsp` on `PATH`. A startup
failure displays those remedies rather than failing silently.

## Opening from Sindri

Double-click a `.decay` asset in Sindri's Project panel, or right-click it and
choose **Open in External Editor**. Sindri looks for `code`, `code-insiders`,
then `codium`, opens the project directory and focuses the script. Set
`SINDRI_VSCODE` to an executable path when none of those commands names your VS
Code installation. A missing external editor is reported in Sindri and does
not prevent editing or playing the project there.

## Current language features

- live syntax and semantic diagnostics
- completion for Decay keywords, script members, Sindri globals, and typed host members
- hover signatures/types (for the symbols the current server resolves)
- document symbols for scripts, components, fields, and functions
- scene-aware entity-name completion inside `World.find("...")`
- project-aware audio asset completion inside `Audio.play("...")` and `Audio.loop("...")`
- project script types, events, and shared state across files
- TextMate syntax highlighting and bracket/comment/indentation behavior

The watcher refreshes project scripts and authored scene/asset data. Sindri's
existing file watcher recompiles a saved script and reports success or compiler
diagnostics; the extension does not implement a second compiler or reload path.

Not yet supported are go to definition, references, rename, signature help,
formatting, semantic highlighting, code actions, workspace symbols, and a
debugger. Those belong to `decay-lsp`, not TypeScript substitutes in this
client.

For a local package, install `@vscode/vsce` through `npm ci` and run
`npm run package`. This creates a VSIX for testing; this repository does not
publish it. Cross-platform packaging and marketplace release remain later
distribution work.
