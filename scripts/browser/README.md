# Running the engine in a browser

> **Historical first-run note.** This records the cube proof that found the
> original browser-host failures. Current static exports fetch manifests and
> content-hashed assets, and Gather, Orbital, Shapes Lab, and Weave
> run through the shared browser host; see `docs/export.md`.

The engine compiled for `wasm32` for several releases, CI checked it every time,
and nobody had ever loaded the page. Two things were broken the whole while, and
neither is the kind of thing a compile check can find:

- **A failure in a browser was silent.** `run` hands the event loop to the page
  with `spawn_app` and returns `Ok` immediately, so the error a host recorded had
  nobody to return to. The engine stopped at the device request and said nothing.
- **The surface refused every canvas.** A browser canvas offers `bgra8unorm` and
  no sRGB format at all, and the engine took that to mean colours could not be
  encoded. See `docs/rendering-color.md`: they can, through a view format.

So this exists.

```sh
npm install --prefix scripts/browser
wasm-pack build examples/cube --target web --out-dir pkg
node scripts/browser/smoke.mjs examples/cube target/browser.png
```

It serves the example, opens it in Chromium with WebGPU asked for explicitly,
waits for the device request, and exits non-zero if the page did not start the
engine. The tell is the canvas: one the engine never configured keeps the HTML
default of 300x150, which is the difference between "the page loaded" and "the
engine started".

`CHROME_PATH` points it at a browser that is already installed, for environments
that ship one rather than letting Playwright download it.

The platformer goal smoke reads terrain and the goal position from the exported
scene and plays with keyboard input. It brakes near the flag so a jump over the
sensor can land there; only the read-only observer's win counts as success.
Run its input-sequence regression with `node --test scripts/browser/*.test.mjs`.

The Orbit Camera Lab capture regression uses full Chromium's headless mode
(`channel: 'chromium'`), which `playwright install chromium` installs alongside
the headless shell. The shell does not deliver the expected relative motion
from Playwright's mouse moves while captured. The regression keeps the exact
1420-by-60 displacement assertion, release checks, sandbox denial and game goal:

```sh
SINDRI_BASE_PATH=/examples/orbit/ node scripts/browser/pointer-lock.mjs \
  target/dist/orbit target/browser-orbit-pointer-lock.png
```

## What the first run proved

The module instantiated, `run` executed, winit adopted the page's canvas, a
WebGPU adapter and device opened, the surface configured, and the frame pipeline
drew the cube in the same colours as native.

At that snapshot the cube still embedded its texture and ran no Decay. Those
were real gaps then. They are closed by the shared project host and static
export path: browser smoke tests now load the generated manifest and hashed
assets, run Decay gameplay, verify audio promises, and exercise readable
capability failures under the GitHub Pages subpath.
