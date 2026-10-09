# Exporting a project to the web

```bash
cargo run -p sindri-export --bin sindri-export -- game dist --base /sindri-engine/
wasm-pack build game --target web --out-dir pkg
cp -R game/pkg/. dist/pkg/
```

The export writes the project; `wasm-pack` writes the host. They are separate on
purpose: building WebAssembly needs a toolchain the export does not want to own
or version, and a step that silently ran it would be one nobody could reproduce
by hand.

## What comes out

```text
index.html                 the page, with the base path and the project's name
pkg/                       the host, as wasm-pack built it
assets/sindri.manifest     what the project is made of — never cache this
assets/<content hash>/     every asset — cache this for ever
```

## What ships

Everything is worked out from the scene. A texture ships because a component
names it, a font because a text element does, a script because an entity runs
one, a sheet because it sits beside a texture that has one. So an asset that
stopped being used stops being carried, and one that started being used cannot
be forgotten.

Physics material component references ship their `.profile` assets from scenes
and prefabs, even when an entity starts inactive. Physics profiles are validated
before output is written. Native and browser project hosts deliver them through
the normal profile loader and resolve coefficients at the scene boundary; no
manual include list is required. See [the material contract](physics.md#reusable-collision-materials).

The exception is in `sindri.toml`:

```toml
[assets]
include = ["audio/pickup.wav", "audio/victory.wav"]
```

A script can name a clip at run time — `Audio.play("pickup.wav")` is a string
inside a program, and no walk of a scene can see it. Scanning script text for
anything that looks like a path would ship whatever a comment mentioned and miss
whatever was built from a variable, so a project says instead.

## Cache invalidation

**`assets/sindri.manifest` must never be cached. Everything under
`assets/<hash>/` can be cached for ever.**

The directory is named after what every asset in it hashes to, so a build
differs from another build exactly when something a player would download
differs. A changed asset cannot land in a directory anyone has already cached,
and an unchanged build keeps its name so a re-deploy re-downloads nothing.

The manifest is the one file that has to be re-fetched, because it is how a
browser learns which directory to look in. It is small.

For a static host, that is one rule each:

```
/assets/sindri.manifest   Cache-Control: no-cache
/assets/*                 Cache-Control: public, max-age=31536000, immutable
```

Exporting again removes the previous build's directory. Without that, every edit
would leave a whole copy of the project behind.

## Getting one onto GitHub Pages

`.github/workflows/pages.yml` exports every project the site serves and copies
the browser host in beside each one:

```bash
cargo run -p sindri-export --bin sindri-export -- \
  games/orbital-baked target/pages/examples/orbital-baked \
  --base "${PAGES_BASE_PATH}/examples/orbital-baked/"
cp -R game/pkg/. target/pages/examples/orbital-baked/pkg/
```

One host serves every project, because it reads the manifest rather than
carrying a list of one game's assets. The `--base` is the route the project
will be served from, which for a Pages project site includes the repository
name. The workflow gets that prefix from `actions/configure-pages`'s
`base_path` output before exporting: `/sindri-engine` for the default project
site, and an empty prefix for the custom domain. Thus the current live route is
`https://sindri.vardir.no/examples/orbital-baked/`, with
`--base /examples/orbital-baked/`. Do not infer the prefix from the repository
name: custom-domain Pages sites are served at the domain root.

The site used to copy an asset directory and write its own manifest instead.
That was a second answer to the question the export exists to answer, and the
two disagreed: the hand-written manifest named no kinds, the host reads kinds
to know what to ask for, and a project whose manifest names no scene is a
project that does not open. Nothing assembles a build by hand any more.

## Deployment, and the subpath

`--base` is where the export will be served from: `/` for a domain of its own,
`/repository-name/` for a GitHub Pages project site. It is baked into the page's
`<base href>` rather than guessed at run time, because a page that guessed is a
page that works locally and 404s once it is deployed.

The trailing slash is not optional and the export adds it: `<base href="/repo">`
resolves `pkg/host.js` against the *site* root, and `<base href="/repo/">`
resolves it inside the project. That is the whole GitHub Pages subpath problem.

## Shared browser host

The browser host reads the manifest and asks for what it names, kind by kind. It
used to carry a list of asset IDs per kind, compiled in — which meant adding a
texture meant editing Rust, and a project the host crate had never heard of
could not be exported at all. `AssetKind` in the manifest is what replaced that.

The shared bundle is currently built from the historical `sindri-causeway`
crate in `game/`, so its files are named `sindri_causeway.js` and
`sindri_causeway_bg.wasm`. That name does not identify the project being
loaded. Its manifest determines the scenes, assets and Decay scripts.

The execution host still has Causeway coupling: it uses that crate's session
and terrain setup and the fixed `sindri.causeway.save` browser storage key.
Extracting a generic project host, keeping Causeway terrain setup in its own
project, and isolating save storage per project remain follow-up work. Renaming
the bundle alone would not complete that separation.

Each kind has its own bounded asynchronous queue. The host sizes that queue from
the manifest before requesting the kind, while keeping the number of concurrent
fetches small. The bound therefore limits the project described by the manifest
instead of imposing the asset pipeline's default queue size as an undocumented
maximum number of textures, scripts, or other assets in an exported game.

## What a player sees while it loads

The page opens on the Sindri loading screen — the forge mark, the wordmark
and a moving bar — drawn in plain CSS and SVG, so it is there the moment the
page is, before any script, wasm or asset has arrived. That covers the whole
wait a browser build has: the host downloading, WebGPU starting, and the
project's assets arriving.

It stays until the host announces the game is on screen. The host dispatches
`sindri:ready` on `window` once, on the first frame it presents while the
application says it is ready (`DesktopApp::ready`; the browser game is ready
once its project is installed, not while it is still clearing the screen
waiting for assets). The Sindri mark shows for at least 0.9 seconds so a fast
load is not a flash. A failure removes it at once and shows the reason.

A project may follow the Sindri mark with its own brand, in `sindri.toml`:

```toml
[web.splash]
image = "brand/logo.png"   # relative to the project: png, jpg, webp, svg or gif
title = "Vardir Games"     # under the image, or in its place
caption = "presents"       # optional, smaller
background = "#101820"     # optional, #rrggbb; the Sindri ink otherwise
seconds = 1.5              # the least it shows, 0 to 10 (default 1.5)
```

It needs an image, a title or both. The image is written beside the page,
named by its contents so a changed image is never served from a cache. The
game is revealed once it is ready *and* the brand has had its time. A setting
that is not one — an unknown key, a colour that is not `#rrggbb`, a time
outside 0 to 10 seconds — fails the export with the reason. With
`prefers-reduced-motion`, nothing on the loading screen moves.

## What a browser is told when it cannot run this

The page checks for a canvas, for `navigator.gpu`, and for an adapter that
actually answers — because `navigator.gpu` existing is not the same as WebGPU
working, and Chrome on Android exposes the interface more widely than its
drivers can serve. Each failure produces a sentence a player can read rather
than a blank canvas. The engine's own failures arrive on `window` as
`sindri:failed` and are shown the same way.

`scripts/browser/smoke.mjs` runs against the exported directory in CI — the page,
the manifest, the hashed assets, and the deliberate removal of each capability
to prove the message appears. It also waits for the loading screen to give way
before touching the game, and fails if it never does: that is the proof the
host announces `sindri:ready` in a real build.

The host module is imported by the page when it starts rather than at the top of
the script, so a host that is missing or broken is reported like any other
failure instead of leaving the loading screen up for ever.

## What this does not do yet

- **No compression.** Assets ship as they are; a host that serves gzip or brotli
  will do it on the wire, and pre-compressing is a size decision nothing has had
  to make yet.
- **No unused-scene pruning.** One scene is exported: the one the project names.
  A project with several would need to say which ship.
- **The host crate is named by hand.** The export takes a module name because a
  project does not say which binary will run it.

## Static imported models

`sindri.model.asset` references are discovered from every listed scene and
expanded prefab, including inactive instances. Models ship once as manifest
kind `model`; their GLB bytes are preserved rather than converted into scene
vertices. The exporter validates the bounded static subset before output, so
missing or unsupported/malformed models fail with the logical asset named.
Both project-root and `assets/` layouts are covered. See
[imported models](imported-models.md) for native/browser proof and limitations.
