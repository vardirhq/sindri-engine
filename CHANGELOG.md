# Changelog

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

- Make editor scene Save/Save As and subtree prefab authoring remap registered runtime-local entity references to assigned stable IDs automatically. Invalid references fail before writing or adopting a path; the live world and unknown fields remain intact. All four joint kinds survive save/reopen and saved-subtree reuse, with the platformer’s Decay-spawned windmill proving the real editor save path.

- Add damped slider position motors with signed local-axis distance targets, stiffness/damping and force caps. Typed Decay controls select position or velocity drive while retaining body motion and rebuild semantics; old scenes keep velocity mode. Platformer O parks or releases its lantern trolley, and J restores the selected drive after rebuilding its rail joint.

- Add force-based hinge position motors with validated relative angular targets, stiffness/damping and torque caps. Typed Decay controls switch between position and velocity drive without resetting body motion; old payloads retain velocity behavior. Platformer P holds both windmills or resumes reversal, and H rebuilds the placed hinge while preserving its selected drive.

- Add typed `Physics.create_slider_joint` with finite local anchors, unit local axes and optional travel limits. Empty owners, scoped endpoints and settings validate atomically; motor controls work before synchronization. Platformer J rebuilds its bounded trolley slider while retaining its current direction and independently owned spring.

- Add typed `Physics.create_spring_joint` with finite local Vec2 anchors, positive rest length and non-negative stiffness/damping. Empty owners and scoped endpoints validate before mutation; creation preserves body motion and legacy constraints. Platformer B rebuilds its lantern spring without resetting the tuning phase or affecting the trolley rail.

- Add typed `Physics.create_hinge_joint` with finite body-local Vec2 anchors, validated empty owners and scoped endpoints. The new hinge starts with limits/motor disabled and accepts existing motor controls before fixed synchronization. Platformer H rebuilds its placed windmill hinge independently of the spawned mechanism.

- Add typed `Physics.create_distance_joint` to author a maximum-distance constraint on an existing owner with no joint. Scoped references and positive lengths validate atomically; creation keeps body motion and takes effect at next fixed synchronization. Platformer C repairs its cut lantern cord using the selected hook and length.

- Add typed authored 2D joint removal with `Physics.remove_joint`. It releases the constraint at the next fixed synchronization while keeping the owner, its other components and endpoint bodies; legacy connections remain separate. Platformer Z cuts the lantern cord for the remainder of the run.

- Add typed 2D joint endpoint retargeting with `Physics.set_joint_endpoints`. Scoped handles become stable scene IDs or local prefab paths; null clears an endpoint. Invalid references fail atomically while settings, unknown fields and body motion are retained. Platformer switches its lantern between two hooks with R.

- Add scoped entity-reference pickers to registered component fields in the native inspector, including nested lists. Choose by name/ID, clear a reference, or keep typing; missing and inactive targets are marked. The picker shares runtime scene/prefab resolution rules, and all four joint kinds exercise picker edits and command undo.

- Add opt-in registry-based serialization for spawned-prefab entity references. After assigning stable IDs, `World::to_scene_with_references` remaps registered fields, including nested lists, without editing the live world. Invalid or unresolved references fail the save. All four joint kinds and the platformer's spawned windmill exercise reopening; qualified joint scene IDs take precedence over relative namespace lookup.

- Preserve authored joint root references in placed nested prefabs through native/browser scene entry and scene namespaces. Editor prefab reload, duplication and undo retain or rebase runtime aliases without serializing them. Platformer's level windmill is now a placed nested assembly, sharing its prefab with runtime spawning.

- Add reversible enabled flags to authored 2D distance, hinge, slider and spring constraints, plus typed Decay state controls and owned distance tuning. Suspension/reconnection preserves endpoint bodies and unknown fields. Platformer reels its lantern tether with T and releases/reconnects it with L. Invalid suspended distance settings now fail validation even before endpoints are available.

