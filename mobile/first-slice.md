# Sindri Mobile: first technical slice

Status: started. Nothing runs on Android yet. The slice is defined in
[the mobile direction](README.md#proposed-first-technical-slice). This file
tracks what has been proven, what has not, and what is blocking the rest. Update
it in the same change as the work it describes.

## Progress

| Slice step | Headless, shared seams | On an Android phone |
| --- | --- | --- |
| 1. Open a real project and display a scene with the real renderer | 🟡 Opened through the desktop's own path. Nothing is drawn | ❌ |
| 2. Select and move an entity with touch; edit one schema-checked property | 🟡 A drag is applied as merged moves and becomes one undo step. An invalid value is refused before it reaches the world or history. No touch input or inspector UI yet | ❌ |
| 3. Undo, redo, save, restart, reopen; open the result on desktop | 🟡 Undo returns to the file and redo returns to the edit. The saved scene reopens through the desktop path and the build path. "Restart" means dropping all in-memory state | ❌ |
| 4. Start and stop a real `Session`; touch input; the scene returns to its state | 🟡 A build-style run starts from the saved edit, steps 120 frames, and leaves the authoring world and the file unchanged. Running from unsaved edits and the keep/discard review of Play edits are not covered | ❌ |
| 5. Background/resume, surface recreation, keyboard/insets, interrupted save; device notes | — needs a device | ❌ |

A 🟡 cell means the behavior holds for the engine and editor seams a mobile host
is meant to share, when driven from a test. It does not mean any of it builds,
renders or behaves on Android.

## Evidence

### Headless slice test

`editor/tests/mobile_first_slice.rs` copies Lamplighter to a temporary folder
and edits `world.scene` through `SceneFile`, `CommandHistory`, `WorldCommand`
and `ComponentSchemaRegistry::validate_payload`. It drags Wick, and it changes
the intensity of the `cover-17` lamp's `sindri.light2d` after first trying a
value that must be refused. It then undoes, redoes, saves, reopens, and plays
the saved project with `ProjectRun`.

```bash
cargo test -p sindri-editor --test mobile_first_slice
```

### Android compile check

These are type checks only: no NDK, no linking, no APK, no device.

| Crates | `cargo check --target aarch64-linux-android` |
| --- | --- |
| `sindri-core`, `sindri-scene`, `sindri-decay`, `sindri-runtime`, `sindri-render`, `sindri-assets`, `sindri-platform` | Passes (checked 2026-10-31) |
| `sindri-editor` | Fails. winit's Android backend (`android-activity`) needs the app to enable either `game-activity` or `native-activity`. Other errors may be hidden behind this one |

Linking needs the Android NDK, and C dependencies (audio, for example) may fail
at that point even though the type check passes. That has not been tried.

## What a mobile host cannot reach yet

Shared and engine-side today, with no editor dependency:

- `CommandHistory`, `Transaction`, `WorldCommand`, and the undo/redo revision.
- `ComponentSchemaRegistry` and payload validation.
- `sindri-runtime`: `Session`, `Checkpoint`, and `ProjectRun`, which opens a project
  the way a build does.

Lives in the desktop editor crate, which a mobile host cannot depend on as it
stands (it pulls in eframe, rfd and the desktop window):

- `scene_file::SceneFile`: migrated open, prefab expansion, missing-prefab stand-ins,
  canonical writes, and save adoption.
- `native::{load_world, scene_extractor}` and `project::Project`.
- `scripts::SceneScripts`: the asynchronous, file-watching script loader that Play
  needs. `weave_styles::ProjectStyles`. Tile-set binding, which the editor
  does while loading textures on the GPU.
- `play_session::start`, which runs from unsaved edits, and the run-edit review
  of changes made during Play.

Before any of this moves into a shared crate, the mobile host has to exist and
need it. The direction document requires a proven dependency boundary
before adding a crate.

## Blocked on decisions

These must be settled before the on-device column can move:

1. **UI host spike.** Compose plus Rust is the initial candidate, and Rust/egui is
   the alternative. The choice also settles the activity type (`game-activity`
   or `native-activity`) that blocks the editor's Android build.
2. **Device and minimum Android version.** Choose a reference phone and an API level.
3. **Build environment.** Choose where the Android SDK and NDK live (local, CI, or
   both). This cloud environment has neither, and no device.
4. **Slice storage.** The simplest option is an app-owned copy of the project.
   Document-provider access can follow as its own investigation.

## Next

- Settle the decisions above, then build the smallest host that does step 1 on
  the reference phone: open Lamplighter's `world.scene` and draw it with
  `sindri-render`.
- Make the same headless test pass against whatever authoring code that host
  ends up calling, so desktop and mobile are checked by one test.
- Record device, frame time and limitations here for each step as it moves.
