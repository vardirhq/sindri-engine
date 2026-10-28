- Put the dusk art in Lamplighter's playable Dusk Road: the meadow tiles and normal map, scattered decorations, parallax sky and hills, dusk lighting, and Wick's baked side-view sprite. The level keeps its existing collision layout while the old showcase mechanisms are moved into their planned levels.

# Changelog

- A plain `cargo build` or `cargo test` now builds with line tables instead of full
  debug info, and dependencies with none, which CI already did through
  `CARGO_PROFILE_*_DEBUG`. Backtraces keep their file and line; stepping through code in
  a debugger needs `CARGO_PROFILE_DEV_DEBUG=2`. On one crate's tests that is 1.6 GB of
  build output instead of 7.6 GB, and a third less time compiling.
- Lamplighter's gloom now hunt. Using `Lighting.level`, each creeps through the dark
  toward Wick, holds where a light begins, and from the edge winds up (low, wide and
  shaking, with red sparks) and pounces; a pounce that lands throws Wick back and
  scorches his lantern, which shrinks and gutters, and a few in a row put it out and
  start the field over. One that misses is spent and open to the sword. They thin where
  a light is strong and smoke violet motes. The field opens calm: they only hunt once Wick has first walked. `Game.truce` holds them still for tests.

- A thing with `sindri.shadow2d` is now lit as something standing on the ground: the
  question of whether a shadow covers it is asked where its feet are, not at each
  pixel. The occluder round a tree's or boulder's base no longer darkens the side of
  it that faces the light. `lit_from_base` on the component turns it off.
- Lamplighter's trees, boulders, lamps, Wick and the gloom now throw shadows on the
  ground: a soft contact ellipse under each, and for trees and boulders their own
  silhouette laid away from the nearest lamps and Wick's lantern, so a boulder's
  shadow is the shape of a boulder and falls away from the light.
- Add `Lighting.visible(from, to)` and `Lighting.level(point)` to Decay: ask the 2D
  lighting whether one world point sees another past the scene's occluders, and how
  bright a point is with every light and shadow applied. Built from every light and
  occluder in the world, not only those near the camera, so a game rule can use it;
  a scene with `lighting_2d` off says so.
- Lamplighter: dragging a finger down the screen now walks Wick down it, and a controller's
  left stick up walks him up. Both read down as positive, as the screen does, and Wick
  had taken positive as up.
- Lamplighter's lamps stand on cobblestone pads (a second `corners` terrain from a
  Pixel Lab tileset, laid from Decay with the plain ground made transparent so the
  paving lies on the dirt), the beacon's wider than the rest. Trees and boulders
  block light with an ellipse on their footprint instead of a box centered on their
  foot, so they no longer throw a rectangle under themselves, and the lamps' and
  Wick's light reach further past an edge so the face toward the light stays lit.
- Add `sindri.shadow2d`, the shadow things cast on the ground. A soft contact
  ellipse under a sprite, and, with `cast` on, the sprite's own picture laid on the
  ground away from the two strongest 2D lights near it: longer for a taller thing
  and a further or lower light, fainter where less light reaches its feet, and none
  from a light the thing carries. Both are painted over the lit ground and block
  nothing; a light is still blocked by an occluder. Needs `lighting_2d` on to cast.
- 2D light shadows are now exact. A shadow-casting light no longer keeps a table of
  256 distances by direction with a blur across three of them; it keeps the region it
  sees as sorted angular intervals, each with the line of the nearest blocking edge,
  so a shadow ends at the true angle of an edge's end at any distance and two walls
  meeting cast a true corner. The shader finds a pixel's interval by binary search and
  the CPU `Lighting2d::at` runs the same arithmetic; `sindri_render::sees` is the
  new yes-or-no query. A point light's shadow edge is now hard, and the five-sample
  area light is what softens it. `SHADOW_BINS_2D` is replaced by `SHADOW_INTERVALS_2D`
  and `SHADOW_MAP_LEN_2D`.
- The Pixel Lab client gains an `object` command (a map object, with the free
  promotion of the candidates worth keeping and their export), `animate` moves to
  the multi-direction endpoint with `--directions` so a walk or a slash is made for
  all four directions at once, and `call --wait` on a request that starts several
  jobs keeps every job's response instead of the last. Its README records what the
  first real runs showed: costs, canvas sizes and the unreliable tileset labels.
- Lamplighter's field gets real creatures and a lived-in ground: four Pixel Lab
  gloom (a smoking blob, a hunched one, one trailed by motes, a low crouch) that
  breathe and squash when struck, five dead trees and four boulders instead of one
  of each, and a scatter of tufts, night flowers, pebbles and glowing mushrooms over
  the grass, kept clear of the path and of everything solid. A sword blow now throws
  a gloom along the direction it was struck, not away from a point, and the ambient
  light is raised so the far trees read.
- Add `games/lamplighter`, the top-down action-adventure, starting with a dark
  field: Wick walks with a carried lantern that lights and shadows the world,
  swings a sword at a gloom, and lights three lamp posts, the last of which opens
  the way. The art is from Pixel Lab: a grass-to-dirt ground laid with the new
  `corners` terrain, Wick's four-direction walk and slash, trees, boulders and
  lamp posts. It is exported to the site at `examples/lamplighter/`, listed in the
  editor's samples, and scoped as its own game in CI.
- Add a `corners` terrain kind: sixteen sprites chosen by the terrain at a tile's
  four corners, the layout of a Wang or Tiled corner set and of what a tileset
  generator makes for two terrains. `Grid.set_terrain_corner(map, column, row,
  terrain, set)` sets or clears one corner and rewrites the up to four tiles that
  meet there; `Grid.set_terrain` refuses a corners terrain and says so. The editor
  brush does not offer corners terrains yet.