- Resolve authored 2D joint endpoints inside runtime-spawned prefabs through separate instance-local identity. Repeated spawns keep their own roots and children without assigning saved scene IDs. Platformer adds a reusable powered windmill placed/removed with V. Nested runtime root aliases survive library expansion; canonical paths take precedence and ambiguous aliases fail before spawning. Placed-root aliases now survive library-aware native/browser scene entry.

- Add scene-authored 2D sliders with travel limits and force-capped velocity motors, plus damped force-based springs. Typed Decay controls tune their runtime components through synchronization and rebuilds. Platformer adds a reversing lantern trolley with a retuned spring suspension. Structural controls and full prefab references remain in progress.

- Add scene-authored 2D hinges with local anchors, angle limits and torque-capped velocity motors. Typed `Physics.set_hinge_motor` controls their next synchronized drive without resetting body motion. Platformer adds a reversing windmill. Sliders, springs and full prefab reference integration remain in progress.

- Add scene-authored 2D distance joints with stable scene endpoint IDs, explicit ownership, edit/undo support and reconnection after endpoint rebuilds. Platformer has a wind-driven hanging lantern. Other joint kinds, typed ownership controls and spawned-prefab references remain in progress.

- Add reusable 2D physics material profiles with shared coefficient validation and explicit local overrides. The editor can create, select and hot reload them while preserving body motion; exported games collect and load their references. Platformer’s crate and planks share a wood material. Native project capture now expands placed prefabs before scene entry, matching browser delivery.

- Add copied 2D solid contact snapshots to runtime and typed Decay: world points, support normals, normal/friction impulses and force. Sleeping contacts keep support geometry; sensors stay separate. Platformer jumps use solved support and its crate flashes on hard landings.

- Add 2D force, torque, angular velocity and angular/world-point impulse controls in runtime and typed Decay. Forces accumulate for one fixed step; spawn requests preserve call order. The platformer has a wind-driven wooden crate that K or the west controller button tosses and spins.

- Add authored 2D one-way platforms with configurable local support normals and timed Decay drop-through. The platformer has visible planks and Down/S/d-pad down drops through them; support-side grounding prevents jumping from underside sensor overlaps. Ordinary floors and sensors remain active.

- Add opt-in continuous collision to the 2D physics engine and live dynamic-body controls. Old body payloads default to discrete collision; sensors remain discrete. Scene authoring and typed Decay controls preserve velocity and joints through live toggles and the spawn window; the platformer hero opts in.

- The editor's welcome window is redesigned around what a returning or new user needs: a drawn dusk-range banner with the Sindri mark; recent projects with a search box, "2 days ago" times and name-tinted tiles; a Learn panel opening the guides in `docs/` (locally in a clone, on GitHub otherwise); and an Examples panel listing every shipped game project with a one-line summary instead of Causeway alone.

- Low Tide separates flooded flats from permanent water: empty-handed crew can swim, ordinary crates can be waded out without automatic loss, and golden floats mark submerged wrecks. Shallow dives start while swimming and surface in the same water; one floated ore bundle must be hauled back up the ramp. The ebb no longer recalls divers from permanent water. Walking uses a closer, smoothly eased camera, with whole-crawler framing restored aboard. `Camera.orthographic_size(camera, size)` adds explicit, validated runtime projection sizing.

- Low Tide wrecks contain six visible, individually selectable crates: three scrap, wood, ore and fibre. Set carried cargo down with X / Drop and pick it up again; only boarding stows it. Wreck streaming retains each taken crate, while loose ground cargo washes away in a flood.

- Low Tide adopts dark instrument panels, brass details and authored icons, with live hold segments, speed and tide meters. Nearby cargo is highlighted and named in a contextual action panel; empty Use actions are hidden. Discarding a crate now requires a second press, cancelled by movement, Escape or a timeout. Crawler frames gain metal detailing, pipe fittings and painted lamp glows.

