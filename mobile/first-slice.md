# Sindri Mobile: first technical slice

Status: steps 1 to 3 are built. The step 1–2 app has been installed and run on
the reference Pixel 8 ("it kinda works"); the checks below are still to be
recorded one by one, and step 3 has not been on a device yet. The slice is defined in
[the mobile direction](README.md#proposed-first-technical-slice). This file
tracks what has been proven, what has not, and what is still open. The app's
screens now follow the prototype's design; [design.md](design.md) tracks that
side, including which screens are wired to what is proven here. Update it in
the same change as the work it describes.

## Decisions

| Question | Decision |
| --- | --- |
| UI host | Kotlin and Jetpack Compose around the Rust engine, through JNI. The engine draws into a `SurfaceView` with `wgpu` on Vulkan; Compose draws over it |
| Reference device | Pixel 8 (arm64-v8a, Vulkan 1.3) |
| Minimum Android | API 29 (Android 10). The Rust library is linked against the same level |
| Project storage for the slice | An app-owned copy: bundled projects are copied out of the APK into the app's private storage. Document-provider access is a later investigation |
| Build environment | Any machine with the Android SDK, NDK 29.0.14206865 and Rust's `aarch64-linux-android` target. CI builds the APK on every full run |

Package and application ID are `dev.sindri.mobile`, a placeholder until a
published identity is chosen. Release builds are signed with the debug key.

## Progress

| Slice step | Headless, shared seams | Android build | On the Pixel 8 |
| --- | --- | --- | --- |
| 1. Open a real project and display a scene with the real renderer | ✅ Lamplighter's `world.scene` opens as an authored world and draws offscreen at 1080×2400 through `SceneRenderer` | ✅ The APK builds, bundles Lamplighter, and links `libsindri_mobile.so` for arm64 | 🟡 Runs on the Pixel 8; details to record |
| 2. Select and move an entity with touch; edit one schema-checked property | ✅ `Editing` pans, pinches, picks through the engine's own camera matrix, outlines the selection round its own shape, moves, turns and scales it in one undo step per drag, and edits component fields with schema validation and undo | ✅ Tap, drag and pinch on the `SurfaceView`; the design's outline, ring and turn and scale handles on the selection; undo and redo in the top bar; the design's inspector sections (Place, Light 2D, Sprite, Collider 2D …) with sliders, swatches and switches | 🟡 Runs on the Pixel 8; details to record |
| 3. Undo, redo, save, restart, reopen; open the result on desktop | ✅ The phone opens and saves through `sindri-authoring`, the desktop editor's own code. A touch edit, undone and redone, saves, reopens after a restart, reads back through desktop's open path, and plays in a build; an unchanged second save writes identical bytes; a failed save keeps the edit unsaved | ✅ Every edit is saved as it is made, a drag when the finger lifts (the design has no Save button); the project lives in app-specific external storage, so `adb pull` can fetch it; an app update no longer replaces it | ❌ Not yet tried |
| 4. Start and stop a real `Session`; touch input; the scene returns to its state | ✅ `Playing` runs the project's real `Session` through `ProjectRun`, from the saved files (every edit is saved as it is made, and Play saves a failed one first): a saved move is where the game starts, the stick walks Wick, Pause holds the clock and Step moves one frame, the run draws through the editor's renderer with its animations, and the editor's world is never the game's. The keep/discard review of Play edits is not covered | ✅ Play, Pause, Step, Stop and the clock; drag anywhere for the design's stick (held as W/A/S/D), tap for interact; a game that fails stops and says why; Back stops it | ❌ Not yet tried |
| 5. Background/resume, surface recreation, keyboard/insets, interrupted save; device notes | — | 🟡 The surface is released and rebuilt on the existing device. Unexercised | ❌ |

✅ in the first two columns means built and tested where those columns are
tested: off the device. Only the last column says anything about a phone.

## How it fits together

```text
mobile/android  Kotlin + Compose app
  MainActivity        Forge-styled top bar and status line over the canvas
  SceneSurface        SurfaceView: hands its Surface to Rust, draws per vsync,
                      turns tap, drag and pinch into editing commands
  InspectorSheet      the selection's numbers and switches, over the canvas
  EditorState         the overlay's state, read from the host as JSON
  BundledProjects     copies projects/lamplighter out of the APK once, into
                      Android/data/dev.sindri.mobile/files/projects
  NativeBridge        external funs: open, attach, resize, detach, draw,
                      call (every editing command), state
        |  JNI (main thread only)
mobile/native   sindri-mobile crate, libsindri_mobile.so
  android/            JNI exports, Surface -> ANativeWindow -> wgpu surface
  OpenedScene         SceneFile::open + load_world, save; ProjectAssets for drawing
  Editing             selection, CommandHistory, touch press/drag/release,
                      unsaved against the saved history revision
  Canvas              flat viewer camera the touch pans and pinches
  SceneRenderer       textures, sheets, tile sets, fonts; extract + encode frame
        |
engine crates   sindri-authoring (SceneFile, load_world, prefab files),
                sindri-runtime (ProjectAssets), sindri-scene (picking),
                sindri-render, sindri-gpu (GpuContext, WindowSurface)
```

The project reader the native player used is now `sindri_runtime::ProjectAssets`,
entity picking the desktop Scene view used is now `sindri_scene::picking`, and
the desktop editor's scene open and save is now `sindri-authoring`, so the
player, the desktop editor and the phone share them rather than copying. The
editor re-exports each at its old path.

### The unsafe exception

The workspace forbids unsafe code, and a JNI bridge cannot be written without it:
every function Java calls is exported unmangled, and a `Surface` becomes a
native window only through an unsafe NDK call. `sindri-mobile` therefore denies
unsafe code instead of forbidding it. The nine items that need it, all in
`mobile/native/src/android/`, each carry an `#[allow(unsafe_code)]` and a reason.
`AGENTS.md` records the exception. Every editing command crosses the bridge
through one `call(command, arguments)` export, so adding a gesture adds no unsafe
code.

## Build and try it

```bash
rustup target add aarch64-linux-android
sdkmanager "ndk;29.0.14206865" "platforms;android-36"
cd mobile/android
./gradlew assembleRelease          # also builds the Rust library
adb install -r app/build/outputs/apk/release/app-release.apk
```

CI's **Android app** job uploads the same APK as the `sindri-mobile-apk`
artifact. `-PsindriRustProfile=dev` builds the library unoptimized, which is
faster to compile and much slower to run.

### What to check on the Pixel 8 for step 1

1. The app opens to Lamplighter's field: Wick with his lantern at the center,
   in dusk light, under a top bar reading *Lamplighter · world.scene*.
2. The status line names the scene, its entity count, and a Vulkan adapter.
3. Rotating the phone redraws the scene at the new size without stretching.
4. Going home and coming back redraws the scene; no error card appears.
5. Note the time from launch to the first frame. The project is opened on the
   main thread, and a long open risks an "app not responding" dialog.

### What to check on the Pixel 8 for step 2

1. Dragging empty ground pans the field; pinching zooms about the fingers.
2. Tapping Wick or a lamp draws a bright-metal edge round its shape, a ring at
   its feet, a turn handle above and a scale handle at its lower right, and
   names it in the chip above the bar; tapping the ground (which is never
   selected itself) or pressing Back lets it go.
3. Dragging the selected entity moves it under the finger. Undo returns it in
   one step, and redo moves it again.
4. Dragging the turn handle turns it to face the finger, in 15° steps with Snap
   on; dragging the scale handle scales it about where it stands. Each is one
   undo step, and the handles stay the same size at any zoom.
5. With Lighting preview off (the default) the night field shows in its own
   colors; turned on in the ⋮ menu, it is dark and lit by its lamps. The menu
   remembers the setting when closed and reopened.
6. With Lighting preview on, dragging a lamp's Radius or Brightness slider in
   its inspector changes its light as the finger moves, and undo takes the
   whole drag back in one step. A color swatch recolors it; Turn, Size and
   Mirror in Place and Sprite change it on the canvas.
7. Dragging something that was not selected first pans instead of moving it.
8. TalkBack reads the canvas, the handles, the undo and redo buttons, and each
   field.

### What to check on the Pixel 8 for step 4

1. Move Wick in the editor, then press ▶. The game starts with Wick where he was
   put, lit as the game is, through its own camera; the clock counts.
2. Drag anywhere: a ring appears under the thumb and Wick walks the way it is
   pushed, diagonals included; let go and he stops. Walking into a lamp lights
   it as on desktop.
3. ❚❚ pauses: the game holds and the clock stops. ⏭ moves one frame. Resume
   carries on.
4. ■ (or Back) stops. The editor shows the scene exactly as before Play, with
   nothing the game did kept.
5. Leave the app while playing and come back: the game carries on from where
   it was, without running fast to catch up.

### What to check on the Pixel 8 for step 3

Reinstall over the step 1–2 app; the project is installed afresh in its new
place the first time.

1. Move a lamp and change its intensity. Each is saved as it is made: no `•`
   stays after the scene's name.
2. Force-stop the app (Settings › Apps › Sindri › Force stop)
   and open it again. The lamp is where it was left, as bright as it was set.
3. Pull the project and open it on desktop:

   ```bash
   adb pull /sdcard/Android/data/dev.sindri.mobile/files/projects/lamplighter
   cargo editor lamplighter
   ```

   The lamp is in the same place with the same intensity, and Play runs.
4. Update the app (`adb install -r` of a newer build) and check the edit is
   still there.

## Evidence

### Headless tests

- `editor/tests/mobile_first_slice.rs` covers steps 2 to 4 on a copy of Lamplighter
  through `SceneFile`, `CommandHistory`, `WorldCommand`,
  `ComponentSchemaRegistry::validate_payload` and `ProjectRun`.
- `mobile/native/tests/edits_by_touch.rs` covers step 2 on Lamplighter's field: the
  canvas opens where the scene camera looks, a tap where the canvas says an entity
  is picks that entity after panning and pinching, a drag on the selection moves
  it in one undo step while a drag elsewhere pans, the turn and scale handles
  change it in one undo step each, the outline's box encloses the selection and
  follows the view, the outlined copy of the world leaves the scene untouched,
  pinching keeps the point under the fingers, and a field edit is validated,
  shown and undone.
- `mobile/native/tests/saves_and_reopens.rs` covers step 3 on a copy of
  Lamplighter: a touch drag and a field edit, undone (back to saved) and redone,
  are saved; after a restart the phone reads the edit back; desktop's open path
  and a build both see it; a second save with nothing changed writes the same
  bytes; and a save that cannot be written leaves the edit unsaved.
- `mobile/native/tests/shows_lamplighter.rs` opens Lamplighter's scenes through
  `OpenedScene` and draws `world.scene` at the Pixel 8's resolution, lit and
  unlit (the unlit night is far brighter), and draws a selection with its
  bright-metal edge, lit or not. The draw needs
  a GPU adapter. It skips without one unless `SINDRI_REQUIRE_GPU` is set; it was
  run here on Mesa's software Vulkan.

```bash
cargo test -p sindri-editor --test mobile_first_slice
SINDRI_REQUIRE_GPU=1 cargo test -p sindri-mobile
```

### Android build

| What | Result (2026-10-31) |
| --- | --- |
| `cargo clippy -p sindri-mobile --target aarch64-linux-android -- -D warnings` | Clean |
| `./gradlew assembleRelease` | Builds a 35 MB APK: minSdk 29, targetSdk 35, arm64-v8a only, Vulkan 1.1 required |
| `libsindri_mobile.so` | Exports the seven `Java_dev_sindri_mobile_NativeBridge_*` functions; needs only `libandroid`, `libdl`, `libm`, `libc` |
| `cargo check -p sindri-editor --target aarch64-linux-android` | Fails: winit needs an activity type. The mobile app does not use the editor crate, so this no longer blocks anything |

## Known limits

- **Frame everything.** The overflow action fits world objects, sprite art and tilemaps with a margin; screen UI is excluded. Zoom remains limited to the canvas's normal range, so very large scenes may not fit completely. Device verification is open.
- **Add.** Lists project prefabs and places linked instances at the view center; Empty and Light 2D are available without prefabs. Each is undoable and saves through the scene. Lamplighter currently has no prefabs, so its prefab list is empty. Drag placement and thumbnails are not wired; device verification remains open.
- **Objects.** Layers shows the scene hierarchy, including screen UI and prefab children, with collapsible branches and parent selection. Reparenting is undoable; prefab-owned children cannot be restructured. Layer visibility is not wired. Screen UI selection is inspectable but has no canvas handles, and reparenting screen UI retains its local layout. Device verification remains open.
- **Script finder.** Search all supported calls or browse engine categories. Object arguments default to this object and vectors to zero. Some argument types are still unavailable. Object pins can use this object, an Entity field, or a uniquely named scene object found by name; duplicate or unnamed scene objects are not offered. Device verification remains open.
- **Fields.** The inspector edits a component's top-level numbers and switches.
  Text, vectors, colors, enums and references are not shown yet, and a field
  authored as a whole number steps by whole numbers.
- **Scenes.** The Scenes sheet lists the project's manifest and opens a selected
  scene; it saves the current scene first and remembers the selected scene for
  recovery after restart. New scene and scene-door editing remain unwired.
  Scene names must be unique because the runtime resolves them by file name.
  Switching and restarting have not yet been exercised on the Pixel 8.
- **Village ground.** The village's roads, cobbles, flagstones and soil are
  authored in its scene, so the mobile editor can draw them before Play. An
  unchanged copy of the previous bundled village is upgraded on app update;
  edited village files and projects with pending recovery are left untouched.
  Existing edited copies need a separate, non-destructive migration. The new
  layout still needs a Pixel 8 check.
- **Picking and selection.** Sprite taps use the sheet anchor and nontransparent
  pixels, and selection handles use visible pixel bounds. Sliced and custom-shader
  sprites fall back to their full frames. Screen UI is not picked yet. Select
  more toggles objects by tapping; Duplicate and Delete apply to selected subtrees
  as one undo step. Multi-object dragging and property editing are not implemented.
  Device verification is still open.
- **Recovery needs device proof.** Before a mobile scene save the app writes an
  app-private recovery copy. Backgrounding finishes a gesture and attempts to
  save it; after a failed or interrupted save, the next opening restores the
  copy as unsaved. If the scene changed externally, the user chooses which
  version to keep. A scene that cannot be opened at all cannot yet be restored.
  Abrupt termination before an edit reaches a save or background callback can
  still lose that edit. Interruption behavior needs to be exercised on the
  phone. Scene and prefab saves reject symbolic-link targets.
- **The first save rewrites the whole file.** Lamplighter's checked-in scenes are
  not stored in canonical form, so the first save from any editor, desktop
  included, rewrites them in canonical order and spacing (`world.scene` goes
  from 263 KB to 212 KB). The document is unchanged; after that, saves change
  only what was edited.
- **Main-thread open.** The native host lives on the UI thread, because the
  surface's rebuild closure in `sindri-gpu` is not `Send`. Opening a large project
  there may stall the UI.
- **Duplicated drawing code.** `SceneRenderer` repeats the native player's texture
  binding and frame encoding, which are tied to its desktop host. Extract a
  shared version once the mobile host's needs are settled.
- **No models or audio.** 3D models are not bound and nothing plays sound, in
  the editor or a game. A game's screen UI is styled; the editor's is not.
- **Play is portrait and keyboard-shaped.** A game is opened at the screen's
  size when Play is pressed and not re-laid out if the phone turns. Touch
  reaches it as held movement keys (W/A/S/D) and the interact key (E), which is
  what Lamplighter reads; a game bound only to other keys, or to the pointer,
  is not playable yet. Game saves are not kept between runs.
- **No live tuning or run edits.** Pausing holds the game, but tapping
  something to tune it (the design's `26-frozen`) and the Review sheet at Stop
  are not wired, and Step back is not.
- **Projects.** The dashboard opens bundled Lamplighter and blank 2D projects created on the phone. Created projects have separate scene preferences and recovery. Import and export are not available yet, and project creation needs device verification.

## What a mobile host cannot reach yet

Lives in the desktop editor crate, which the app cannot depend on:

- `play_session::start` (Play from unsaved edits). The phone does not need it:
  it saves every edit, so it plays the saved project through `ProjectRun`.
- The run-edit review of changes made during Play.

Already shared: `CommandHistory`, `WorldCommand`, schema validation, `Session`,
`ProjectRun`, `ProjectAssets`, `GpuContext`, `WindowSurface`, picking, and
`sindri-authoring`'s `SceneFile` and `load_world`.

## Next

1. Record what "kinda works" means on the Pixel 8: run the step 1–3 checks above
   and fix what fails before adding more.
2. Run the step 4 checks on the Pixel 8.
3. Step 5: prove interrupted saves and recovery across Android ending the app, and
   the surface, keyboard and inset behavior on the device.