- Add `tools/pixellab/pixellab.py`, a standard-library command-line client for the
  Pixel Lab API: `balance`, `spec` (every endpoint, or one endpoint's fields, from
  the API's own OpenAPI document), `tileset`, `character`, `animate`, `download`,
  `job`, and `call` for anything else. It waits out the API's asynchronous jobs and
  its "still being made" answers, retries rate limits, saves inline images as PNGs,
  and takes the key from `PIXELLAB_API_KEY`. Tested against a local stand-in; the
  first real generation run is the first check of the real responses.

- Lamplighter becomes a top-down action-adventure, and the platformer it was
  turning into goes back to `games/platformer` as the side-view genre showcase,
  with its Dusk Road level and the title, level select, settings, pause and
  saved-progress shell it grew. Its crate, Pages route (`/examples/platformer/`),
  directory entry, CI scope, browser scripts and capability references are named
  for it again. `docs/lamplighter-plan.md` now plans the new game, which starts as
  a new project in `games/lamplighter`; `AGENTS.md` says so.

- Run focused checks and a phone-browser export for single-game pull requests.

- Add optional area-sampled 2D shadows for Lamplighter lanterns, with matching
  CPU visibility and GPU shading.

- Darken Lamplighter around Wick’s carried lantern and lit road lanterns,
  and lower Wick’s sprite slightly to ground both feet.

- Try the PixelLab Wick sprite and walking animation in Lamplighter.

- Give Lamplighter's portrait camera a smooth look ahead in Wick's travel
  direction, while keeping it inside the level bounds.

- Keep Lamplighter's camera framed inside Dusk Road at portrait and landscape
  sizes, and start Wick in view on a phone. Camera confinement can account for
  the current orthographic viewport instead of using fixed center limits.

- Keep Lamplighter's 16:9 gameplay camera within Dusk Road's level edges while
  following Wick.

- Resolve camera follow targets in their scene or prefab scope, so Lamplighter's
  camera follows Wick after entering the level from the title screen. Inactive
  cameras and targets no longer advance follow behavior.

- Add a responsive browser Normal Map Studio using the editor's shared generator.

- Add Particle Lab to author Sindri particle emitters in the editor and browser,
  with engine-backed motion preview, presets, bursts, and JSON export.

- Improve default Sound Lab generation with related Rise motifs, modal phrase
  cadences, coordinated arpeggios and smoother chord inversions. Save the
  composer version; older songs retain Legacy generation and accompaniment.

- Add explicit Sound Lab motif development, call-and-response arranging,
  performance expression, and related Exploration/Danger/Victory snapshots.

- Add Sound Lab bar-range regeneration, non-destructive ensemble auditioning,
  selected-note harmony inspection, and named snapshots saved with songs.

- Keep every Sound Lab layer enabled during seeded generation so section masks
  do not cut off musical phrases; retain seeded sounds and manual section controls.

- Seed Sound Lab instrument ensembles and per-section layer participation, with
  independent note, sound, and arrangement locks; preserve existing song mixes
  and add random seeds plus repeatable word/phrase seeds.

- Rebuild the Sound Lab arrangement around five selectable layers, including
  generated drum and chord previews, with a navy and amber track inspector,
  a fitted piano roll, visible velocities, and four-bar phone paging.

- The Sound Lab arrangement now has named Bass, Arpeggio, and Lead rows with
  direct lock, mute, and solo controls. Section headers show their bar ranges,
  and mobile places the arrangement and piano roll before the inspector.

- In the Sound Lab, make a lead melody variation with **Vary melody** (V). It
  keeps the rhythm, repeated phrases and resolving note; undo restores the
  previous melody.

- In the Sound Lab's music workstation, keep a bassline or other track with
  **Keep on Generate**, or replace just one part with **Regenerate track**.
  Regeneration keeps the song's harmony and the other tracks; the lock is
  session-only, so existing `.sindrisong` files remain unchanged. While locks
  are active, new parts stay in the current key, mood and harmonic plan.
  Regenerated tracks can be undone.

- Put the supplied pixel-art Lamplighter wordmark on the title screen.

- Lamplighter stage 4, Dusk Road: Wick lights checkpoint lanterns for
  sparks, a 2D light flare and respawn, collects optional embers, and reaches a
  beacon that saves the level. The level now loops the Tide Song and the
  scripted run plays from the title through the beacon.

- Add `Scene.restart(name)`: moves to a scene and starts it over. `Scene.go`
  finds a scene as it was left, which makes a pause screen a scene of its own,
  but a level begun again from a menu needs the opposite, so `restart` throws
  the scene away first and loads it from its file, whether it is the scene being
  played or one visited earlier. The editor's Scenes board draws its doors as it
  does `Scene.go`'s, and `docs/scripting.md` says which to use when.

- Lamplighter's shell, the third stage of its plan (`docs/lamplighter-plan.md`):
  a title screen, a level select that shows which beacons are lit and the embers
  found, settings (music and effects volume, reduced motion) and a pause screen,
  each its own scene, with the old level now `level-1.scene` behind them. Escape,
  a pad's Start button or a drawn button pauses a level without losing it;
  reaching the flag saves the beacon and returns to the level select; what a
  person did is kept with `Save`. The game now opens on its title screen.

- Lamplighter's art foundation, the second stage of its plan
  (`docs/lamplighter-plan.md`). `games/lamplighter/art/dusk/build.py` draws a
  dusk palette's meadow and harbor tile sets as 47-tile `blob` terrains with
  normal maps, water, sky and hill layers, decoration, the lantern with a
  stepped glow, and bakes Wick from a recipe it writes with the Sprite Baker.
  Two art scenes show it under the 2D lighting the game will use, a test opens
  them, and CI re-runs the script and fails if the checked-in art moves. The
  game itself is unchanged. The work found a gap: nothing keeps a pixel game's
  scale to whole multiples of the window, recorded in `docs/parity.md`.

- Move the platformer to `games/lamplighter`, the first stage of turning it into
  Lamplighter, a short real game (`docs/lamplighter-plan.md`). The crate, the
  Pages route (`/examples/lamplighter/`, `/play/lamplighter/`), the directory
  entry and screenshot, the editor's welcome list, CI and the browser scripts
  (`lamplighter-goal`, `prepare-lamplighter`, `check-lamplighter-batches`) are
  renamed, and the capability pages point at the new path. Nothing about how it
  plays changes. The scene file is still `assets/platformer.scene` until the
  game's shell replaces it.

- Open models that come as several files in the Sprite Baker. A `.zip` of a
  download (a `.gltf`, its `.bin` and a `textures/` folder), a dropped folder,
  **Open folder…**, or a `.gltf` chosen together with its `.bin` now just opens.
  A `.gltf` missing its `.bin` says which file to add, and the message stays on
  screen: before, it was replaced a moment later by the previous asset's "Baked"
  line, so a failed open looked like nothing had happened. Textures now give a
  model its colors, which a model with only base-color textures did not have: each
  triangle takes the texel under the middle of its UVs and the colors are cut to a
  chosen few (`texture_colors`, 1 to 16, default 8), each recolorable by name. The
  Model tab lists the colors the bake will use rather than the file's materials.

- Bake Blender models in the Sprite Baker: drop a `.glb`, `.gltf` or `.obj` on
  the page (or open one) and it is read by Three.js into triangles, centred on
  its footprint, stood on the floor, shaded into the baker's four-shade palette
  by each material's base color, and baked in four directions like any other
  asset. A recipe may now name a model with `"kind": "file"`, with `auto_fit`,
  `fill`, `scale`, `rotation_y`, `offset` and per-material color overrides; a
  recipe waiting for its model says so and takes it when it is dropped. The page
  follows IsoGame's Sprite Factory: a catalogue of assets, every direction on its
  floor, and an inspector with Render, Model and Recipe tabs, and on a phone its
  panes sit behind a bar along the bottom. The browser page now has one
  dependency, Three.js, pinned in `tools/isometric-baker/web` and used only as the
  loader; the baker has none, and the command line cannot read model files yet.

- Put the isometric baker on the website as a tool called Sprite Baker, at
  `/examples/isometric-baker/`: edit a recipe, see the sheet it bakes, and
  download the PNG, `.sheet` and prefab, or all three as a zip. Each direction
  is drawn where it will stand, on the tiles its footprint reserves with a ring
  of ground around them (diamonds, squares or a ground line by view), at a
  chosen zoom with the floor optional; sliders for supersample, alpha cutoff,
  padding, palette snap, outline, light angle and the shading bands edit the
  recipe text and bake again as they move. The layout follows IsoGame's Sprite
  Factory, the tool the baker was adapted from. It runs the tool's own sources
  in the page, so it bakes the same pixels as the command
  line; a bake now returns `Uint8Array` rather than a Node `Buffer`. The Sound
  Lab's web build still exports WAV only, because its Ogg encoder (libvorbis)
  cannot be built for `wasm32-unknown-unknown`.

- Show a thin health bar over a Hollow once it has been hurt, in place of its
  row of diamonds that was always there; one that dies to a single blow shows
  none.

- Run less CI for a pull request that only changes the companion game: when
  every changed file is a game script, asset, test, generator or its changelog
  and capability entry, CI runs the game's own tests, the Decay preflight,
  Clippy, the baker and the format checks, and skips the engine's workspace
  tests, render captures and browser runs. Any other file in the diff runs
  everything, and a game-only run leaves no verified record, so the push to
  main after the merge still runs in full.

- Fix Causeway sticking at the Lighthouse: going Onward after it dealt the
  warden offer four cards when only some wardens were called yet, the script
  failed, and the screen stayed blank. An offer, or a boon hand, now deals as
  many cards as it has.

- Generate uncharted islands for Causeway from each crossing's seed, in
  `generate.decay`, to each world's rules: a hump of ground highest in the
  west with low landing ground east, the beacon and houses above the tide
  with a way up to each, the wardens near what they guard, stones, vents and
  lava, or ice by world, and waves from the world's Hollow. An island that
  breaks a rule is thrown away and the next seed tried. Off the causeway and
  deep in the mountain and the ice, a fork offers an uncharted island beside a
  written one; the battle builds it at runtime from per-world column models,
  in its world's surroundings. The bot reports how generated islands play.

- Let Causeway's crossings open things for the next: four wardens answer the
  first muster, holding the Lighthouse calls the Harpooner, and making a
  crossing calls the Shieldback and opens spring tides, a harder crossing
  with one more Hollow every round and a smaller Hearth, chosen at the muster.

- Give each of Causeway's islands a bonus objective, rolled with its omen and
  followed in the HUD: lose no house, send three Hollow into the sea or the
  lava, destroy six, lose no warden, or lose no more than one Hearth. Meeting
  it earns one more boon, and a rewind puts its count back with the board.

- Hang an omen over each of Causeway's islands past the first, rolled when a
  crossing sets out and shown on the road's cards, the banner and the HUD:
  still water brings a Hollow fewer each round and no boon after; a Hollow host
  one more each round and two boons; a wreck ashore an upgrade for the boon;
  and lit windows rekindle the Hearth. A fork in the road is now a choice of
  risk and reward as well as of island.

- Give each of Causeway's wardens two upgrades that change how its skill
  works, won as boons between islands: the Bulwark's bash can shove two cells
  or ward the Bulwark, the Mason's stone can hurt everything beside it or fly
  further, the Lamplighter's bolt can pierce to a second figure or knock back
  two cells, the Tidecaller's water can fall three wide or draw figures in,
  the Harpooner's line can reach across the board or strike again, and the
  Shieldback can ward a whole group or hold the wardens beside it in place.
  The upgrades a warden has are listed on its card.

- Add a third world to Causeway, the Rime: past the Lighthouse the road divides
  between the burning mountain and the ice, and a crossing takes one. The Rime
  is four islands, Icebound Strand, then The Calving or Hoarwood, then The
  White Barrow. Its ground is snow and frost with pools of ice. Anything shoved
  onto ice skids on until something stops it or it goes into the sea, and the
  Tidecaller's water freezes where it lands. New Hollow: the Hailer lobs hail
  that leaves ice behind, and the Rimeback sweeps the three cells in front of
  it for 3 and ices them over. The ice has its own cold light, snow on the
  wind, bergs and floes in the sea, and its own song.

- Redesign the website and give every game and demo a page of its own at
  `/play/<route>/`, with the game embedded, what it is, its controls and what
  it proves. The game loads when Play is pressed and plays inline, or over the
  whole window with **Full page**, which is on by default on a phone; Back
  leaves full page. The landing page now shows the platformer, Scorchball,
  Low Tide and Flappy, which it had never listed, and its game grids are built
  from the site catalog so they stay in step with what is published. Its list
  of what Sindri lacks is taken from the parity table rather than an old list
  that still named lighting, glTF import and a language server. Every play
  page has a poster captured from the project itself.

- Dress Causeway's interface: the HUD's panels are framed in brass with
  corner brackets, the Hearth is a row of lanterns that go out as it is lost,
  and the turn's controls are plates that dim while they would do nothing.
  Each warden has a portrait, drawn by `tools/causeway_art.py` from its own
  model, on the card of the warden in hand and on its muster, boon and recruit
  cards. The fork in the road shows the whole route as a map of stops, the
  islands held in gold and the fork on offer bright.

- Muster Causeway's crossing: three of six wardens are chosen before setting
  out, and where the road enters the burning mountain a fourth joins, chosen
  from those left behind, or none, and the Hearth grows instead. Two new
  wardens: the Harpooner throws a harpoon down a line and drags the first
  thing it bites back to it, or itself to anything too heavy to drag; the
  Shieldback wards itself, a warden or a building against the next blow, and
  nothing shoves it. Boons are drawn from what the party can use.
- Fork Causeway's road: a crossing is six steps, and after Saltmarsh and again
  deep in the mountain the player chooses the next island on a screen that
  shows the whole road. The second fork leads to a new island, The Glass
  Stair, terraces of black glass where a shove down a step hurts.

- Add a second world to Causeway, the Cinderfast: three islands inside a
  volcano after The Lighthouse, ending with the Kiln. Lava rises there instead
  of the tide and burns what it reaches. Vents mark where they will spill it
  next. Water poured on lava cools it into a basalt column, which is new ground.
  The Tidecaller joins the wardens there and throws water: it chokes vents,
  cools lava, drenches fire-born Hollow and pushes the rest. New Hollow: the
  Cinder Imp, which leaves lava where it walks; the Burster, which throws its
  neighbours apart when it dies; and the Kiln, which lobs lava. The lake is a
  shader of crust and glowing cracks under falling ash. The world has its own
  song, and the music crossfades between worlds. The Hearth is rekindled when a
  new world begins.
- Mark Causeway's selected warden with a brass gem bobbing over its head.

- Add a floor-plan view to Causeway: T or the new Top button tips the camera
  up over the board to look straight down, squared up with the screen so the
  cursor's up key goes up it, and eases back the same way. Cells in pits and
  behind ledges can be seen and picked from there, and the quarter turns
  still work from above.

- Give Causeway's air more to it: smoke curls from every chimney with somebody
  home and bends with the wind, violet motes rise from each rift the Hollow
  will climb out of, and mist banks drift low over The Fold. Window and rift
  lights now ease on and off instead of switching. A house's light burns
  inside it and stays with that house, so it no longer glares off the walls
  or jumps to the next house when one falls; it gutters out instead. A rift
  flares as the Hollow climb out of it.

- Give each Causeway island its own mood and weather: a clear morning on
  Saltmarsh, mist and drizzle over The Fold, a gold afternoon on Quarry Rise and
  a night storm at The Lighthouse, with lightning, thunder and rain rings on
  the sea, and dusk on the title. The sun, sky, fog, grade, bloom, lamps,
  window and rift lights and the drifting pollen, dust, fireflies and embers
  all ease between them. The sea is a shader with shallows, swell, glints and
  foam. Islands sink and rise rather than swapping, marks, the cursor and the
  banner fade, and the screens between islands come up a piece at a time.
- Rebalance Causeway so its islands ramp: Saltmarsh and The Fold ask more,
  The Lighthouse less, and a seeded bot crosses about half its runs. Boons are
  now four cards drawn from nine, including more hearth, extra rewinds and
  upgrades per warden.
- Let Decay read and write fog, the color grade and bloom while a game runs,
  through `this.environment.fog`, `.post_process` and `.bloom`, checked as the
  scene's own values are.
- Never round a color or shader-parameter channel a script writes, so a tint
  the scene authored as `[1, 1, 1, 0]` fades instead of jumping between 0 and 1.
- Add Weave `position: absolute`: the element leaves its parent's line or grid,
  which closes up, and is placed by its own `x` and `y` from the parent's middle.
- Add `ProjectRun::set_board`, so a test can set a shared board value as a
  script would.

- Skip an imported model scaled to nothing instead of stopping the frame. A
  figure a game shrinks away has nothing left to draw, but its singular
  transform was refused, and in the browser that ended the game: Causeway's
  title shrank a Hollow below the limit on a phone's short first frame. A
  transform that is not finite is still an error. Causeway's title also
  switches those walkers off rather than shrinking them to a speck.
- Let active touches drive the unified pointer on hybrid devices, retaining the
  release position for taps before returning to the mouse.

- Add four smoothly eased Causeway battlefield views, J/L and compact touch
  buttons, and horizontal swipes beginning in empty sea. Keep picking and
  keyboard movement aligned with the view.
- Keep Causeway units visible behind raised terrain with muted silhouettes,
  using the reusable optional `sindri.model.occluded_color` rendering path.
- Write Decay's numeric type as `number` in every checked-in script and in
  the docs' Decay snippets. The examples and games still said `f32`, its first
  spelling, so the knowledge pack taught a model to write it. A test in
  `sindri-capabilities` now fails on any `.decay` file that uses `f32`, which
  still compiles.

- Give Causeway a title worth opening on: the Lighthouse at dusk with its
  lamps lit, the camera swaying round it and Hollow trudging over the
  causeway, the logo and a plated menu rising out of the dark, a How to play
  page, and a dip into the first battle as the camera settles into its view.
  Wide screens keep the logo and menu in a shaded column beside the island;
  a phone stacks them above and below it.
- Draw a lit world in the browser player again when post-processing is on:
  the bloom drew its scene in `Rgba8UnormSrgb` while every pipeline was
  built for the canvas's `Bgra8UnormSrgb`, so WebGPU rejected each frame. The
  scene now takes an sRGB output's own format.
- Fade a text's shadow with its letters, so words faded in through a
  `[color=#rrggbbaa]` span no longer leave their shadow on the screen ahead
  of them.
- Rebuild Causeway as a turn-based tactics game. Three wardens hold four
  islands against the Hollow, who show every strike before it lands, until the
  rising tide drowns them. The diorama is generated low-poly GLB models lit by
  the sun, lanterns and a beacon, with Weave panels, an Ogg score and a bot
  that plays the whole crossing in its tests.
- Give Decay the pointer's world ray, `Pointer.ray_origin` and
  `Pointer.ray_direction`, so a script can pick what is under the cursor in a
  3D scene.
- Apply an environment's fog, occlusion, grade and bloom in the shared player
  and the offscreen project renderer, as the editor already did.
- Draw a font bound from a bold-only file in its own face instead of falling
  back to a system font.
- Deliver Sound Lab songs to a project as Ogg Vorbis, about a seventh of the
  size of WAV. The engine already plays WAV, Ogg and MP3.

- Add Sindri Game Dev plugin overview, support, privacy and terms pages to
  the published documentation and site directory.

- Publish an AI knowledge pack at `/knowledge-pack/` on the site: fifteen
  Markdown files and a zip a chat model can be given as its knowledge, such as
  a custom GPT's. It covers the project layout, writing scenes the way the
  editor does, the Decay language and generated API, every scene component with
  its defaults and a real use, Weave, physics, 2D and 3D, what works today, and
  the feature examples whole. Built from the repository's own documents by
  `site/build_knowledge_pack.py`.

- Generate the Weave property reference, `docs/generated/weave-reference.json`
  and `.md`, from the table `sindri-weave` now applies declarations by: every
  property's names, values, what it writes and an example. `font-style:
  italic` now asks for the font's italic face; it was listed as inherited but
  never applied.

- Rebuild Orbital Last Stand around a complete flight deck: four ship sidegrades,
  difficulty choices, salvage unlocks, commendations, a flight log and archive,
  evasive burst, finite Boss Rush, responsive menus and accessibility settings.
  Original space artwork and a new score give it its own presentation.
- Persist Unicode text from Decay with `Save.text` and `Save.set_text`, using the
  existing versioned game-save store and typed fallback behavior.

- Spell Decay's numeric type `number`. It was always a 64-bit float; `f32`,
  its first spelling, named the engine's storage and still works. Diagnostics,
  hovers and the generated reference now say `number`. The decision, and when
  an integer type would be added, is in `docs/decay-direction.md`.

- Make generated songs sound composed rather than random: each section loops
  four chords, the melody is a motif developed in question-and-answer phrases
  with stepwise voice-leading and cadences, the theme returns note for note,
  the bass keeps one riff and the drums one beat per song. Variety moved to
  choices per seed: chord loops from per-mood libraries and six grooves
  (Drift, Bounce, Drive, Ballad, Shuffle, Half-time), each with its own beats,
  rhythms, riffs and swing that moves every part.

- Retain inline 3D surface geometry across frames and share its GPU buffers
  between shadow and color draws. Transforms and materials reuse geometry; mesh
  and atlas edits replace it. Surface indices now support u32 meshes; invalid
  triangle geometry returns a scene-extraction error.

- Author single- and double-sided 3D materials across cubes, inline meshes,
  voxel worlds and imported models. The new `sidedness` choice inherits existing
  settings by default; explicit overrides apply to color and directional shadows,
  with back-face lighting under reflected transforms and no geometry re-upload.
  Lighting Lab includes a visible back-facing panel and its single-sided control.

- Make the Sound Lab's songs sound full: a new stereo engine with band-limited
  oscillators, filtered and enveloped patches, a chords layer that follows the
  bass, a synthesized drum kit with fills and swing, reverb, echo, ducking and
  a limited master at about -14 LUFS. Bass lines step into chord changes and the
  arpeggio changes pattern by section. Exported songs loop without a seam, and
  existing `.sindrisong` files load and play through the new engine; the mixer
  gains a chords channel and a Room & echo control.

- Fix silent games on phones: browser audio now plays through Web Audio,
  unlocked by the first tap, click or key — including a finger lifting off a
  touch stick — and resumed inside the gesture, as iOS Safari requires. The
  platformer's sounds play on a phone.

- Open the Sound Lab in a browser at `/examples/sound-lab/`, on a phone as
  well as a desktop: the same effects designer and music workstation, drawn
  with WebGL2, with Web Audio playback and downloads in place of a project
  folder. Below 720 points wide it folds into one scrolling column with larger
  touch targets; `?mode=sfx` opens on the effects designer. Find it under the
  directory's new **Tools** filter.

- Make game sound effects in the Sindri Sound Lab (`cargo run -p
  sindri-chiptune`): eight seeded kinds — coin, jump, laser, explosion,
  power-up, hurt, blip, thud — shaped with envelope, pitch, vibrato, arpeggio,
  duty, filter and crush controls, auditioned on every change. **Send to
  project** writes the WAV and its editable `.sindrisfx` into `assets/audio/`,
  lists the clip in `sindri.toml` so exports ship it, and gives the
  `Audio.play` line to paste; `sindri-chiptune sfx coin --project <dir>` does
  the same headless. Its music workstation composes A/B chiptune candidates
  with an undoable piano roll and MIDI import/export; songs save and reopen as
  `.sindrisong` and send into a project as a looping clip. The Lab wears the
  editor's forge palette, with a song overview, piano-key gutter, live
  oscilloscope and keyboard shortcuts.
  The platformer's jump, landing, coin and flag sounds were made with it.

- Publish the 3D Transform, Orbit Camera, Character, Model Collision, Lighting
  and Day/Night labs on Pages under `/examples/`, listed in the Directory.

- Add Day/Night Lab: a configurable Decay clock blends sunrise, midday, sunset
  and night, rotates the shadow-casting sun, and moves an imported prop carrying
  night lights. Frozen-hour controls isolate imported shadows and illumination.

- Change scene ambient intensity/color and background from typed Decay
  environment properties. Writes validate atomically and appear in the next
  frame; Lighting Lab now switches cool/warm ambient fill and sky during play.

- Drive 3D lights from Decay through typed color, intensity, range, cone angles
  and enabled state. Invalid writes retain the previous settings; switching a
  light off leaves its entity and script active. The new 3D Lighting Lab animates
  parented lights and the sun through the shared runtime, with lit/dark goals.

- Aim 3D spotlights with feathered inner/outer cone angles. They share eight
  local-light slots with point lights across meshes, voxels and imported models.
  The editor creates downward-facing spots, shows selected cone guides and
  retains valid light settings when an inspector edit is refused.
  Local-light shadows, Decay light properties and game adoption remain open.

- Add bounded 3D point lighting with range falloff across meshes, voxels and
  imported models. Lights follow active parent transforms; the editor creates
  points and shows selected range guides. Point shadows, spotlights and typed
  Decay light properties remain open.

- Add outward cube normals and optional authored inline/cached mesh normals for
  smooth 3D shading. Normals transform correctly under nonuniform scale and
  reflection; world color and shadow draws preserve mirrored front faces.
  Omitted normals retain flat shading; malformed authored normals are rejected.

- Preserve authored GLB emissive factors through import and rendering. Models
  retain self-emission without scene overrides, and removing an override restores
  it. Alpha masks and shadows remain unchanged; emissive maps remain unsupported.

- Add shared standard material factors and GGX shading for opt-in 3D geometry.
  Author checked base color, metallic/roughness and self-emission through
  `sindri.material` in the Rendering inspector. Imported overrides preserve
  textures/masks and geometry residency; removing overrides restores existing
  looks. Shared material assets and game adoption remain pending.

- Imported GLB models now cast directional shadows using cached geometry and
  retained hierarchy. Alpha masks, material alpha, mirrored transforms and
  single/double-sided surfaces govern their silhouettes. Shadow and color
  uniforms stay independent; invalid shadow transforms return typed frame errors.

- Imported models now receive directional shadows from world geometry and share
  its distance/density/height fog shader. Shadow-map resizing preserves model
  geometry residency and refreshes bindings. Authored model fog is currently
  delivered by editor viewports; imported-model shadow casting remains pending.

- Record physical sprite, shape and glyph batch draws/passes, instance counts
  and queue buffer payloads in standalone and editor benchmark reports. Empty
  batches record no work; counters survive slot reuse, and extra Scene work
  stays separate. Clears, post and atlas/resource uploads remain open.

- Measure optional GPU timestamps in native editor benchmarks, with separate
  Game and extra Scene intervals, adapter/period/scope metadata and null for
  unavailable timing. Execution stays outside CPU phases; normal editor
  launches retain their device policy.
- Measure native standalone GPU frame intervals with optional timestamp queries.
  Reports record adapter/period/scope and keep execution separate from CPU wait;
  unsupported devices report unavailable timing. Default and browser devices
  retain baseline feature requirements.
- Record physical 3D draw/pass/upload counters in editor benchmarks, separating
  the Game view during Play from the extra Scene view. Missing views stay absent,
  measured zeroes stay visible, and the summary prints both scopes separately.
- Record physical textured 3D/model draw and render-pass counts, uniform queue
  writes/bytes, and transient/persistent upload events in standalone benchmark
  reports. The comparison tool prints these counters without treating older or
  editor reports as measured zeroes. GPU timestamps and editor counters remain open.
- Add a fixed 300-prop perspective 3D benchmark project with cubes, inline
  surfaces and repeated GLBs. Native/browser players and offscreen captures now
  honor authored directional shadow settings, matching editor viewports.

- Read, copy and restore 3D `transform.rotation` as a typed `Quat` value in
  Decay. Whole writes normalize finite nonzero values atomically; invalid writes
  retain the previous transform. Transform Lab transfers its facing to the goal
  and restores its starting rotation on arrival.

- Keep Physics Playground scripts running when RESET or CLEAN UP removes a
  body named by a collision event. Shells, panes, backstops and
  bumpers check existence before using event references.

- Resize Orbit Camera Lab's barrier with 1/2, inspect it from the far side
  with 3 and resume automatic orbit/removal with R. Synchronized center/edge
  rays verify live collision width. Browser
  controls and native editor scale editing, Undo/Redo, persistence and Play
  provide feature proof; Explorer adoption remains open.

- Show selected orbit camera focus, requested position/yaw circle and current
  pose in the Scene view, including pull-in/recovery during Play. Guides use
  world axes, clip at the near plane and remain outside camera picking.

- Add Camera Orbit through the editor using a real active spatial target.
  Creation requires a Camera; numeric and target edits reject invalid settings
  before writing commands. Native review verifies Undo/Redo, save/reopen and
  authored orbit Play without a camera script. Explorer game adoption remains
  pending.

- Share physics material `.profile` assets between 2D and 3D. The new Physics
  Material 3D component applies profiles and explicit overrides to primitive,
  model, convex and voxel colliders; coefficient edits/reloads preserve motion,
  contacts and streamed groups. Model Collision Lab shares a profile between
  its imported floor and spawned prop. Explorer adoption remains pending.

- Add standalone live 3D collider coefficient reads and atomic updates for
  primitive, convex, triangle and streamed static geometry. Updates retain
  motion, geometry and contacts, wake affected bodies and replay in checkpoints.
  Shared scene profiles and editor/game adoption remain pending.

- Convex-model prefabs now accept typed Decay motion reads, setters and impulses
  immediately after spawning. Queued controls use actual scaled/combined mass,
  preserve previous valid requests on rejected authoring and replay once. Model
  Collision Lab spawns its GLB prop from a typed prefab and verifies this path.

- Automatically discover, export and bind convex GLB collision in the player,
  headless runs and editor Play. Shared triangle/convex uses load once and reload
  atomically; editor defaults and guards support rigid-body/primitive compounds.
  Model Collision Lab adds a falling prop with a Decay impulse and travel goal.

- Add explicit convex-model scene/Session collision binding for movable props,
  with positive nonuniform scale, mixed primitive compounds, shared geometry,
  voxel reach and checkpoint-aware replacement. Automatic project loading,
  editor authoring and game proof remain pending.

- Render-model and static-model-collider fields now offer project GLB pickers in
  the editor. Add Component uses real project models and respects world-space
  and physics ownership constraints. The two asset fields have independent
  widget and undo scopes, while continuous component drags still merge. Native
  directory loading accepts extension case without changing asset IDs.

- Editor Play and native headless project runs now install authored GLB collision
  before gameplay. Play validates its asynchronous model queue, keeps later-scene
  collision assets alive and safely replaces collision on model reload. Model
  Collision Lab has editor/headless goal parity and restore/replay regressions.

- The shared native/browser player now installs static GLB collision before
  gameplay starts, including later scenes and prefab references. Render-only
  models need no collision geometry; missing/invalid collision assets name their
  logical IDs. Model Collision Lab demonstrates wall obstruction and a goal route.

- Discover and export collision-only GLB references from scenes and prefabs,
  including inactive entities and shared render/collision assets. Automatic
  runtime collision binding remains pending.

- Add static model collider scene references and explicit Session GLB collision
  bindings, with positive inherited scale, lifecycle and checkpoint-aware asset
  replacement. Automatic player/editor loading and project proof remain pending.
- Add clone-shared convex collision hulls and mixed primitive/hull rigid bodies
  with bounded construction, validated mass and native dynamic/kinematic/sensor
  regressions. Model scene bindings and browser/editor/game proof remain pending.

- Add shared immutable static triangle collision geometry and keyed physics
  groups, with immediate queries and tested dynamic landing/replay. Invalid
  geometry or group edits fail before mutation. Model scene bindings, dynamic
  hulls and browser/editor/game project proof remain pending.

- Add bounded CPU model collision preparation with accumulated GLB node
  transforms, instance provenance, reflected winding and typed diagnostics.
  The model inspection example accepts `--collision` to report prepared counts.
  Physics triangle/hull shapes and runtime model colliders remain pending.

- Selected 3D characters now show orange slope, step and snap footing guides
  from their validated scaled collider geometry. Native editor review verifies
  settings/undo/save/reopen and keyboard Play to the Character Lab goal; Stop
  restores the authored scene. Explorer game adoption remains pending.

- Add the 3D Character Lab: Decay gravity, keyboard movement/jumping and a
  reversing ferry with solved carry. Native Session tests and real WebGPU
  Chromium keyboard play verify landing and goal completion; CI captures the
  browser goal. Editor controller interaction and Explorer adoption remain pending.

- Decay can queue scene-owned 3D character displacement with
  `Physics3d.move_character` and read copied optional support, raw movement,
  platform carry and collisions through `Physics3d.character_motion`. Invalid
  input preserves the last valid request; scripts observe completed movement
  on the next fixed step. Native host/script and Session replay tests cover the
  API; controller browser, editor and Explorer proof remain pending.

- Scene Character 3D now carries riders through solved platform translation and
  rotation, with classified support, separate carry hits and a `carry_platforms`
  opt-out. Parenting applies motion once, lifecycle changes clear rider caches,
  and actual carry expands terrain residency before movement queries. Session
  checkpoints replay carry. Decay, browser/controller and game proof remain pending.

- Add read-only 3D platform carry composition: verified previous support,
  collision-limited translation and rotation-point travel, separate carry hits
  and classified final grounding. Scene lifecycle, terrain reach and Decay
  integration remain pending.

- Add read-only classified 3D ground probes: nearest support geometry, slope
  limits, skin contacts and explicit initial penetration, using current collider
  poses and deterministic ties. Scene platform carry and Decay access remain pending.

- Add scene-owned 3D character movement: queued XYZ displacement, scaled solid
  probes, terrain residency, cached movement results and shared Session replay.
  Gameplay owns gravity and jumps; Decay access and platform carry remain pending.

- Add a 3D character movement foundation: read-only Rapier queries with
  engine-owned slope, step and snap settings, current collider poses and copied
  movement/hit results. Scene ownership, Decay, editor and moving-platform
  integration remain pending.

- 3D collider dimensions and offsets follow entity and parent scale. Uniform
  positive scale works for every shape; boxes also support nonuniform scale
  when their rotated axes stay orthogonal. Unsupported shear, stretched round
  shapes and invalid scale fail explicitly. Scale edits rebuild collision at the
  next fixed step; collider wireframes use the same geometry. Existing scenes
  using world-unit collider dimensions must convert them to local units.

- Native editor Play supports Decay cursor capture and raw mouse look, with
  checked capture feedback and release on Escape, focus loss, pause/stop or
  hiding the Game view.

- Capture the cursor from Decay with `Pointer.lock()` / `Input.Pointer.lock()`,
  release with `unlock()`, and read actual state through read-only `locked`.
  Native/browser players feed unbounded relative motion through `Pointer.delta`;
  Escape, focus loss and suspension release capture. Browser denial leaves the
  game running. Orbit Camera Lab demonstrates captured look.

- Add shared input groundwork for pointer lock: actual capture feedback and
  relative mouse motion, with mode-change rebasing, focus-loss cleanup and
  once-per-step accumulation. Native/browser capture and Decay controls remain
  pending.

- Read mouse displacement from Decay with read-only `Pointer.delta` or
  `Input.Pointer.delta`, as a `Vec2` in viewport pixels per fixed step. Orbit
  Camera Lab now supports right-button drag-to-look with adjustable sensitivity
  and pitch limits. Pointer lock remains pending.

- Follow a 3D target with an engine-owned orbit camera: smooth movement, aiming,
  immediate obstruction pull-in and recovery when the view clears. Decay selects
  explicit camera entities, changes orbit settings and perspective FOV, and can
  stop orbit without resetting the pose. The Orbit Camera Lab runs to its goal.

- Aim and orbit entities from Decay with `transform.look_at` and
  `transform.rotate_around`, including under rotated/scaled parents. Read
  world-space `forward`, `right`, and `up` to move the way an entity faces.
  Direction writes are rejected by the compiler and runtime; completion,
  hover, and the generated API identify them as read-only.

- Turn solid objects from Decay. `this.transform.yaw`, `pitch` and `roll` read and write a 3D rotation in radians: yaw turns on the ground, pitch tips up or down, roll turns about the facing, and writing one keeps the others.
- Orbital has the original's sector events. The first arrives at 0:42 and another 38 to 60 seconds after each ends: System Overload, Signal Harvest, Elite Hunt, Swarm Tide, Glass Space or Temporal Echo, each bending fire rate, enemy speed, salvage pull, elites, spawns, experience, hull damage or score for 13 to 18 seconds under a banner beneath the boss strip. A boss rush has none.
- Orbital plays at the original's pace. Salvage is worth what the original pays (a scout's gem is 8, a boss leaves 384), so the module chooser comes about twice as often early on. Bosses arrive on the original's one-a-minute clock and take as many shots as they did there (a base-build Warden is about 58 rather than 206). Contact hurts for as long as an enemy touches the ship, through a 0.22 s mercy window instead of one hit per touch and 0.9 s of immunity, at the original's per-kind damage; regular enemies' shots do the original's 9 to 15 of 100 hull. Enemies are drawn and hit at the original's size (0.24 rather than 0.5), move at the original's speed across the screen and keep its distances. Salvage is pulled from the original's 110 px, faster as it closes, and lasts two minutes. The ship has the original's 5% base critical chance and its faster, longer-lived rounds, and the camera no longer shakes on every hit and kill.
- Orbital's modules do what their names say. The original hands its 77 plain modules one of fifteen stat bundles by list position, so Fork Drone raised hull, Ballast sped the ship up and Combat Medic sped up rounds. Each now carries the bundle its name suggests (Fork Drone is a prism that circles the ship and splits any round fired through it into three, the Binding of Isaac's Angelic Prism rather than a spread upgrade; Combat Medic and Field Rations repair hull, Ballast adds hull and armour, Gravity Mote and Mag Clamp widen the salvage pull), at the original's strengths, and hull, repair and salvage modules say which of those they do.
- Orbital is closer to the game it recreates. Its bosses are the size the original draws them (the Harrower was about two thirds of it, the Last Light under half) and are hit where they are drawn. The background grid drifts left and up as the original's does instead of turning, which read as tilt. On desktop the hull and XP panel is a quarter of the screen's width rather than two fifths. Hull reads in whole points ("HULL 1/5", not "0.65/5"), and an unnamed pilot's score is labelled SCORE.
- Low Tide is lit. The sun keeps time with the tide: a bright day at low water, a golden hour as it runs out, a moonlit night when the sea is in and a dawn with the ebb. The salt, the crawler, the crew and the wrecks have normal maps, so the low sun rakes across cracks, plates and boards. At dusk the cabin lamps, the crew's lantern and two headlights come on, and a wreck caught in the headlights casts a shadow. The treads kick up dust and the engine smokes while it drives, swimmers throw spray, divers trail bubbles and a lamp lights the wreck below, and submerged wrecks ripple under the brine through a sprite shader. A voxel world viewed as a map is now lit by 2D lights, where before it was always drawn at full brightness, and its new `normal_maps` names a normal map for each texture its squares draw from.
- Make normal maps in the editor. The **Normal map** tab, opened from an image or from the sprite sheet slicer, derives a tangent-space map from a sprite's brightness or from a painted height map, keeping its size, alpha and pixel edges and each sheet cell apart, with strength, smoothing, detail, invert, edge and green-axis settings and Metal, Wood, Stone, Fabric and Terrain presets. It previews the map as a picture or one channel, lit by a `sindri.light2d` you drag around (drawn by the same renderer and extraction as the Scene view), split against the sprite without a map, and on a plane under an orbiting camera. Save writes `<name>_n.png` and the settings that made it as `<name>_n.normalmap`, so the map is made again when the art changes, and "Use on sprites" points the scene's sprites and tilemaps at it in one undo step. The platformer's hero is now lit through a map made there. `docs/normal-map-studio.md` is the contract.
- Orbital is drawn as neon vector shapes again, as the original is. Enemies are filled polygons in their own warm colours with as many sides as the original gives them, asteroids are crisp outlined shards, the ship is the original's dart with its chevron, and every boss is its own coloured polygon with a turning rim that thickens once it is below half health. Each sector tints the backdrop and grid as the original does. The charger now glows white-hot through its shape's fill as it winds up, so the mine blast's flash is the sprite colour offset's user in the repository. The boss banner is a slim strip rather than a panel over where bosses enter, and `orbital-baked-capture boss-N` reaches all twelve bosses again; from the seventh on it had been photographing the Warden.
- Place text in the world and give it spans. `sindri.text` carries the UI label's fields in world units, sits at its entity and draws over the sprites of its layer, staying level unless `follow_rotation` turns it with the entity; `Ui.set_text` and `Ui.set_number` write it, and Decay reaches `this.world_text.{color,font_size,visible,layer}`. `rich: true` on either text component reads `[b]`, `[i]` and `[color=#rrggbb]` spans, which fade with the text's alpha. Orbital's landed shots now float their damage, critical hits in bold gold. `docs/world-text.md` is the contract.
- Mask sprites. `sindri.sprite_mask` places a picture whose alpha is a shape, and a `sindri.sprite` with `mask: "inside"` or `"outside"` and that mask's `mask_name` draws only within or beyond it. The mask rides in the batch and is read in the fragment stage, so masked sprites still batch and no stencil buffer is needed. `examples/sprite-masks` shows a wandering keyhole revealing a meadow and a hidden gem; `docs/sprite-masks.md` is the contract.
- Paint terrains. A sheet declares terrains by name: the sixteen `edges` or forty-seven `blob` sprites a cell draws for each pattern of same-terrain neighbors. The tilemap inspector offers them above the palette, and the brush paints one and redraws every neighbor that depends on the cell; erasing beside a terrain redraws too. `Grid.set_terrain(map, column, row, terrain)` does the same from Decay. The platformer's level is painted with a ground terrain, so its gaps show rims and its tops grow grass; `docs/terrains.md` is the contract.
- Slice sprites into nine. A sheet records a border per sprite in pixels (`borders`, with the image `size`), and `sindri.sprite` and `sindri.ui.image` with `draw: "sliced"` keep it at its size while the middle stretches, scaled by `border_scale`. A sliced UI bar keeps its end caps as it empties. The sheet slicer edits the border with numbers and by dragging dashed guides on the image. The platformer's HUD frame and its signboard are one sliced panel; `docs/nine-slice.md` is the contract.
- Emit particles. `sindri.particles` keeps throwing particles while it plays: a rate and a cap, lifetime, speed, direction and spread, a point, circle, box or ring shape at an offset, gravity, drag, size over life, a color ramp, rotation and spin, in world or local space. They live in the fleck pool and its random stream, so they are cheap and a seeded run stays the same run. `Effects.play`, `Effects.stop` and `Effects.emit(entity, count)` drive one from Decay, and the Scene view plays the selected emitter without entering Play. The platformer's coins sparkle and the hero trails dust; `docs/particles.md` is the contract. `project-capture` gains `hold:<Key>` and `release:<Key>`.
- Light a 2D scene. Turn on `lighting_2d` in `sindri.environment` and its ambient color multiplies world sprites, tilemaps, flecks and shapes; `sindri.light2d` point lights add their color by distance, with a radius, falloff and intensity, and a sprite opts out with `unlit`. Name a `normal_map` on a sprite or tilemap and give a light a `height`, and a low light rakes across the art. Put `sindri.light2d.occluder` on a box, a polygon or a tilemap and a light with `shadows` on is blocked by it. Decay writes `this.light2d.color`, `intensity`, `radius`, `falloff` and `height`. Create offers Light 2D and the Scene view draws radii and occluders, with handles. The platformer has a cave lit by lanterns and a torch, and `docs/2d-lights.md` is the contract.
- Draw a sprite through your own shader. A project `.wgsl` file defining `fn sprite_shader(input: SpriteShaderInput) -> vec4<f32>` is named in `sindri.sprite`'s new `shader` field, and four `shader_params` per sprite reach it as `input.params`, beside the unshaded color, UVs, world position and the scene's clock. Sprites sharing a shader still share a draw. The editor hot-reloads a shader on save; a shader that does not compile names its file and line and the sprite draws with the built-in look; export refuses one. Decay writes `this.sprite.shader_params`. `examples/sprite-shaders` shows an outline, a dissolve, a hit flash and a wave, and `docs/sprite-shaders.md` is the contract.

- Find every way in under `Input`. `Input.Action`, `Input.Pointer`, `Input.Stick`, `Input.Gesture`, `Input.Touch`, `Input.Keyboard` and `Input.Gamepad` are the namespaces that grew beside `Input` one at a time, now reached from where someone looking for touch controls starts, checked as the same types and answered by the same state. `Input`'s own calls are still the keyboard, so no script changes; the flat names stay until the scripts here move to the gathered ones.

- Load imported models everywhere a project plays. `sindri-player` loads `.glb` models in the browser and natively. The capture and benchmark tools bind them and light them as authored. The editor loads, binds and hot-reloads them for the Scene and Game views, and draws the rest of a scene while one loads or fails to decode. A project that keeps its assets beside `sindri.toml`, as Low Tide does, plays as it exports.

- Every game steps faster. The engine no longer re-decodes components, lays out an unchanged screen, re-checks script text or steps an empty 3D world every fixed step: components are decoded once per change (`sindri_core::Decoded`), a script setting a value to what it already is no longer counts as an edit, and Decay finds a local without hashing its name. The platformer's step takes about half the work it did, and Orbital's standalone step went from 1.13 to 0.84 ms.

- The editor's remaining gaps from its audits are closed. Right-click menus on the hierarchy's and project browser's empty space, a component's heading, a field's name, the Scene view and console lines; copy and paste of entities (Ctrl+C, Ctrl+V, Paste as child, Paste here) and of a component's values; Move to top level. The Scene view draws 2D joints (a click selects one), 3D colliders, a selected character's slope, step and snap, and a selected effect burst's reach. Current problems are grouped by cause, naming the entities under each, and counted by cause. Angles are edited in degrees; Camera Behaviour's follow and confine are added and removed in the inspector, with an entity picker for the target; a camera's fit is a choice; and a component's fields are listed in the order its type declares them. Components can declare an optional field (`describe_optional`), whose fields can then be described.

- Scrub a run. Play now records the last two minutes of a run, and the Timeline's Run strip takes it back to any step: the run pauses there exactly as it was, and playing on carries on from that step. A run's whole state can now be copied and restored — scripts, both physics solvers, the screen UI, effects — and a restored run given the same input plays exactly as it did.

- Edit a scene while it plays. Inspector fields, the hierarchy's create, rename, duplicate and delete, gizmos, and tile and block painting now act on the running game at once, so a jump can be tuned and tried in the same run. Stop restores the scene as it was and lists what you changed; the changes you keep are applied as ordinary undoable edits, carrying only the fields and transform parts you touched, never values the game wrote while it ran. An edit to something the run spawned is explained rather than dropped. Saving, new scenes, prefab files and undo/redo still wait for Stop.

- Fix the Physics Playground's domino run sometimes stopping short of its red button. The last domino's tip only just reached the button's edge, so after the room had been played with it could come to rest leaning against the cap without pressing it, about one time in four at the pace a busy browser plays. The button now sits where the domino lands on it squarely. Setting a contraption up again while everything is dropped now puts the room back first: the room's parts lay wherever gravity had thrown them, and on the domino ledge they knocked the run over from its far end.

- Every exported game is played by `sindri-player`, the engine's own host, rather than by a bundle built from Causeway's crate: a published game now loads `sindri_player.js`, and `cargo run -p sindri-player -- <project>` plays any project directory natively. Browser saves are kept per project, under `sindri.<entry scene>.save`, where every game on a site used to share `sindri.causeway.save` and overwrite each other's progress; a save made under the old key is not carried over. A native host on a machine with no sound device now plays silently instead of refusing to start.

- Run the editor with `cargo editor <project>`: an optimised, incrementally built profile in which a game plays about as fast as in a release build. The plain dev profile now optimises every dependency, which makes Play in a `cargo run` editor about 1.6× faster at no cost to rebuilding after an edit.

- Every host binds the engine's own block set and textures. `builtin:blocks` was only ever bound by the editor, so a project whose voxel world named it failed in the exported browser build and in `project-capture`.

- The Profiler times the editor's whole frame, not only Play's steps: upkeep, presentation, extraction, encoding, the panels, egui's painting and the wait for the next frame each have a phase, and a toolbar toggle records frames while editing. `sindri-editor <project> --benchmark <report.json>` measures a project at rest and in Play and exits; `project-benchmark` measures the same project as the browser host plays it, and `scripts/frame-benchmark.py` compares the two.
- Load external model references through native/browser project asset queues,
  export original validated GLB bytes as model assets, and apply authored scene
  lighting in both hosts. The real Low Tide cutaway crawler is visually verified
  in native offscreen and WebGPU runtimes. Imported-model shadows and full editor
  model UX remain deferred.

- Assign the registered imported-model component its existing rendering family
  and mesh icon so the editor component catalogue remains complete. Model
  authoring and loading UI remain deferred.

- Add `sindri.model` external asset references, shared scene bindings, inactive
  reference discovery and world-transform extraction. The public facade prepares
  decoded assets without coupling scene/render to assets. Project hosts and
  export packaging remain pending.

- Add an independent imported-model GPU path with retained hierarchy, normals,
  UVs, 16/32-bit indices, base-color textures, metallic/roughness lighting,
  depth testing, mirrored transforms and shared GPU residency. Scene and project
  host integration remains in progress.

- Add CPU-side static GLB decoding with retained hierarchy, reusable meshes,
  normals/UVs, 16/32-bit indices, material factors, embedded base-color images,
  and asset-aware malformed/unsupported diagnostics. Runtime model rendering
  and scene/export integration remain pending.

- The `cube` mesh primitive is now a unit cube, one unit across like a default Collider 3D box, so a cube mesh and its collider match at any scale. It used to be two units across, which drew 3D bodies twice the size of their colliders. Editor picking follows the new size; the cube example and the editor fixture are drawn at scale 2 to look as they did.

- Fix switching scenes when both have an environment: a switched-off scene's `sindri.environment` counted as a second one and stopped the frame. `project-capture` now loads every scene a project lists, so captures can follow `Scene.go`, and can drag the pointer with `move`, `down` and `up` steps.

- Fix reaching an entity held in a struct field: `hit.entity.transform.position` passed the checker but failed at runtime with NotAReference. The field is now read as a value and the path walks on from the entity it holds, for reads and writes alike.

- Add `Physics.gravity()` and `Physics.set_gravity(v)` to Decay. They read and change the first active `sindri.physics2d.world`, so the change shows in the inspector and survives a solver rebuild. Changing gravity now wakes resting bodies in 2D and 3D, so a sleeping crate falls up when gravity turns over, and collision layer names follow the active world settings as gravity already did.

- Authored voxel worlds collide with 3D bodies. Add `sindri.physics3d.voxel_collider` and a block-set `collides` flag (default on; off for water); bodies land on generated and edited terrain wherever they are, with geometry kept resident only near them. In Causeway, a block taken back now tumbles out of its cell and lands before it returns to the stock.

- Add a one-way plank ferry across the platformer’s first gap, driven by Decay kinematic velocity. Scene controller support carries riders in both directions without extra gameplay displacement; jumping leaves carry, landing resumes it, and dropping through reaches the pit and normal respawn.

- Add low stone steps and an inclined boardwalk to the platformer. The hero walks over the risers and follows the slope downhill without jumping; intended horizontal speed now survives a blocked controller sweep so subsequent requests can step forward. Native control tests prove disabling steps or lowering the slope limit blocks the same terrain.

- Move the platformer hero to the scene-owned Character 2D controller. Decay retains movement, gravity and jump policy; separate sensors preserve coin/flag pickups and timed drop-through still reaches ordinary floors. The dynamic crate retains CCD and contact-impulse proof. Editor interaction and game slope/step/platform-carry acceptance remain open.

- Add typed Decay character displacement requests and copied optional motion/support/carry snapshots. Shared runtime/editor Play offer scene controller context; character drop-through uses the scene timer while dynamic-body hosts retain their solver API. Invalid input preserves queued requests. Platformer controller adoption and editor/game proof remain open.

- Add scene-owned 2D controllers with validated settings, queued displacement/snap input, timed drop-through and synchronized platform snapshots. Movement applies once after the solve; collider response and discrete sensors observe it at the next solve. Editor/Decay/platformer proof remains open.
- Grounded 2D movement now respects one-way support sides and cones across sliding, snapping, steps and platform carry. Request-scoped drop-through ignores only one-way solids; the host owns duration. Ordinary geometric queries stay two-sided; editor/Decay/platformer integration remains open.
- Add opt-in synchronized platform carry to grounded 2D movement. Previous support is verified, translation and rotation-point motion are swept before character movement, and carry collisions/current pose are returned separately. Hosts retain snapshot ownership; one-way controller and editor/Decay/game integration remain in progress.
- Add opt-in grounded 2D steps with lift/forward/landing clearance, walkable support and exact fallback on rejection. Refine movement cast normals to prevent artificial hops on flat box faces. Moving platforms, one-way controller policy and editor/Decay/game integration remain in progress.
- Enforce slope limits during grounded 2D sliding: steep surfaces cannot create unrequested upward motion, explicit jumps retain bounded rise, and steep descent remains ungrounded. Geometric sliding stays unrestricted; full character integration remains in progress.
- Add read-only grounded 2D sweep/slide results with optional downward snapping, post-move support and ascent suppression. Snapping defaults off; slope movement, steps, platforms and editor/Decay/game integration remain in progress.
- Add read-only 2D ground probes with configurable up, slope limits, travel and skin. They report steep obstructions, initial penetration and zero-travel support without snapping or changing bodies. Character movement integration remains in progress.

- Add read-only 2D `move_and_slide` queries with skin, bounded iterations, ordered collisions and explicit initial-penetration/budget outcomes. This is the geometric foundation for character movement; grounding, steps, moving platforms and editor/Decay/game integration remain in progress.

- Offer velocity and position motor modes as schema-driven inspector choices for 2D hinges and sliders, including older payloads that omit the mode. The choices use the engine enum’s scene spellings.

- Make editor scene Save/Save As and subtree prefab authoring remap registered runtime-local entity references to assigned stable IDs automatically. Invalid references fail before writing or adopting a path; the live world and unknow…131 tokens truncated…e is worked out when the project compiles from literals, operators, variants and other constants — including another file's — so every use costs what the literal would. Assigning one, declaring one twice, a value that calls something or reads the world, and constants that need each other are all compile errors. decay-lsp completes constants, shows their value on hover and lists them in the outline, and VS Code highlights `const`. Both Orbital games now declare the camera's `VIEW_SIZE` once, where 21 scripts in Orbital Baked and 10 in Last Stand each exported their own copy.
- The editor's title bar no longer shows stray text under the Stop button. With floating panels, the Scene and Game tabs are drawn into the title bar, and the Game panel's note ("the panel's own shape") was drawn at the far end of that row, under the transport controls. Tabs drawn in the title bar no longer carry their panel's controls; the floating Game view has its own.
- Scripts can set a shape's `dash_duty`, how much of each dash is drawn rather than gap. Orbital's `enemy-mark.decay` already did, and so failed to compile in both games — which the editor reported and no test did, because the games' compile test only covered scripts the opening scene names. Each Orbital game now also compiles every script in the project.
- The editor's welcome window lists each project once. A project opened as `games/orbital/` and as `games/orbital` was remembered as two, because the list compared paths as written; paths are now made absolute and normalized before they are remembered or compared, and a preferences file that already holds both spellings is merged when it is read.
- The editor no longer re-scans the whole scene every frame to decide which scripts to load. That cost 2.5 ms a frame at rest and 6 ms while Orbital Baked played; it now happens after an edit or when a prefab arrives, which is all that can change the answer since the whole project is loaded on open. With the faster scripts, a playing frame of Orbital Baked in the editor went from 45 ms of CPU to 17 ms, nearly all of it drawing.
- Decay scripts run about five times faster. A step of Orbital Baked mid-run took 4.9 ms of scripts and host, and now takes 1.0 ms; a drifter enemy's update went from 0.24 ms to 0.014 ms. Two costs were paid on every call and every read: the runtime cloned the script's whole compiled container (every function and instruction) to call into it, and reading any value from the engine — `this.transform.position.x` included — serialized all of the entity's components into a fresh JSON object to look one number up. Both now borrow instead. This was most of Orbital's lag in the editor, where a slow frame also ran several catch-up steps.
- Switching projects in the editor no longer draws the new one with the old one's textures. The platformer's ground showed Orbital's ship after one was closed and the other opened, because the sprite and mesh renderers cached a GPU binding per texture handle, and each scene's handles start from one again. Each `TextureRegistry` now has a `RegistryIdentity`, and a renderer forgets its cached bindings when it draws from a different registry.
- The editor draws what a game spawns. Orbital Baked's asteroids and enemies were the magenta missing-texture checker in the editor while the exported game drew them, because the editor only asked for textures the scene's own entities name, and those are named only by the prefabs a script spawns. Every loaded prefab's textures, sprite sheets and fonts are now asked for alongside the scene's, as soon as the prefab loads.
- The editor loads a whole project before running any of it. Opening Orbital Baked used to fill the console with script errors — "asset load queue is full", `Game` has no member `run_state`, unknown name `visible_half_x`, a prefab "this project has not loaded" — which disappeared on the second Play. The script loader refused anything past sixteen files, and compiled and started what had arrived against half a project. There is no file limit now (`AssetLoadQueueConfig::unbounded`, used for the editor's scripts and textures); every script, prefab and profile in the project is asked for when it opens, including a prefab only another prefab names; and nothing compiles or runs until all of it has arrived.
- Orbital Last Stand and Orbital Baked declare their last eight shared values. `hp` and `max_hp` start at 5, `next_level` at 35 and `sector` at 1 — the values each run already set at its start — and `boss_hp`, `boss_max`, `target_x` and `target_y` at 0, which means "no boss" and "no target". The two readers whose fallback carried meaning now say so: boss ornaments show full health until a boss has written its own, and the Aegis reservoir beam fires straight ahead when there is no live target instead of at wherever the last one was. Each game is down to 25 board calls, all with keys built at runtime.
- Every name a Decay script can reach in Sindri now says what it is for. `docs/generated/decay-api.json` gives each global, `this` member, host type and member a plain-language `description`, written for someone new to Decay, and each call its `parameter_names`; `decay-api.md` shows them, as `Input.axis(negative: String, positive: String)` — -1, 0 or 1 from a pair of keys. The text lives in `sindri_decay::reference`, and the capability check fails when a host name is added without a description or a description outlives its name.
- Every Decay compiler error has a stable code. `decay-lsp` reports `error[decay-semantic/unknown-name]` rather than just the phase, in the editor, `--check` and `--check --json`; `sindri_decay::SourceDiagnostic` carries the code too; and `docs/generated/decay-language.json` lists all 49 with a summary each, so documentation, tests and fixes can key on what went wrong instead of matching message text. A message may be reworded; its code never changes.
- `docs/generated/decay-language.json` describes the Decay language for tools: its keywords and contextual words, item forms, attributes, operators with precedence and grouping, built-in types and their spellings, constructors, what a vector, timer, text and list can be asked with each operation's signature, the lifecycle functions Sindri calls, and the runtime's limits. It is generated with the other capability documents and CI fails when it drifts. Every list is the table the compiler itself reads: keywords and operators now live in `decay_syntax::vocabulary`, which the lexer and parser use, built-in types in `decay_semantic::BUILT_IN_TYPES`, and member signatures in `decay_semantic::members`, which the analyzer and decay-lsp's completion now share.
- decay-lsp completes a value's own members. `card.` offers a struct's fields, `v.` a vector's `x`/`y`/`length`/`dot`, `name.` text's `length`/`contains`/`slice`, `items.` a list's `push`/`pop`/`length`, `this.cooldown.` a timer's `done`/`left` — for a local, a parameter, a loop's binding or a field, typed by the analysis rather than read off its declaration, so `let card = make();` completes too — and follows chains such as `card.at.` through the language's members and the engine's. Hovering a local shows its type.
- A scene can author a list or struct `@export`. `"loot": [{ "name": "gem", "weight": 1.5, "kind": "Gem" }]` is read by the type the script declared — a struct as an object of its fields, a list as its items, an enum inside either by the variant's name — and a wrong item or field is refused with which one and why; a struct field the scene leaves out keeps the script's default. The inspector draws a list with a row per item and add, remove and move-up buttons, and a struct with a row per field, all the way down. Orbital's module chooser now reads its four card names from a list the scene authors.
- Orbital Last Stand and Orbital Baked declare what they share. `game-state.decay` in each names every board value whose readers agree on its starting value — 108 in Last Stand, 121 in Baked — and `Game.get("kills", 0.0)` / `Game.set("kills", n)` became `Game.kills` / `Game.kills = n` everywhere, so a misspelt name is a compile error. Last Stand went from 463 board calls to 88 and Baked from 739 to 153. What stays is keys built at runtime (`stat + "_add"`, a module's `owned_key`) and eight values whose callers read them with different fallbacks — `hp` alone with five — which need someone to decide the right start before they are declared.
- Decay has structs. `struct OfferCard { card: Entity, name: Entity, blurb: Entity }` in any file is a type every script can hold, pass, return, compare and keep in lists; `OfferCard(card: c, name: n, blurb: b)` builds one with every field named, `shown.name` reads a field, and `best.weight += 1.0`, `this.hand.best.name = "Nova"` or `hand.cards.push(c)` change one through the `var` that holds it. A struct is a value, copied where it is assigned, and `print` shows its fields. decay-lsp completes, hovers and outlines structs; VS Code highlights `struct`. Orbital's module chooser keeps each offer slot's card, name and blurb in one `OfferCard` instead of three lists kept in step.
- Decay has lists and ranges. `[a, b]` writes a list; `push`, `pop`, `insert`, `remove_at`, `clear` and `xs[i] = v` change the one a `var` or field holds, in place; `contains`, `index_of` and `length` ask it; and `for i in 0..n` walks a range without building a list. `List<T>` is the type host queries such as `World.with_tag` already return (`Array<T>` still works), and its size is now spelled `.length`, as for text and vectors (`.len` still works). A list is a value — assigning one copies it — and a script's list is capped at 10,000 elements. Orbital's module chooser keeps its cards in lists built from their names instead of twelve numbered fields, walks its catalogue with ranges, and passes the offers taken so far as one list.
- Decay joins text. `"Score " + score` works, with numbers (a whole one without its `.0`), flags, vectors and enum variants on either side, and `+=` appends; text has `length`, `uppercase`, `lowercase`, `trimmed`, `contains`, `starts_with`, `ends_with`, `find`, `slice` and `replace`, counting characters rather than bytes. Text is capped at 64 KiB so a loop that keeps doubling a string fails its script, not the editor. `print` writes numbers the same way, so `0.1` prints as `0.1`. Scorchball's score digit and ball ring pick their clips by joining in place of sixteen `if`s, and both Orbital games clear each stat's `_add`/`_mul` keys from its one name.
- Decay has enums and `match`. `enum Phase { Lobby, Countdown, Play }` in any file names a type every script can hold, compare and pass, written `Phase.Lobby`, never a number; `match this.phase { Phase.Lobby => { } Phase.Countdown | Phase.Play => { } }` must say what happens for every variant or end with `_`, so adding a variant points at every `match` that has not caught up. An enum can be a `state` field and an `@export`, which a scene authors by the variant's name (`"kind": "Grow"`) and the inspector offers as a dropdown. Scorchball's match phase and power-up kinds are enums: `0`/`1`/`2` and `1`–`4` with a comment to decode them are gone. decay-lsp completes `Phase.` with its variants and lists enums as symbols; VS Code highlights `enum`, `match` and `=>`.
- Decay scripts can share functions. A `fn` written outside any script is its file's; declared `shared fn`, it is callable by name from every script in the project, checked across files. Neither has a `this`, so what it needs, it is passed. Callers link their own copy when they compile and recompile when it changes, and a helper file that does not compile fails only the scripts that call into it. Orbital Last Stand and Orbital Baked now keep `visible_half_x`, `visible_half_y` and `on_screen` in one `view.decay` each, replacing twenty-five identical copies of the first two and thirteen of the third.
- Decay has timers. `var cooldown = Timer(0.0);` holds a countdown, `Timer(2.4)` starts one, and `.done`, `.left`, `.duration` and `.progress` read it; every timer a script's fields hold runs down by itself before that script's `update`, so the hand-kept `-= dt` goes, and an early `return` can no longer stop the clock. A field initialized with `Timer(...)`, `Vec2(...)` or `Vec3(...)` now has that type without an annotation, which also fixes unannotated vector fields whose `.x` failed at runtime. Scorchball's banners, power-ups, burning, aftertouch and bot reaction times use timers. VS Code highlights `Timer`.
- Decay now has an official VS Code v1 experience: current-language highlighting, comments/brackets/indentation, and the real project-aware `decay-lsp` for diagnostics, completion, hover, symbols, events, shared state, and cross-file script types. Double-click a `.decay` file in Sindri, or choose **Open in External Editor**, to open its whole project and that script in VS Code; saving continues through Sindri's existing watched compile/reload path. VS Code remains optional and its executable can be configured with `SINDRI_VSCODE`.
- Decay scripts can declare the values they share. `state Game { var score: f32 = 0.0; var won: bool = false; }` in any file makes `Game.score` and `Game.won` names every script reads and writes, each with one type and one starting value; a misspelt name, a wrong type or a write to a `let` is a compile error across files, where `Game.get("scroe", 0.0)` silently read the fallback. Values live on the same board under the same names, so older `Game.get`/`Game.set` calls and tests keep working while a project moves over. Scorchball and the platformer now declare theirs.
- Decay has events. `event GoalScored(team: f32);` in any file of a project declares one, `GoalScored.emit(1.0)` sends it, and `on GoalScored(team) { ... }` in any script handles it; handlers run after the frame's pass, in order, and a misspelt event, a wrong value or a mismatched handler is a compile error across files. Paths also work from a reference held in a field or computed on the spot: `this.ball.kick(v)` and `Bolt.on(hit).damage` used to fail at runtime. Scorchball no longer uses signals: its goals, wind and fireball are events, and its kicks, aftertouch and power-ups are typed messages.
- Decay scripts can name each other by type. `Bolt.on(entity)` finds a script on an entity; its fields read and write live (or set what a just-spawned one starts with, replacing `World.set_property`), and calling its functions sends typed messages delivered after the frame's pass. Misspelt fields, messages and arguments are compile errors, across files. The editor, the exporter and `decay-lsp` now see every script in the project.
- Decay has vectors. `Vec2(x, y)` and `Vec3(x, y, z)` add, subtract, scale and compare; `.x`/`.y`/`.z` read and assign; and `length`, `normalized`, `dot`, `distance` and `lerp` do the maths scripts used to write by hand. `this.transform.position`, `world_position` and `scale` can be read and written whole (the `.x` paths still work), and `Pointer.position`, `Pointer.overlay` and `Stick.direction` are new `Vec2` values. Also new: `floor`, `ceil`, `round`, `sign`, `clamp`, `lerp`, `PI` and `TAU`. Scorchball's ball and player and Orbital Last Stand's aim and bullet homing now use vectors.
- The local assistant's setup now ends on "Test that it works" — a question with a known answer — instead of a script-fixing test. What the assistant can do in Sindri is tested separately and shown in the ready card: the Decay repair test runs by itself right after setup, a feature that did not pass is shown as off with "Test again" rather than failing the whole assistant, and each result is remembered, passed or not. The offer card now describes a general assistant rather than only script fixes.
- The local assistant now sets itself up entirely inside the editor. The Assistant panel says what it does, that it runs only on your computer, and what it costs (a 4.7 GB download) before anything starts; one button then downloads a pinned llama.cpp runner and the Qwen2.5 Coder 7B model into Sindri's own folder, checks both, starts the model and checks it can fix scripts, showing each step with a progress bar, speed and time left. No download page, terminal, installer or password; it can be stopped at any point and resumes where it left off, failures say what to do in plain words, the check is remembered across restarts, and the model can be stopped or removed from the panel. Available on Linux (x86-64) and Macs with Apple silicon; other computers are told it is not available yet. The Ollama-based setup is gone.
- The editor can propose a fix for a Decay script that does not compile, using a local model. Selecting a `.decay` file now shows its compile errors with line and column, the same ones Play would report. Once the Assistant panel's check has shown the model can repair Decay (it is given two broken scripts and must fix both), **Propose a fix** asks it for one: the answer is compiled, sent back with its errors at most twice if it still fails, and only a version that compiles and keeps every declared script is shown, as a diff. Nothing is written until you press Accept, and the previous version can be put back. The Assistant panel's check now actually runs, and a feature it cannot test yet reads "not offered yet" rather than "unavailable".
- Scorchball has bots: Select in the lobby adds one to the side with fewer players. A bot readies itself and plays like a person rather than a homing missile: it looks a few times a second with a little aim noise, attacks round whoever is in front and shoots at the far side of the goal, supports a teammate on the ball, defends goal-side instead of chasing, and goes for loose balls it can reach first.
- A child's transform is now local to its parent for everything in the world, not only shapes: sprites, meshes, tiles, voxels, cameras, lights, physics bodies and particle bursts move, turn and grow with their parent. Scripts read where a child really is with `transform.world_position` (writable too); the editor's handles work in the world and store the local result, and dragging an entity onto a new parent keeps it where it was. Orbital's boss visuals and Aegis core fire, and Scorchball's markers, signposts and ball ring, now ride their parents instead of being placed every frame.
- Scorchball, a couch football game for two to four pads, ported from an earlier Unity project with its original art and sounds: join by pressing, ready up, kick off, and fight over the ball with Enlarger, Push Boost, Wind and Fireball power-ups. It is exported to the site at `examples/scorchball/`.
- Gamepads, on desktop and in the browser. Players join by pressing a face button or Start on their own pad and leave when it is unplugged; scripts read each player's pad through `Gamepad` by player slot (`Gamepad.joined()`, `Gamepad.just_pressed(1.0, "south")`, `Gamepad.axis(2.0, "left_x")`), with slot 0 meaning any pad. Action bindings accept `gamepad.<button>` and `gamepad.axis.<axis>`. The editor's Play reads pads too, and starts the players over each time. Building on Linux now needs `libudev-dev`, as audio needs `libasound2-dev`; turn off `sindri-platform`'s `gamepad` feature to do without.
- Inspector edits can wait. A component says, per field or as a whole, when an edit applies: as it is made (the default), once you stop typing or dragging, or only when you press Apply. A voxel world now waits for Apply, with Revert beside it in the component's header, so typing a number no longer regenerates the terrain at every keystroke; the editor shows "Regenerating…" while it rebuilds. `ComponentSchemaRegistry::apply_when` declares the mode, and the generated component list reports it.
- Weave devtools can edit. Clicking a value in the inspector's Styles section edits it in place, and the new value is written into the stylesheet on the rule's line (only the value changes; comments and spacing stay), then the styles reload from the file. The Game view has a Pick toggle: while the game runs, the next click selects the frontmost element under it instead of reaching the game, so its styles show in the inspector. `weave::set_declaration` makes the same edit for other tools.
- Weave devtools in the editor. Selecting a UI element shows a Styles section in the inspector: its box model as drawn, every rule that matched, strongest first, with the file and line it was written on and the declarations a stronger rule overrode struck through, and the computed values and variables, at the Game view's size. Rules now remember where they were written, through `@use` imports and `@media` blocks, and `weave::matched` and `sindri_weave::inspect` answer the same questions for other tools.
- Weave selectors and lengths come closer to CSS. Structural pseudo-classes (`:first-child`, `:last-child`, `:only-child`, `:nth-child()` with `an+b`, `odd` and `even`, `:nth-last-child()` and `:root`), `:not()`, `:is()` and `:where()` with CSS's specificity, and the `+` and `~` sibling combinators, all in scene order. `calc()` mixes units wherever a length goes, `var()` included, and shorthands keep a `calc()` whole. The Weave showcase styles its feature cards by position instead of a class each.
- UI grid layout. A UI Grid component (`sindri.ui.grid`), or Weave's `display: grid`, places children in columns and rows: tracks are fixed lengths, `auto` or `fr` shares (with `repeat()`), items can start where they say and span tracks while the rest flow into free cells, extra rows or columns are added as needed, and items fill or align in their cells. In Weave: `grid-template-columns`, `grid-template-rows`, `gap`, `row-gap`, `column-gap`, `justify-items`, `align-items`, `grid-column` and `grid-row`. The Weave showcase's row of feature cards is now a grid, drawn identically on desktop and phone.
- Text sizes what holds it. A text element can size itself to its words, measured by its font (Weave's `width: auto` and `height: auto`, or `fit_content` on the UI Box component), plus its padding; a layout that fits its content then grows with its label, and a label is the least a shrinking flex item keeps. The game, the browser export, the editor's views and the Weave showcase's capture measure each draw, and clicks land at the measured size. The showcase's "Live layout" badge now fits its label instead of a hand-set width.
- UI layouts are CSS flexbox. Children can grow into spare room, shrink when there is too little (weighted by size, as in CSS), start from a basis, change order, wrap onto new lines and align themselves, within minimum and maximum sizes, and a layout can size itself to its children. The layout now decides sizes as well as positions, and what is drawn and what is clicked follow it. In Weave: `flex`, `flex-grow`, `flex-shrink`, `flex-basis`, `flex-wrap`, `flex-direction`, `order`, `align-self`, `align-items: stretch`, `justify-content: space-around` and `space-evenly`, and `width`/`height: auto`. As in CSS, a row whose children are wider than it now shrinks them to fit instead of letting them overflow; `flex-shrink: 0` keeps a child's size.
- UI elements have a box model. A new UI Box component (`sindri.ui.box`) gives any element padding and margin per side, and layouts honour them: children flow inside their parent's padding and keep their own margins clear, as in CSS flexbox. Weave's `padding` and `margin` take one to four values as in CSS, with `padding-top` and the other per-side longhands overriding one side, and percentages of the containing element's width. A layout that had Weave padding now starts its children inside that padding, so a phone layout whose content touched its card's border now sits inside it.
- Weave has `box-shadow`: an offset, blur, spread and colour, drawn as a soft copy of the shape behind it with matching corners. The Weave showcase's cards and buttons now cast them.
- Weave `:hover` and `:active` now work in the running game: the editor's Play, native games and browser exports lay the pointer's states over the styled UI each frame, and clicks land where things are drawn, so a button a media query moved is clicked where it is drawn. A game's styles are applied when it starts and when the screen changes shape, and a value a script writes afterwards is not styled back, as an inline style beats a stylesheet. Stylesheets can animate between states with CSS `transition` (durations, delays, `all`, and `ease`/`ease-in`/`ease-out`/`ease-in-out`/`linear`/`cubic-bezier` easings) on colours, lengths and numbers.
- Weave selectors now work as in CSS: element names (`text`, `button`), compound selectors (`button.primary:hover`), descendant and child combinators (`.menu text`, `.menu > button`), selector lists, and CSS specificity. Text properties inherit from their container, custom properties (`--accent`) and `var()` with fallbacks give themes one place to live, and media queries combine with `and` and commas. `:disabled` and `:checked` follow the entity's own data; `:hover` and `:active` match when a host passes pointer state in. Existing stylesheets keep working, `sindri.ui.text` included. `docs/ui-direction.md` sets out the plan to make Sindri's UI better than Unity's and Godot's.
- A platformer showcase: `games/platformer`, a small side-view game with a painted, solid level, a hero who runs and jumps, coins, a flag and a camera that follows. It is the first of a set of genre showcases, and is playable in the editor and on the site.
- Moving a physics body's transform from a script now moves the body, as it does in Unity, instead of the next physics step putting it back. A respawn or an edge clamp written as a position now works; a position-kinematic body treats the move as its next target, so a platform moved this way carries what stands on it.
- A frictionless collider is now frictionless against everything: friction combines as the smaller of the two rather than their average, so a platformer's hero pressed into a wall slides down it instead of clinging. Two equal frictions combine as before.
- Camera follow, confinement and shake now run in the editor's Play and in exported browser games, not only in the camera example.
- Tilemaps can be solid. Add a Tilemap Collider 2D beside a Tilemap and every painted tile collides, merged into as few boxes as cover them so characters don't catch on tile seams; list sprites as passable for decoration. A scene can set its own gravity with a Physics 2D World component, so a platformer falls in the editor's Play as it will in its build.
- The Scene view shows collision. Every collider is outlined in green, a tilemap's generated boxes included, and the selected collider has handles: drag a box's edge, a circle's radius or a capsule's height to size it, as one undoable step.
- The Scene view has a 2D mode beside Perspective and Ortho: straight onto the level, with no perspective, and dragging pans instead of orbiting. A scene whose camera is a flat 2D camera opens in it, framed on what that camera sees.
- The Scene view's camera, light and collider markers no longer draw on top of the floating panels.
- Mountains no longer have holes through them. Natural terrain's overhangs were carved into thin ridges from both sides at once, and tunnels broke through ridges too narrow to hold them, leaving windows to the sky with rock hanging above. Undercuts and cave mouths now only cut into walls thick enough to keep rock behind them, so ranges keep their ledges and overhangs but peaks and ridges are solid, and snow caps stay on the peaks. Worlds from the same seed change shape slightly in their ranges.
- The Scene view has a scene-lighting toggle, as Unity's does: the bulb in its toolbar. On (the default), the view is lit exactly as the game is, so a sun at zero and no ambient is black. Off, the Scene view is lit evenly by the editor, from over the camera's shoulder with no shadows, so a dark scene can still be worked on; the Game view always shows the scene's own lighting.
- Sunlight now lands on the side of the world its arrow points at. The textured shader worked out which way a surface faces with the sign backwards, so every face turned away from the sun was the one lit, and shadows (which were right) fell on the lit side. Scenes look lit from where their Sun is aimed, and slopes facing the sun are bright instead of nearly black.
- Blocks can move and glow. The built-in water now ripples and lava churns and glows, drawn by shifting where each face reads rather than by rebuilding the world, and a block set can give any face an animation (frames on one texture, and a speed) and any block a glow. Leaves and other blocks that don't hide their neighbours are now cut out, holes and all, and a lake no longer has walls inside it. The block editor has a Glow field.
- Blocks are premade and chosen by name. The engine ships a block set, `builtin:blocks` (grass, dirt, stone, sand, snow, ice, mud, moss, gravel, clay, planks, log, leaves, water, lava and more), and a voxel world can name a block set and have its generator say "grass" and "water" instead of material numbers. A new Voxel World is built from the built-in blocks, and Voxel Lab now is too. The inspector offers blocks as a menu of cubes.
- Block sets are edited in the inspector: select a `.tileset` to see its blocks as cubes and edit each block's faces (with pictures), whether it hides its neighbours, supports, is walkable, its height and its tags. "New block set here" starts one as a copy of the built-in set.
- Blocks can carry tags (`hot`, `liquid`, …) that scripts ask about with `Grid.tagged(volume, column, row, level, tag)`.
- Exports no longer look for engine-provided `procedural:` and `builtin:` assets on disk.
- Texture fields in the inspector show the picture they name beside the reference, and the reference picker shows each texture and sprite as a picture. A long reference is cut off before its sprite name, so fields naming different parts of one sheet no longer look identical.
- The inspector draws voxel materials as the blocks they are: every material picker (a biome's surface, the water, the trunk…) shows each option as a small cube with its own top and side textures, and each material in a Voxel World's list shows its cube beside its ID.
- The sun is now an object in the scene, like a camera: a Directional Light entity aimed by its rotation. The Scene view draws it as a sun with an arrow showing which way the light goes (and, when selected, a trail across the world), it can be clicked and rotated, and Create GameObject offers Directional Light. New scenes start with one. Scenes that stored a sun direction in the Environment are migrated to a Sun entity aimed the same way (scene format 10), so a light shining upwards, which put shadows on hilltops, is now visible as an arrow pointing up.
- Voxel Lab's Scene view now starts on open ground by the coast instead of inside a mountain.
- Voxel worlds can generate natural terrain: continents and sea, mountain ranges with overhangs and cave mouths, rivers, beaches, snow lines, frozen seas, tunnels and caverns, trees, and biomes you define by climate, surface materials, tree density, roughness and terraces. Biome edges fray into each other and their heights blend. The inspector edits every field with material pickers (with *None* for optional features) and switches between generators cleanly; Voxel Lab now opens on a natural world.
- Canvas panels can be moved anywhere over the scene: drag the empty part of a panel's tab strip to float it, drag its bottom-right corner to resize it, and double-click the strip or drop it back in its corner to anchor it again. Panels snap to the canvas edges and to each other, and cannot be dropped off the canvas.
- A voxel world is no longer rebuilt when an unrelated texture loads or hot-reloads; it compiles its sections again only when a texture its own faces draw with changes.
- Editor layout fixes: the search box no longer sits on top of the Play controls in the Docked and Wide arrangements; docks share the window so the scene always keeps room, and a dock squeezed by a small window grows back with it; Wide keeps the Assistant as a tab beside the Inspector; the Project browser in a bottom dock lists every file; the Scene view's status plate and axes no longer hide under the title bar and transport; the Canvas inspector uses the height of the window; and long read-only values end in an ellipsis.
- The console separates what is wrong now from what the log remembers: the status bar names the current problem and counts current problems rather than past errors, the count clears as soon as the cause is fixed, and clicking it opens the console's new Now view. The console tab carries the count on its icon instead of a chip that covered neighbouring tabs, console lines can be copied from a right-click, and the entity a line is about is linked beneath it rather than cut off beside it.
- The inspector now edits Environment and Voxel World with the right controls: colour pickers for their colours, values held inside the ranges the engine accepts, a menu for shadow map size and tone mapping, texture pickers for voxel material faces, and whole-number section coordinates.
- Texture pickers offer the sprites cut from the project's sprite sheets, so references like `blocks.png#stone-0` are no longer marked as missing.
- The editor no longer freezes the Scene view when a Voxel World or Environment value is invalid: the last valid terrain and lighting keep drawing, and the problem is reported against its entity, naming the exact field and the values it accepts.
- Adding a voxel material in the inspector now gives it an unused ID, a material the generator still uses cannot be removed, and the generator's surface, subsurface, and deep layers are chosen from the defined materials.
- The viewport's error banner no longer sits under the status bar or the bottom-left panel, and wraps long messages.
- Added authored world fog/atmosphere with distance, exponential-density, and height contributions, plus Voxel Lab horizon integration and consistent browser materials.
- Camera gameplay can now trigger engine-owned shake through Decay with `Camera.add_trauma`, and the camera acceptance demo moves its target and triggers impacts from Decay instead of bespoke Rust gameplay.

All notable user-facing changes to Sindri Engine are documented here.

This project has not made its first release yet. Until then, changes are collected
under `Unreleased` and kept intentionally concise. Detailed implementation
history, design rationale, debugging notes, and test archaeology belong in pull
requests, commit history, and subsystem documentation rather than this file.

## [Unreleased]

### Added

- A left virtual stick and a right jump control for Lamplighter touch screens, usable together with two fingers.

- Add reusable `render_material` profiles for shared 3D factors and sidedness,
  with explicit local overrides, validated editor authoring, asynchronous loading
  and automatic export discovery. Lighting Lab shares profiles across cubes,
  inline surfaces and models; texture bindings and unified alpha remain open.

### Changed

- 2D physics rays, overlaps and shape casts select collider candidates through
  a synchronized spatial index, preserving filtering, inside hits and exact ties.
  Controller penetration, movement and ground probes share the index with
  bounds expanded for skin; historical platform support reads only its own pieces.

- A text element that draws nothing else is pressed where it is drawn: its
  `bounds`, pivoted on its anchor, rather than a box centred on its position.
- Tab and the arrows can reach a row scrolled out of view inside a scroll
  region, which then scrolls to show it, as a browser does.

- Orbital Last Stand's UI is redesigned: a title that separates the pilot's
  identity and best score from the play actions, with START above Boss Rush
  and the boss picker; a HUD whose sector, survival clock, score and level,
  and hull and XP sit in their own backed panels, with a warm boss frame; a
  module chooser of structured cards that fits four offers and portrait;
  a pause screen with a scrolling field manual; and results with a score
  breakdown. It is laid out for desktop, short landscape, phone and tablet.
- Tab order follows the order a scene is written in, as a browser's does,
  rather than reading the screen top to bottom.
- A width or height Weave gives a text element is also the box its words
  wrap and align in.
- A scroll region measures its laid-out content, so a list is never squeezed
  into the view or cut short by a stale `content_height`.

- Voxel Lab's authored scene now uses the engine-owned `sindri.voxel_world`
  component in the editor, with deterministic layered terrain, bounded 3D
  section residency, neighbour-aware block meshing, and persistent cached GPU
  geometry. Its previous editor-only tile-volume island is no longer the scene
  being inspected.
- Voxel Lab now uses the editor Scene view's orbit, pan, and zoom interaction
  model. Touch screens use one finger to orbit and two fingers to pan or pinch,
  while an unmoved tap still digs at the camera focus.
- Causeway now materializes deterministic 16×16 terrain chunks around its
  camera instead of generating a complete 160×160 island up front. Its sparse
  navigation work follows loaded ground rather than the declared world bounds,
  so Build panning can reveal new biomed terrain inside a 65,536-cell envelope
  without scanning that envelope.
- A tile volume is resolved into the faces it draws once and kept, rather than
  rebuilt every frame. Extracting Gather's farm cost 6.7 ms a frame and now
  costs 1.2 ms; the volume's own share of that fell from 5.7 ms to about 0.2 ms.
  Only the depth each cell sorts at is measured again, because that is the only
  part a moving camera changes. An entity now carries a revision, and texture
  and tile-set bindings a generation, so an edit to a volume — its cells, its
  grid, its transform, the art it draws from, or whether it takes part in the
  scene at all — is picked up on the next frame.

### Added

- Add a native scene adapter for resident voxel collision snapshots, with
  occupancy/policy revision caches, transformed partial shapes, owner lifecycle
  and bounded atomic reconciliation. Authored terrain and game-host wiring
  remain open.

- Add independently replaceable 3D static collider groups under one entity,
  preserving the body and unaffected collider handles. Queries update immediately;
  invalid changes fail before mutation. Scene streaming/voxel wiring remains open.

- Add renderer-independent voxel section collision geometry: exact boxes with
  deterministic full-cube merging, partial slabs/posts, explicit noncollision
  policy, typed invalid-shape errors and bounded output. Scene/solver integration
  and actual game collision remain follow-up work.

- Standalone 3D spawn controls queue finite velocity setters and impulses in
  order, replay after collider mass is known and expire unresolved requests.
  Scene synchronization validates pending controls before mutating its batch.
  Typed `Physics3d` controls now use this queue for valid authored bodies and
  nonempty colliders. Pending reads copy last setters/authored starts; impulses
  resolve at materialization and locked angular velocity stays zero.
- Typed 3D box/capsule overlaps and casts accept rotation axes and radians,
  returning copied hits or entity lists over indexed active geometry. Sweeps
  keep orientation fixed; invalid dimensions or rotations fail explicitly.
- Typed `Physics3d.layer(name)` and `mask(names)` select checked query masks
  from active authored 3D world labels independently of 2D. Invalid names,
  arguments or world settings fail explicitly; masks retain all 32 bits.
- Typed `Physics3d.raycast`, `overlap_sphere` and `cast_sphere` over indexed
  synchronized 3D geometry, with masks, sensor opt-in, whole-entity exclusion
  and inactive/despawned filtering. Closest hits are copied optional `RayHit3d`
  values; overlaps are sorted unique copied entity lists. Rotated box/capsule
  probes and occupied/resident/edited voxel proof remain open.

- Typed `Physics3d` Vec3 velocity/angular-velocity controls, dynamic impulses and
  copied non-draining collision/sensor event lists in shared native/browser
  sessions and editor Play. Invalid input preserves motion; inactive/stale handles
  fail explicitly. Controls retain authored starting fields; the later spawn
  queue and rotated probe slices extend this surface. Voxel/game proof remains
  open.

- Registered 3D body, compound collider and gravity-world scene components plus
  `ScenePhysics3d` lifecycle/transform synchronization and parent-space write-back.
  Invalid batches, mixed 2D/3D ownership and moving Z-locked 3D bodies fail before
  runtime mutation. Shared native/browser sessions and editor Play now step both
  dimensions before scripts; fresh Play, Stop and scene replacement reset editor
  solvers. The Physics menu includes the three 3D components. Native editor
  interaction, typed rotated probes and voxel/game proof remain open.

- A query-only per-piece spatial index for 3D rays, overlaps and shape casts,
  refreshed on insertion/removal/teleport and completed simulation steps while
  preserving filtering and deterministic hit ordering.

- Standalone 3D rays, overlaps and shape casts with quaternion probes, masks,
  sensor opt-in, whole-entity predicates and deterministic results at current
  body poses. Game/editor/Decay proof remains open.

- A standalone 3D physics engine world with fixed-step bodies, validated unit
  quaternions, box/sphere/capsule pieces, masks, collision/sensor events and
  basic velocity/impulse/kinematic/teleport controls. Game/editor host, Decay and
  voxel collision integration remain pending.

- Radio groups (toggles sharing a `group`) and dropdowns (`sindri.ui.dropdown`
  with `sindri.ui.option` rows the engine shows while open), with Weave's new
  `:open` state. Decay gains `Ui.selected`, `Ui.set_selected` and
  `Ui.is_open`.
- Text fields edit at a caret with a selection: arrows, Home, End, Shift,
  Delete, and Ctrl or Cmd with A, C, X and V through the system clipboard.
  `Ui.caret`, `Ui.selection_start` and `Ui.selection_end` let a script draw
  them.
- In a browser a text field takes IME composition, a paste and a phone's
  on-screen keyboard, through a hidden textarea that holds the page's focus
  while the field edits. Natively, IME is allowed while a field edits and its
  commits are not typed twice.
- The arrow keys and any pad's d-pad move focus toward the nearest control,
  South presses and East backs out, and a row reached inside a scroll region
  is scrolled into view. A control marked `autofocus` takes focus when it
  appears; with nothing focused and nothing asking, the arrows focus nothing.
- Orbital Last Stand's boss picker is a scrolling dropdown of the twelve
  bosses, and its callsign field draws its caret and selection.

- UI widgets: `sindri.ui.toggle` (switch or checkbox), `sindri.ui.text_input`
  (single line, Unicode, submit on Enter) and `sindri.ui.scroll` (vertical,
  clipped, wheel, drag and touch), keyboard focus with Tab and Shift+Tab, and
  Weave's `:focus` and `:checked`. Decay gains `Ui.is_checked`,
  `Ui.set_checked`, `Ui.input_text`, `Ui.set_input_text`, `Ui.scroll_offset`,
  `Ui.set_scroll_offset`, `Ui.changed`, `Ui.submitted` and `Ui.is_focused`.
  Buttons can be `disabled`.
- `examples/ui`, the Weave Control Room, published to Pages and smoked in a
  browser on desktop and phone.
- Orbital Last Stand has a pilot callsign carried onto the HUD and results,
  and a saved compact-HUD switch.
- `project-capture` photographs any project offscreen through the same
  session and Weave styling the browser uses, after scripted clicks, keys,
  typing and wheel steps.
- Added an authored world post-processing stack with exposure, tone mapping,
  contrast, saturation, bloom, and vignette. World effects resolve before
  overlay/UI rendering so interface content remains crisp; Voxel Lab now uses
  the complete stack as its presentation acceptance surface.
- Added mesh-time ambient occlusion for voxel corners and contacts, with authored environment strength and Voxel Lab proof.

- Added authored directional shadows for textured world and voxel geometry, with environment controls for coverage distance, map resolution, and bias.

- Added authored ambient and directional world lighting to `sindri.environment`; textured 3D and voxel geometry now share the same renderer lighting in editor and browser Voxel Lab, while scenes without it retain the previous unlit appearance.

- `sindri.environment` now authors ambient and directional world lighting. Textured 3D geometry and Voxel Lab terrain respond to the same sun direction, colour, and intensity in editor and browser rendering.

- Cached voxel sections outside the current camera frustum are no longer
  submitted for drawing or uploaded merely because they remain resident. Their
  compiled CPU geometry stays cached and becomes drawable when the camera can
  see it again.
- `sindri-scene` now bridges semantic block-mesh output into deterministic
  texture/atlas batches and revisioned persistent renderer commands. Stable GPU
  identities survive remeshing, superseded worker results are discarded before
  upload, and sections leaving residency explicitly release their buffers. The
  first bridge deliberately accepts opaque block batches only; cutout and
  transparent pipelines remain follow-up work.
- `sindri-render` now owns persistent textured-mesh GPU buffers keyed by an
  opaque cache identity and monotonic revision. Unchanged meshes reuse their
  buffers, pending replacements keep the last uploaded geometry drawable, and
  explicit release drops meshes that leave residency. Cache counters expose
  installs, uploads, reuse, stale draws, misses, and releases for diagnostics.
- Persistent textured meshes use 32-bit indices, so a highly fragmented 16³
  voxel section is not limited by the 65,535-vertex ceiling of transient
  authored meshes.
- Voxel mesh work now carries a monotonic section revision and meshing profile,
  and a renderer-independent persistent cache keeps old compiled geometry
  available while a replacement is built. Superseded worker results cannot
  replace newer terrain, and one removal releases every cached profile of a
  leaving section.
- `sindri-voxel` now compiles neighbour-aware block sections into indexed
  CPU geometry split into opaque, cutout, and transparent passes. Material and
  face identity remain semantic so renderers can choose their own atlas or
  shader path, while configurable occlusion avoids internal transparent faces.
- Causeway can derive a smoothed textured triangle surface from the same
  deterministic voxel terrain that still owns picking, building and navigation.
  The first slice intentionally meshes only one 16×16 camera-centre chunk so
  the visual approach can be judged before voxel generation and chunk meshing
  are promoted into a general engine subsystem.
- `sindri.mesh` can carry explicit textured surface triangles, including a
  sprite-sheet region, through the opaque 3D render path.


- Tile volumes expose one shared engine chunk coordinate and sparse runtime
  chunk store. Scene serialization remains the readable sparse cell list; the
  runtime store is the unit generators, renderers, and future persistence use.
- `Grid.walkable(floor, x, y)` tells a script whether a walker can stand where a
  point falls, read from the same walkable surface the pathfinder uses. A script
  could previously ask only about a route between two entities, so a game moving
  its own player had no way to ask about terrain at all.

### Fixed

- Giving up from Orbital Last Stand's pause screen ends the run; it used to
  leave the game paused with no hull until it was resumed.

- Causeway's Wanderer now crosses grid steps smoothly, and the Play camera
  holds a small dead zone before easing after it. The generated scene also keeps
  its intended central start instead of a stale override placing it beside the
  world edge, so BUILD panning has terrain around it in every direction.
- Causeway Play taps now move their runtime Target instead of having its
  authored cell snap it back every frame, and use the voxel's grid row rather
  than its height, so the Wanderer walks toward the place the player tapped.
- The editor no longer draws a tile volume as a comb of vertical stripes after
  the first edit. A volume names a tile set rather than a texture, so nothing
  about the world asked for the sheet that cuts its blocks: the tile set's
  arrival requested it, and the next pass over what the scene references —
  which runs on every edit — found nothing claiming it and released it. An
  unbound sheet does not blank the texture, it unbinds the slicing, so every
  face resolved the whole strip at once.

- Gather's player no longer walks across its moat or into its outcrop. It
  compared positions against tagged props, which are entities; water and the
  hill are not, so neither stopped it while the Wisp routed around both.
- Gather's player no longer starts inside the hill. Its scene authored no
  starting position, so it began at the world origin — the middle of a 25x25
  island, which is the top of the outcrop. It now starts on the path just north
  of the ridge.
- A walker standing over water is no longer lifted onto it. An authored occupant
  rests on whatever holds it up, but a walker stands only on what it can stand
  on.

- The editor's grid chooser offers entities carrying `sindri.tile_grid` as well
  as `sindri.tilemap`. It asked for the flat map alone, and no scene has carried
  one since Gather moved to a volume, so a grid could not be named at all.

- Flat ground no longer draws over what stands on it. Something placed on a
  grid sorts past ground no higher than its feet, since no face of such a cell
  can cover it; a block raised a step ahead is a wall and still covers it.
- A tile volume's depth is taken from a cell's column rather than from each
  drawn face, so raising a block no longer moves it toward the viewer and a
  block's own faces are no longer sorted against each other.

### Changed

- A grid placement names its grid with the same stable-ID type a grid occupant
  uses, and carries the cells it covers. Standing something over a hole is now
  an error naming that cell instead of placing it at height zero.

- A tile says whether its top holds anything up (`supports`) and whether a
  walker can stand on it (`walkable`) instead of one `solid` flag that meant
  both. Water supports without being walkable, so a pond is no longer
  indistinguishable from a hole. `solid` still reads as `walkable`.

- Resolving a tile volume indexes its cells once instead of scanning them for
  every neighbour it asks about, and no longer revalidates a bound tile set on
  every frame.

### Added

- A tile can declare several looks with weights, chosen per cell from a stable
  hash of where the cell is, the tile's name and the volume's `variant_seed`.
  One tile ID replaces a family of near-identical ones, and the same field
  looks the same on every machine and after every reload.

- Choosing a `.prefab` in the project browser arms it, and clicking a cell
  in the Scene view puts it there: one undoable step, standing on that cell of
  that grid, keeping whatever footprint the prefab declares.

- The editor can read a `.prefab` and put it into the open scene as one
  undoable step, with every entity given a stable identity nothing else is
  using. Distinct from the runtime's spawn, which deliberately gives none.

- `sweep_occlusion` walks a virtual actor over every standable surface of a
  scene and reports where something that cannot cover it is drawn over it
  anyway, with the reason attached. Gather's farm is swept in its own test.
- The editor draws that report on the grid it describes, under Build →
  Ordering, so a run of faults along one edge reads as one cause.
- Entities can be placed by naming a grid cell rather than a world position.
  `sindri.grid.placement` derives the transform from the cell, including the
  height of the ground in that column and the Z that orders it, so raising the
  ground raises what stands on it and nothing authors a draw order.

### Changed

- Draw order in a 2D scene is derived from where a thing stands rather than
  from an authored render layer. Gather carries no layer on any world sprite,
  its player and wisp scripts no longer compute one, and `layer_step` on tile
  volumes is replaced by the grid's `depth_step`.
- Gather's floor is a stacked tile volume rather than a flat tilemap, and no
  scene in the project carries `sindri.tilemap` any more. Water is no longer
  walkable, so the moat and pond now bound the island.

### Added

- Tile volumes can spread their cells across render layers with `layer_step`,
  which is what lets blocks interleave with sprites in a 2D scene.
- The isometric baker can give a material grain: a texel grid that shifts each
  texel a step along the ramp it already has, so a baked block reads as a
  surface rather than as three flat faces. Gather's blocks are rebaked with it,
  and the set grows from three tiles to twelve, half-height slabs included.
- Decay reaches a stacked volume: `Grid.block` and `Grid.set_block` read and
  write the tile in one cell at a column, row and level, and a script's
  pathfinding now sees the holes and walls a volume floor makes.
- Grid navigation derives what a stacked volume allows: a column with nothing
  solid in it is a hole, and `sindri.grid.navigation` gains `max_step` deciding
  how big a step between columns a walker may take, with a slab counting as
  half. A volume only becomes the floor once the flat map is gone.
- Tile volumes work in orthogonal projection as well as isometric. The
  projection now decides which faces a view can see and how cells are ordered
  back to front, so one tile set serves both.
- Tile set tiles can declare how much of their cell they fill, so half-height
  slabs and other partial blocks are logical cells rather than art tricks.
  Face culling now hides a face only when a neighbour covers it completely.
- Grid position, placement, pathfinding and navigation now accept a floor
  carrying `sindri.tile_grid` instead of `sindri.tilemap`, so a scene can move
  onto a stackable tile volume without its scripts, walls or occupants moving
  with it. `Grid.tile` and `Grid.set_tile` remain flat-map calls and say so.
- Added an optional advanced sprite colour transform with independent RGBA
  multiply and offset, alongside the existing simple tint, reachable from
  scenes, the editor and Decay.
- Added a typed batch preflight and agent-facing runtime-contract guidance for
  Decay gameplay scripts.
- Added multi-scene projects and runtime scene switching, including persistent
  scene state, exported secondary scenes, and Decay scene navigation.
- Added reusable profile assets with runtime, editor, Decay, and export support.
- Added Tile System 2 foundations for stackable isometric volume cells and
  globally correct transparent painter ordering.
- Added a substantially expanded Gather showcase with a larger farm, multiple
  playable places, camera following, richer baked environment art, and
  script-editable grid tiles.
- Added broader Decay gameplay APIs, including runtime signals and improved
  environment interaction support.
- Added extensive Orbital Last Stand and Orbital Baked gameplay, boss, module,
  UI, hazard, combat-lab, and visual-parity work.
- Added a managed local-AI runtime path using llama.cpp, model manifests,
  hardware-aware compatibility grading, and continued Ollama support.
- Added major editor authoring improvements across project browsing, component
  editing, asset pickers, snapping, play controls, scene handling, diagnostics,
  viewport behavior, and native rendering.
- Added export/browser hardening, asset manifests, verification, hot reload,
  sprite sheets and animation, GPU-backed rendering tests, and additional
  project/scene validation.
- Added isometric baker improvements and reproducible baked-art workflows used
  by Gather and Orbital.

### Changed

- Reworked Sindri's editor around native `egui`/`wgpu` rather than the earlier
  Tauri/React direction.
- Evolved the scene format through multiple migrations as transforms, sprite
  sheets, component namespaces, cameras, and UI ownership were clarified.
- Consolidated 2D and 3D transform/rendering behavior so world sprites, UI,
  cameras, animation, physics, grids, and scripts share clearer runtime
  contracts.
- Expanded Gather's role from a passive capability showcase into a playable
  farming-game proving ground that may drive reusable engine features.
- Reworked Orbital's baked assets and bosses so authored rotations, silhouettes,
  attacks, reactions, and environment interactions survive the baking pipeline.
- Raised the Rust MSRV as required by current `wgpu` and `egui` releases.

### Fixed

- Fixed exports dropping a sprite sheet whose texture is named from the project
  root rather than from `assets/`, which shipped the texture with no slices and
  drew an animated sprite as its whole sheet in one quad.

- Fixed native and browser hosts discarding the underlying cause of application
  failures at their reporting boundary.
- Fixed exported browser projects failing before asset fetch when one asset kind
  exceeded the loader's default queue capacity.
- Fixed numerous editor issues involving selection, unsaved changes, undo/redo,
  viewport color handling, hierarchy layout, project browsing, asset loading,
  console reporting, and controls that previously did nothing.
- Fixed browser/export failures involving missing scripts, incorrect asset kinds,
  manifest handling, startup errors, and project-relative asset paths.
- Fixed rendering bugs involving multi-batch cameras, transparent ordering,
  sprite-sheet frames, world/screen sprite behavior, viewport color spaces, and
  multi-mesh clears.
- Fixed gameplay/runtime bugs involving input edges, inactive scene lookups,
  scene identity handling, projectile hit ordering, camera activation, enemy
  placement, hazard persistence, and physics interactions.
- Fixed Spine sections failing at runtime when a destroyed middle section
  severed and promoted the surviving rear chain.
- Reworked Spine's body to follow one exact cardinal route instead of being
  dragged around corners like a physics rope, including after a severed rear
  chain becomes independent; Spine now hunts through the arena rather than
  circling its boundary.
- Fixed the isometric baker's authored rotations, which had been supplied in
  radians while the baker interpreted them as degrees.

## Changelog policy

Keep entries release-oriented and readable:

- Record changes that matter to users, game authors, plugin/tool authors, or
  downstream integrators.
- Group entries under `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`,
  or `Security` where useful.
- Prefer one concise entry for a feature or behavior change. Do not append a
  debugging diary, implementation essay, test narrative, or commit-by-commit
  history.
- Put detailed rationale and technical history in the pull request, relevant
  documentation, or architecture decision record.
- When the first release is cut, rename `Unreleased` to that version and date,
  then add a fresh empty `Unreleased` section above it.
- Added the authored `sindri.environment` presentation component and connected the existing bloom renderer to editor viewports and Voxel Lab, making bloom scene-controlled instead of a stranded renderer-only capability. Voxel Lab now serves as the acceptance lab for the world-presentation roadmap.