- Low Tide gets a responsive workbench with directly selectable blueprints, live cargo costs, installed/limited states and explicit build/close actions. A compact tide/status HUD and consistent touch controls leave more room for the world.

- Low Tide gains high-tide diving from the flooded crawler ramp into a side-view cutaway of a nearby submerged wreck. Swim through a broken hull, manage air, recover a heavy sealed cache and return up the boarding line for real ore and scrap. The tide continues while diving; air loss, Recall and the ebb return crew safely without the loot. Recovered wreck caches remain empty during the voyage. Keyboard and phone touch share the same controls.

- Low Tide gains a working construction bench: spend physical cargo on stern extensions (two rows and four slots, up to three extensions), a bunk, and a stronger engine. Construction keeps the deck origin and crew fixed while driving, grows the treads and shadow, and adds permanent weight. Keyboard, gamepad bindings and phone buttons share the blueprint flow.

- Low Tide starts with a small seven-by-nine crawler, one engine, a helm, a basic workbench and four cargo slots. The deck keeps a stable construction origin and segmented tread belts, ready to grow without shifting the crew or stretching its cleats. Movement, ramp boarding, gathering and flood tests now play the starter layout.

- Low Tide gains biome gathering and a twenty-slot resource hold: wood, stone, ore, scrap, fibre and salt. Harvests remove actual Basin blocks; carrying, stowing, jettisoning and flood loss preserve resource kinds and cargo weight. The phone's contextual button offers Gather. Wreck prefabs stream at deterministic sites across the Basin and remember depleted scrap when revisited; one introductory wreck stays near the start.


- Low Tide's visible upper tread belts now move toward the bow during forward travel. Their animation frame order was reversed; pivoting still sends the two sides in opposite directions.

- Low Tide gains distinct bridge, living-quarter and hold materials, richer furniture and worn metal fittings, authored in its deterministic art generators. The Basin gains quieter, more varied salt, kelp, dunes, reef and brine textures. A compact instrument panel, cabin runner and exposed heating pipes dress the moving home; tread belts roll with driving and pivoting, and freeze while flooded. The floor plan and cargo rules are unchanged.

- Low Tide now has a repeating flood season: three minutes of low water, a gradual rise, high water and an ebb. Lowlands turn to brine, flooded crawlers wait for the ebb and lose one unsecured crate per season, and caught crew lose carried salvage but can wade out slowly. No game over. The HUD names the phase and its countdown. A general map-only voxel flood overlay and `Grid.set_flood`/`Grid.flooded` make the drawing and gameplay agree without changing generated terrain, sparse edits or cached map columns.

