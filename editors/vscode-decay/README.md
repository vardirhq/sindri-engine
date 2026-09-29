# Sindri Decay for VS Code

Language support for `.decay` gameplay scripts.

The extension supplies syntax highlighting and starts the repository's `decay-lsp` language server. The server uses the same `sindri-decay::environment()` as the runtime, so host members accepted by the game are the members offered by completion and checked by diagnostics.

## Install a packaged extension

Release packages are self-contained: each VSIX carries the `decay-lsp` executable for the platform it targets. Installing the VSIX is therefore enough to get syntax highlighting, diagnostics, completion and hover; Rust, Cargo and a separate PATH setup are not required.

For a locally built package, build `decay-lsp`, copy it into `bin/`, then package the extension:

```sh
cargo build --release --package decay-lsp
mkdir -p editors/vscode-decay/bin
cp target/release/decay-lsp editors/vscode-decay/bin/decay-lsp
cd editors/vscode-decay
npm ci
npm run check
npm run package
```

Install the resulting `.vsix` with **Extensions: Install from VSIX...** in VS Code.

`SINDRI_DECAY_LSP` remains the highest-priority override. An explicitly changed `decay.server.path` is next. Otherwise a packaged extension uses `bin/decay-lsp` (`bin/decay-lsp.exe` on Windows), falling back to `decay-lsp` on `PATH` for source-tree development.

## Development setup

Build the server from the repository root:

```sh
cargo build --package decay-lsp
```

Install this extension's pinned dependencies and run its manifest, grammar, and project-discovery checks:

```sh
cd editors/vscode-decay
npm ci
npm test --if-present
npm run check
```

Open `editors/vscode-decay` in VS Code, press **F5**, and open a `.decay` file in the Extension Development Host. The client walks upward from that file to the nearest `sindri.toml` and gives that whole project to `decay-lsp`, including when Sindri launched VS Code for one file.

## Opening from Sindri

Double-click a `.decay` asset in Sindri's Project panel, or right-click it and choose **Open in External Editor**. Sindri looks for `code`, `code-insiders`, then `codium`, opens the project directory and focuses the script. Set `SINDRI_VSCODE` to an executable path when none of those commands names your VS Code installation. A missing external editor is reported in Sindri and does not prevent editing or playing the project there.

## Current language features

- live syntax and semantic diagnostics
- completion for Decay keywords, script members, Sindri globals, and typed host members
- hover signatures/types (for the symbols the current server resolves)
- document symbols for scripts, components, fields, and functions
- scene-aware entity-name completion inside `World.find("...")`
- project-aware audio asset completion inside `Audio.play("...")` and `Audio.loop("...")`
- project script types, events, and shared state across files
- TextMate syntax highlighting and bracket/comment/indentation behavior

The watcher refreshes project scripts and authored scene/asset data. Sindri's existing file watcher recompiles a saved script and reports success or compiler diagnostics; the extension does not implement a second compiler or reload path.

Not yet supported are go to definition, references, rename, signature help, formatting, semantic highlighting, code actions, workspace symbols, and a debugger. Those belong to `decay-lsp`, not TypeScript substitutes in this client.
