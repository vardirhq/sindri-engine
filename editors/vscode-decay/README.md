# Sindri Decay for VS Code

Language support for `.decay` gameplay scripts.

The extension supplies syntax highlighting and starts the repository's `decay-lsp` language server. The server uses the same `sindri-decay::environment()` as the runtime, so host members accepted by the game are the members offered by completion and checked by diagnostics.

## Sindri-managed installation

Sindri owns the normal installation path. When a `.decay` asset is opened in the external editor, Sindri asks VS Code whether `vardir.sindri-decay` is installed. If it is missing, Sindri installs the extension that belongs to the current Sindri build before opening the script.

Packaged Sindri distributions place `sindri-decay.vsix` beside the editor executable or under `share/sindri/`. Source/development builds need no manual VSIX setup: Sindri finds `editors/vscode-decay` in the checkout, builds the matching release `decay-lsp`, runs the pinned extension checks/package step, installs the resulting VSIX, and then opens the requested script. The first open can therefore take a little longer; subsequent opens use the already-installed extension.

`SINDRI_DECAY_VSIX=/path/to/sindri-decay.vsix` remains an explicit override for unusual package layouts and testing, not a normal setup requirement. `SINDRI_VSCODE` similarly overrides VS Code executable discovery.

The packaged path never needs Rust, Cargo, npm, or a separately installed `decay-lsp`: each platform VSIX carries its matching language-server executable. Those build tools are only used by the automatic bootstrap when Sindri itself is running from a source checkout.

## Install a packaged extension manually

The same VSIX can still be installed directly with **Extensions: Install from VSIX...** in VS Code, or with:

```sh
code --install-extension sindri-decay.vsix
```

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

`SINDRI_DECAY_LSP` remains the highest-priority language-server override. An explicitly changed `decay.server.path` is next. Otherwise a packaged extension uses `bin/decay-lsp` (`bin/decay-lsp.exe` on Windows), falling back to `decay-lsp` on `PATH` for source-tree development.

## Development setup

For extension-only development, build the server from the repository root:

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

Double-click a `.decay` asset in Sindri's Project panel, or right-click it and choose **Open in External Editor**. Sindri looks for `code`, `code-insiders`, then `codium`, ensures its matching Decay extension is installed, opens the project directory and focuses the script. A missing external editor is reported in Sindri and does not prevent editing or playing the project there.

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