- A voxel world can be viewed as a map. `"view": "map"` on `sindri.voxel_world` draws the same world flat from straight above, each column as the top face of its highest block, lit by height so ridges, cliffs and water depth still read; only what the camera sees is worked out. `Grid.surface(world, column, row)` and `Grid.height(world, column, row)` tell a script what is on top of a column and how high, `null` where nothing stands (the first host call to answer with an optional). Low Tide's Basin is now an endless generated drained sea floor: salt flats, salt crust, mud, dunes, dead reef, kelp beds and old islands with trees, with brine left in the deeps. Soft and rough ground slow the crawler, and it runs aground at brine and cliffs; the crew will not wade into brine. The race to the Rise is gone for now, and the Tide returns in its next step as something that pushes rather than kills. Stepping ashore drops the anchor so the crawler no longer drives off without you. `project-capture` binds a project's tile sets.
- Low Tide plays on a phone. The first touch brings up its controls: a thumb anywhere off the buttons is the virtual stick, drawn as a ring where it landed, and Use, Drop and View buttons stand in for E, X and Tab, with Use labelled for what it does here (Helm, Leave, Salvage) and Drop shown only beside a crate. The hints switch to touch wording, a tap sails again after a voyage ends, and the HUD is narrow enough for a phone held upright. A keyboard player never sees any of it. `tests/played_on_a_phone.rs` plays it on a 390 × 844 screen with fingers only, including a whole salvage run.
- **Low Tide**, a new genre showcase in `games/low-tide`: a top-down game about a moving home. You crew a crawler, a house on treads drawn as a Prison Architect-style floor plan, racing across a drained seabed to the Rise ahead of the returning Tide. Set the throttle at the helm and walk the deck while it drives; the camera turns with the deck while you walk it and stays north-up at the helm. Go ashore at wrecks for scrap, but every crate in the hold slows the crawler. It uses existing capabilities only: child transforms on a rotating tilemap, `World.set_parent` at the ramp, camera roll, and `Grid.tile`/`set_tile` from Decay. `tests/a_voyage_is_played.rs` plays it with keys: driving, turning, salvaging, winning and losing.
- Causeway's ground is the engine's voxel world. Its floor is a camera-following `sindri.voxel_world` -- the engine's natural terrain from seed 1, drawn with Causeway's own blocks -- and `game/` no longer generates, streams or meshes terrain: the worldgen, chunk streaming, skirts and the smooth-surface prototype are gone. The game opens on a sandy bank with the beacon across a river; clicking the water lays planks, kept as edits on the world, and the walker crosses them. Leaves are cut out rather than drawn with white where the art is clear. A step costs about 1 ms on a desktop.
- Voxel worlds draw blocks the way tile volumes do. A block is meshed as the box its tile describes, so slabs, posts and rails are their own shapes and hide a neighbour's face only where they cover it; each cell wears one of its block's variants, chosen as a tile volume chooses them and seeded by the world's new `variant_seed`; and grass under a block wears its buried look.
- A voxel world can be a grid's ground. On a grid of boxes, `sindri.voxel_world` is what the grid's cells hold: scripts read, build and remove its blocks with `Grid.block`, `Grid.set_block` and `Grid.tagged`, `sindri.grid.placement` stands things on it, `Grid.can_reach` and `Grid.step_toward` walk across it within the grid's step limit, and the pointer aims at its blocks. Changes are kept in the world's new `edits` list, which stores only what differs from what was generated and remeshes only the sections an edit touches. `follow_camera` keeps a world's resident window under what the camera is looking at. `VoxelGround` answers the same questions for any Rust host.
- Causeway runs far lighter: its terrain stream decoded every resident cell from JSON on every step just to find the camera had not moved to a new chunk, which was nearly all of a step's time (about 9.5 ms to 0.1 ms on a desktop). Its orthographic camera now stands 120 units back rather than 30, so high ground between it and the walker is no longer cut away by the near plane on a tall screen.
- Causeway's beacon lights by sequences rather than by code: arriving plays an authored swell whose first cue sounds the chime, and it then breathes on a loop, both editable in the Timeline. Exports carry the sound a sequence cue names, so `audio/victory.wav` is no longer listed by hand in Causeway's `sindri.toml`. The platformer, Flappy, Scorchball and Orbital test harnesses advance sequences as the shared session does.
- Browser builds no longer stop when Escape is held across a frame. Escape is a game key (back, pause, closing a list), and on a slow device a frame landing between its going down and coming up ended the engine: a paused Orbital or a Back in a menu froze the page. A flick that starts, travels and lifts between two steps now scrolls by its whole travel rather than none of it.
- The editor has a **Sprite sheet** tab in the top row. Choosing an image opens it there filling the canvas, pixel-sharp, with wheel zoom about the pointer, middle- or right-drag panning and Fit; the slice grid and cell names are drawn over it and a click picks a cell, while the slice's settings and names stay in the inspector. With nothing open it lists the project's images. In the canvas arrangement the title bar's search moves aside rather than covering a long row of tabs.
- **Sequences and a Timeline.** A `sindri.sequence` component holds choreography: keyframed tracks that move a transform channel or any numeric component field on an entity, its named children, or anything in the scene by a path from the top such as `/camera`, with CSS easings, and named cues that can play a sound. Every desktop and browser export and the editor's Play run them; scripts start them with `Sequence.play` and wait on cues with `Sequence.cued`. The editor's new **Timeline** panel shows the selected entity's sequences against time, scrubs a preview into the Scene view without changing the scene, and adds, keys, drags and edits tracks, keys and cues as undoable steps. Sequence Stage (`examples/sequence`) is the feature example.
- The editor's Play now **plays sound**: autoplay audio sources start with the run and every scripted `Audio` request is performed through the same mixer a build uses. A new **Audio** panel lists each bus the run uses and everything playing now, with a trim, mute and solo per bus for listening, which never change what the game itself set, and a stop button per sound. `AudioMixer::playing` and `AudioMixer::buses` report what a mixer is playing and its bus volumes.
- The editor has a **Profiler** panel. While Play runs it times every frame on the CPU by phase — effects, physics, screen UI, scripts, sprite animation, cameras, and each view drawn — and keeps the last 300 as a stacked chart against the 60 fps budget. Point at a bar for that frame, click to pin it; otherwise the panel shows the average frame. Each script's time and runs per frame are listed slowest first. Decay hosts can ask for the same per-script timings with `Scripts::set_measuring`, which fills `ScriptReport::timings`.
- The editor has a **Scenes** panel: the project's scenes as a board of cards, the main scene first, each showing the last frame either view drew of it. Arrows show the doors between scenes, read from the literal `Scene.go("…")` calls in the scripts each scene runs (and the prefabs it places), and a door to a scene the project does not carry is marked on the card it leaves from. Click a card to open its scene; **Add scene**, **Remove from project**, **Move earlier**/**later** and **Set as main scene** edit `[project] scenes` and `main_scene`. The editor now writes `sindri.toml` by editing it in place, so `[web.splash]` and comments survive it — before, nominating a main scene dropped both.
- Sound Mixer and Input Actions are laid out by Weave and fit a desktop, a phone held upright and a phone on its side: a centred panel of bus rows, and the Input example's arena sized to the room between its bindings and its controls. Scripts gain `Ui.position(element)` and `Ui.size(element)`, which read where an element was laid out and how big, in overlay units, so a game can fit something in its world to a gap in its UI. `game/tests/the_mixer_and_input_demos_work.rs` plays both through the styled session at all three screen shapes.
- Audio plays through buses. Every sound goes through a named bus under `master` — `Audio.play` through `effects`, `Audio.loop` through `music`, `Audio.play_on`/`loop_on` through any — and `Audio.set_volume(bus, volume)` turns a bus up or down at once for what is playing and what plays next; `Audio.volume(bus)` reads it back. An authored `sindri.audio.source` names its `bus`. The platform's `AudioMixer` applies bus gain in front of the native, browser and silent backends, which can now change a playing voice's volume. Orbital Last Stand's pause screen has Master, Music and Effects sliders, saved between sessions, and the new Sound Mixer example puts a slider on each bus.
- 2D physics answers area checks and swept shapes: `Physics.overlap_circle` and `overlap_box` list every entity a placed shape overlaps, and `Physics.cast_circle` and `cast_box` report the first piece a shape moved along a line would touch, with the raycast's filter arguments. Orbital's hostile mine now damages what its blast circle overlaps, by collider rather than by centre. Physics Playground gains a query button: ray, swept circle, or an area naming what it holds.
- Collision layers have names. A scene's `sindri.physics2d.world` lists them bit by bit, `Physics.layer("ground")` and `Physics.mask(["ground", "hero"])` turn names into masks (refusing a name the world does not give), and the inspector shows every collision mask as a menu of named layers. The platformer names ground, hero and pickups.
- Tweens compose: `Tween.set_delay`, `Tween.set_loops` (0 for ever), `Tween.set_yoyo` and `Tween.after(tween, previous)` for sequences. Orbital's pickups breathe with an endless yoyo after they appear, and Tween Lab gains a row that crosses, waits and returns.
- Input actions reach scripts. A scene declares `sindri.input.actions`; `Action.held`, `pressed`, `released`, `axis` and `vector` read them by name, `Action.bindings` lists them, and `Action.rebind` with `Action.last_pressed` rebinds one while the game runs. The platformer's hero runs and jumps by actions, and the new Input example rebinds its boost to any key or pad button.
- A focused slider moves by its step under the arrows, the d-pad or a pad's stick, and sliders take `autofocus`. A pad's left stick moves UI focus as the d-pad does, once per push.

