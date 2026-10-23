# Screenshots

Every image here is produced by a capture binary in this repository rather than
by a screen grab, so it can be regenerated after a change instead of slowly
drifting away from what the engine actually draws.

Run all of these from the repository root.

## The editor

Needs a display; `xvfb-run` provides one on a machine without.

```
xvfb-run --auto-servernum --server-args="-screen 0 1600x1200x24" \
  ./scripts/capture-editor.sh site/screenshots/editor-cube-scene.png

xvfb-run --auto-servernum --server-args="-screen 0 1600x1200x24" \
  ./scripts/capture-editor.sh site/screenshots/editor-orbital-scene.png \
  games/orbital-baked/assets/orbital.scene
```

The second argument is the scene to open. `editor-orbital-scene.png` uses it to
photograph the editor holding a whole game — a seventy-nine entity hierarchy and
a real asset tree — rather than the two-cube demo, because that is what the
editor is for.

## The games

`orbital-capture` and `orbital-baked-capture` take an output path, a width, a
height, and which moment to photograph: `title`, `combat` (the cast, before the
first upgrade covers the field), `spectacle` (a deliberately excessive build,
for the effect vocabulary), or `playing` (twelve seconds in, with the upgrade
chooser up).

`orbital-baked-capture` also takes `boss-0` through `boss-11`, which open a boss
rush on that boss and photograph the fight seven seconds in. Bosses otherwise
arrive on a timer, so photographing the twelfth one meant playing for most of an
hour.

```
cargo run -p orbital-baked --bin orbital-baked-capture -- \
  site/screenshots/orbital-baked-spectacle.png 1440 900 spectacle
```

The rest of Last Stand's pictures show its screen UI, which Weave styles, so
they come from `project-capture`: the same session and stylesheets the browser
runs, played through the steps that follow the size (`click:<entity>`,
`key:<Key>`, `type:<text>`, `wheel:<entity>:<px>`, `set:<board>=<value>`,
`wait:<seconds>`).

```
C="cargo run --release -p sindri-causeway --bin project-capture -- games/orbital-baked"
$C site/screenshots/orbital-baked-title.png 1440 900
$C site/screenshots/orbital-baked-combat.png 1440 900 click:title-start wait:5
$C site/screenshots/orbital-baked-upgrade.png 1440 900 click:title-start wait:3 set:cores=40 wait:1
$C site/screenshots/orbital-baked-phone.png 390 844 click:title-start wait:5
$C site/screenshots/orbital-baked-aegis.png 1440 900 \
  set:rush_from=11 wait:0.1 click:title-rush wait:7

cargo run -p sindri-causeway --bin causeway-capture -- site/screenshots/causeway.png
cargo run -p sindri-cube --bin capture -- site/screenshots/scene-frame-pipeline.png
```

## The play pages' posters

The genre showcases and feature examples are photographed by the same
`project-capture`, at 1440x900, so a page's poster is what its game draws.
`low-tide.jpg` and `voxel-lab.jpg` are resized to 1200x750 JPEG, because as
PNGs their terrain is over a megabyte each.

```
C="cargo run --release -p sindri-causeway --bin project-capture --"
for p in games/flappy examples/tween examples/camera examples/spatial \
  examples/physics examples/input examples/audio examples/sprite-shaders \
  examples/sprite-masks examples/ui examples/transform examples/orbit \
  examples/character3d examples/model-collision examples/lighting3d \
  games/low-tide games/voxel-lab; do
  $C $p site/screenshots/$(basename $p).png 1440 900 wait:3
done
$C games/scorchball site/screenshots/scorchball.png 1440 900 wait:3
$C examples/sequence site/screenshots/sequence.png 1440 900 wait:2.2
$C examples/daynight site/screenshots/daynight.png 1440 900 wait:6.3
$C games/platformer site/screenshots/platformer.png 1440 900 \
  hold:ArrowRight wait:1.4 release:ArrowRight wait:0.4
convert site/screenshots/low-tide.png -resize 1200x750 -quality 82 site/screenshots/low-tide.jpg
convert site/screenshots/voxel-lab.png -resize 1200x750 -quality 82 site/screenshots/voxel-lab.jpg
```

Scorchball is photographed on its title screen: a match starts only when a
pad joins, which a capture cannot press. On a machine without a GPU, Mesa's
software Vulkan driver (`mesa-vulkan-drivers`) is enough to run these.

## The two without a capture binary

Shapes Lab is assets and tests with no crate of its own, and Weave POC's
crate has no capture binary, so these two are photographed the way CI already
runs a project in a browser. Export the project, then point the smoke at it:

```
cargo run -p sindri-export --bin sindri-export -- \
  games/shapes-lab target/dist/shapes-lab --base /sindri-engine/
cp -R crates/sindri-player/pkg/. target/dist/shapes-lab/pkg/
SINDRI_EXPECT_ASSETS=1 SINDRI_BASE_PATH=/sindri-engine/ \
  SINDRI_VIEWPORT_WIDTH=1440 SINDRI_VIEWPORT_HEIGHT=900 \
  node scripts/browser/smoke.mjs target/dist/shapes-lab \
  site/screenshots/shapes-lab.png
```

The same two commands with `weave-poc` produce `weave-poc.png`. Both need
`crates/sindri-player/pkg` built first (`wasm-pack build crates/sindri-player --target web --out-dir pkg`).

The phone-shaped captures are 390x844 because that is the viewport
`it_fits_a_phone` holds the game to; they are the same game at the size the
layout is actually asserted against.

## What is deliberately not here

A screenshot of the browser build. The games render identically in a browser and
natively — that is the point of one renderer — so a browser capture would show
the same pixels with more machinery behind it. What a browser capture is good
for is proving that claim, and that belongs in a test rather than on a page.

## The Sound Lab

`sound-lab.png` is the browser Sound Lab on its effects designer, taken by the
same smoke test Pages runs, from a wasm-pack build of `tools/sindri-chiptune`:

```
wasm-pack build tools/sindri-chiptune --target web --release --out-dir pkg
mkdir -p target/sound-lab && cp tools/sindri-chiptune/web/index.html target/sound-lab/
cp -R tools/sindri-chiptune/pkg target/sound-lab/
node scripts/browser/sound-lab.mjs target/sound-lab target/sound-lab-captures
cp target/sound-lab-captures/sound-lab-desktop-sfx.png site/screenshots/sound-lab.png
```