- 2D closest-hit raycasts in the engine and Decay: `Physics.raycast` returns a `RayHit2d` snapshot or null, with normalized directions, membership masks, explicit sensor inclusion and whole-entity exclusion. The platformer displays real ground clearance below its hero. Physics Playground visualizes rays, hit points and normals alongside falling/bouncing bodies and sensor events, with desktop/touch controls, directory listing, Pages export and browser checks. Overlap and shape casts remain deferred.

- Weave leaves gameplay transforms unchanged when no sizing declaration applies, so typography no longer makes transformless tagged entities eligible for spatial queries or removes mirrored scales. Spatial Lab regressions cover loaded scenes and desktop/phone stylesheet settlement.

- Spatial Query Lab is an authored Pages feature demo for `World.nearest` and `World.within_radius`, with live ordered results, radius/tag controls, active/parent toggles, composed-transform examples and touch movement. Native and desktop/phone browser regressions exercise the actual queries; homepage and directory entries make it discoverable.

- Camera Lab becomes an exported Pages project with camera-relative dead-zone and camera-center bounds guides, independent follow/confinement/shake/smoothing/dead-zone controls, sampled coordinates, an automatic tour, touch movement and reset. Gameplay stays in Decay and uses the existing engine camera controls. Native and desktop/phone browser regressions exercise the demo; the homepage and directory link it.

- The Pages directory lists every published game, feature demo and documentation route with search, category filters, shareable selections and source links. Route coverage is checked at build time; the full catalog remains accessible without JavaScript.

- Pages exports use the configured site base path, so custom-domain routes load their browser modules and assets from `/examples/` instead of an assumed repository prefix. Browser smoke covers both custom-domain and project-subpath deployments.

- Decay gains managed number, Vec2, Vec3 and Color tweens with typed value reads, pause/resume, cancellation, restart, completion/progress and disposal. Named CSS easing math is shared with Weave while its UI authoring remains CSS-inspired. Orbital pickups now use a short eased appearance animation. Zero duration completes immediately; invalid input fails explicitly; handles are bounded at 8192 and released with their owner. Property binding, sequences/timelines, callbacks, loops/yoyo and CSS keyframes remain deferred.

- Decay adds `World.nearest(tag, position)` and `World.within_radius(tag, position, radius)`: active authored-tag queries over composed world-space `Vec3` positions. Radius results include the boundary, sort nearest first, retain world order for ties, and refuse more than 8192 results. Missing transforms are skipped; nearest returns `null` when none matches. Negative/NaN radii fail; positive infinity searches globally. Orbital player and Arc now use the sorted query while retaining their visibility and impact-point filters. Cone/box queries, physics casts and spatial indexing are deferred.

- A prefab instance can do without some of its prefab's entities: deleting an entity inside an instance removes it from that instance (`"removed"` in the scene), the inspector lists what an instance does without with a way to bring each back, and Apply carries a removal into the prefab. A list component field that changed in a few places is overridden by index (`{ "$items": { "57": 3 } }`) rather than whole, so changing one tile no longer copies the tilemap. An instance's inner entities keep their editor state, such as sibling order. Apply and Make prefab are undone and redone with the scene, prefab files included, unless a file was changed on disk since. A scene whose prefab is missing opens with placeholders that save back unchanged, instead of refusing to open. Prefabs can be dragged from the project browser into the Scene view, Make prefab works on a multi-selection, and a prefab changed while the scene played is followed at Stop.
- Prefabs are placed in scenes as instances rather than copied. An instance names its prefab and keeps only its own ID, parent, name and transform plus what it overrides — a JSON merge patch per component, keyed by the prefab's entity IDs — so an edit to the prefab reaches every instance, keeping each one's overrides. Prefabs nest: a prefab can place another, and an override reaches inside by path (`loot/sparkle`); a script's `World.spawn` makes the whole nest. In the editor, *Add to scene* places an instance in the middle of the Scene view (placing on a grid cell makes one too); the inspector shows what an instance overrides with Revert, Revert all, Apply to the prefab and Unpack; *Make prefab* on a hierarchy row turns a subtree into a prefab and an instance of it; *Edit prefab* opens a prefab as the document being edited; an instance in the open scene follows its prefab as soon as the file changes, as one undo step; and a script's `Prefab` field has a picker. The exporter ships every prefab a scene places, and the browser host makes instances when a project loads. The platformer's ten coins are now instances of `prefabs/coin.prefab`. See `docs/prefabs.md`.
- Sindri's own file formats have their own extensions instead of ending in `.json`: `.scene`, `.prefab`, `.profile`, `.sheet`, `.tileset`, `.actions` and `.isobake`, and a build's ledger is `sindri.manifest`. The editor's file dialogs, project browser, new-scene and duplicate commands, the exporter, decay-lsp, the VS Code extension and the isometric baker all use the new names, and every shipped game, example and fixture has been renamed. The old `*.scene.json`-style names are no longer recognised; rename a project's files and the paths that reference them to migrate (`docs/project-format.md`).
- A browser build no longer opens on a blank page. The exported page shows the Sindri loading screen — the forge mark, wordmark and a moving bar, in plain CSS and SVG so it appears before any script or wasm has arrived — until the game is on screen, which the host now announces as `sindri:ready` (on the first frame presented while `DesktopApp::ready` is true; the browser game is ready once its project is installed). A project can follow it with its own brand: `[web.splash]` in `sindri.toml` takes an `image`, `title`, `caption`, `background` and a minimum `seconds`, checked at export. The host module is now imported when the page starts, so a missing or broken host is reported instead of leaving a loading screen up; and the CI browser smoke waits for the loading screen to give way, failing if it never does.
- Decay has a `Color` type: `Color(r, g, b)`, `Color(r, g, b, a)` or `Color("#ff8800")`, with `r`, `g`, `b` and `a` read and written like a vector's components, `lerp(other, t)` and `with_alpha(a)`. The engine's colours — a sprite's `tint`, `color_multiply` and `color_offset`, a shape's `fill` and `stroke`, a UI image's `tint` — are now `Color` rather than a host `Rgba`, so a script reads, holds and assigns one whole (`this.sprite.tint = HURT.lerp(base, t)`); `this.sprite.tint.r` still reaches one channel. Constants and struct field defaults may be colours, an `@export` colour gets the inspector's swatch, `print` shows one, decay-lsp completes its members and VS Code highlights `Color`. Orbital's chargers colour their elite traits with it.
- Decay has values that may be missing. `f32?`, `String?`, `Vec2?`, `List<T>?` and the like hold a value or `null`, and are never used as their type unchecked: `value ?? fallback` gives the value or the fallback (worked out only when needed), and inside `if x != null { }`, or after an `if x == null { return; }`, a `let` or parameter `x` is its plain type. Using an optional as its type anywhere else — assigning it, arithmetic, a member, an index — is a diagnostic that says it may be `null` and how to handle it. `??` binds between comparison and arithmetic. Entities, structs and enums may already be `null`, so `Entity?` is `Entity`.
- Decay has maps. `Map<K, V>` finds values of one type by keys of another — text, numbers, flags, enum variants or entities — written `["arc": 3.0, "nova": 5.0]`, or `[:]` for an empty one. `m[key]` reads a key the map must have (a missing one is a runtime error naming the key), `get(key, fallback)` one it may not, and `contains`, `keys()`, `values()` and `length` ask it; `m[key] = v` (and `+=`), `remove` and `clear` change the map a `var` or field holds, including through a struct. Keys are listed in the order they were first set, so a walk is the same every run. Like a list a map is a value, copied where it is assigned, and holds at most 10,000 keys. decay-lsp completes a map's members, and VS Code highlights `Map`.
- A Decay struct's field may have a default: `struct Offer { name: String, weight: f32 = BASE * 1.5 }`, and `Offer(name: "a")` then holds the default. A default is worked out when the project compiles, as a constant is — a number, flag, text or enum variant, from literals, operators, variants and constants, including another file's `shared const` — and costs what the literal would. A default that cannot be worked out is reported once, not again at every struct that leaves the field out. decay-lsp shows a struct's defaults on hover.
- Decay's `match` gives a value where one goes: `let clip = match kind { Power.Grow => "enlarger", Power.Boost => "push", _ => "wind" };`. Each arm is `=>` and an expression, separated by commas; the match must cover every variant or end with `_`, as the statement does, and every arm gives the same type. At the start of a statement `match` is still the statement that runs blocks. Scorchball picks each power-up's sign with one, where it assigned a `var` in every arm.
- A camera shaken from gameplay comes back to where it belongs. The camera system took last frame's shake off by working it out again from the current trauma, so every `Camera.add_trauma` between frames took off an offset that was never added and left the camera that much further away; it now keeps the offset it applied. `Camera.shake(strength, frequency, decay)` also took its last two numbers the other way round from its documentation. Scripts gain `Camera.impact(amount)`, which shakes the camera at least that hard without adding up, so a burst of small hits in one frame no longer pins it at its hardest shake. Both Orbital games now shake with the engine's camera instead of moving the camera themselves: their camera carries a `sindri.camera.behavior`, every hit calls `Camera.impact`, and `camera-fx.decay` keeps only the arena's pulse.
- Decay structs have methods. Functions written after a struct's fields are asked of one value — `offer.heavier(best)` — with `this` the value, read as `this.weight`. A method works on a copy, so writing `this.weight` is refused with a note to return the changed value; calls are checked like any other, a method the struct does not have is named alongside the ones it does, and a struct's methods reach every file of a project as the struct does, recompiling their callers when one changes. Calling something a struct does not have used to go unchecked and fail at runtime. decay-lsp completes and outlines methods. Orbital's module chooser asks an offer card to `show` or `hide` itself.
- Decay scripts can write a number a set way: `n.fixed(2)` gives exactly two decimals (`"12.50"`) and `n.padded(2)` the nearest whole number led with zeros (`"05"`), neither ever writing `-0`. Both Orbital games' run clock and "Survived" time now read `1:05`; their scene templates asked for `{}:{.2}`, two decimals rather than two digits, and showed 65.3 seconds as `1:5.30`. decay-lsp completes `n.` with both.
- Decay has constants. `const LIMIT: f32 = 3.0;` at the top of a file names a value usable anywhere in that file, and `shared const` makes it usable in every file of the project. A constant holds a number, a flag, text or an enum variant, and its value is worked out when the project compiles from literals, operators, variants and other constants — including another file's — so every use costs what the literal would. Assigning one, declaring one twice, a value that calls something or reads the world, and constants that need each other are all compile errors. decay-lsp completes constants, shows their value on hover and lists them in the outline, and VS Code highlights `const`. Both Orbital games now declare the camera's `VIEW_SIZE` once, where 21 scripts in Orbital Baked and 10 in Last Stand each exported their own copy.
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
