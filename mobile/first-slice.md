# Sindri Mobile: first technical slice

Status: step 1 is built and waiting to be tried on the reference phone. Nothing
has run on Android yet. The slice is defined in
[the mobile direction](README.md#proposed-first-technical-slice). This file
tracks what has been proven, what has not, and what is still open. Update it in
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
| 1. Open a real project and display a scene with the real renderer | ✅ Lamplighter's `world.scene` opens as an authored world and draws offscreen at 1080×2400 through `SceneRenderer` | ✅ The APK builds, bundles Lamplighter, and links `libsindri_mobile.so` for arm64 | ❌ Not yet tried |
| 2. Select and move an entity with touch; edit one schema-checked property | 🟡 A drag is merged into one undo step, and an invalid value is refused before it reaches the world or history. No touch input or inspector UI yet | ❌ | ❌ |
| 3. Undo, redo, save, restart, reopen; open the result on desktop | 🟡 Undo returns to the file and redo returns to the edit. The saved scene reopens through the desktop path and the build path | ❌ | ❌ |
| 4. Start and stop a real `Session`; touch input; the scene returns to its state | 🟡 A build-style run starts from the saved edit and leaves the authoring world and the file unchanged. Running from unsaved edits and the keep/discard review of Play edits are not covered | ❌ | ❌ |
| 5. Background/resume, surface recreation, keyboard/insets, interrupted save; device notes | — | 🟡 The surface is released and rebuilt on the existing device. Unexercised | ❌ |

✅ in the first two columns means built and tested where those columns are
tested: off the device. Only the last column says anything about a phone.

## How it fits together

```text
mobile/android  Kotlin + Compose app
  MainActivity        Forge-styled top bar and status line over the canvas
  SceneSurface        SurfaceView: hands its Surface to Rust, draws per vsync
  BundledProjects     copies projects/lamplighter out of the APK on first launch
  NativeBridge        external funs: open, attach, resize, detach, draw, status
        |  JNI (main thread only)
mobile/native   sindri-mobile crate, libsindri_mobile.so
  android/            JNI exports, Surface -> ANativeWindow -> wgpu surface
  OpenedScene         ProjectAssets::read + World::from_scene_with (no Session)
  SceneRenderer       textures, sheets, tile sets, fonts; extract + encode frame
        |
engine crates   sindri-runtime (ProjectAssets), sindri-scene, sindri-render,
                sindri-gpu (GpuContext, WindowSurface)
```

The project reader the native player used is now `sindri_runtime::ProjectAssets`,
so the player and the phone find and decode a project's assets the same way.

### The unsafe exception

The workspace forbids unsafe code, and a JNI bridge cannot be written without it:
every function Java calls is exported unmangled, and a `Surface` becomes a
native window only through an unsafe NDK call. `sindri-mobile` therefore denies
unsafe code instead of forbidding it. The seven items that need it, all in
`mobile/native/src/android/`, each carry an `#[allow(unsafe_code)]` and a reason.
`AGENTS.md` records the exception.

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

## Evidence

### Headless tests

- `editor/tests/mobile_first_slice.rs` covers steps 2 to 4 on a copy of Lamplighter
  through `SceneFile`, `CommandHistory`, `WorldCommand`,
  `ComponentSchemaRegistry::validate_payload` and `ProjectRun`.
- `mobile/native/tests/shows_lamplighter.rs` opens Lamplighter's scenes through
  `OpenedScene` and draws `world.scene` at the Pixel 8's resolution. The draw needs
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
| `libsindri_mobile.so` | Exports the six `Java_dev_sindri_mobile_NativeBridge_*` functions; needs only `libandroid`, `libdl`, `libm`, `libc` |
| `cargo check -p sindri-editor --target aarch64-linux-android` | Fails: winit needs an activity type. The mobile app does not use the editor crate, so this no longer blocks anything |

## Known limits of step 1

- **View only.** The scene is drawn through its own camera. There is no pan, zoom,
  selection or editing on the phone yet.
- **Main-thread open.** The native host lives on the UI thread, because the
  surface's rebuild closure in `sindri-gpu` is not `Send`. Opening a large project
  there may stall the UI.
- **Duplicated drawing code.** `SceneRenderer` repeats the native player's texture
  binding and frame encoding, which are tied to its desktop host. Extract a
  shared version once the mobile host's needs are settled.
- **No models, audio or stylesheets.** 3D models are not bound, nothing plays
  sound, and screen UI is drawn unstyled.
- **Project updates.** The bundled copy is replaced whenever the app is updated.
  That is harmless while nothing edits it, and must change before editing lands.

## What a mobile host cannot reach yet

Lives in the desktop editor crate, which the app cannot depend on:

- `scene_file::SceneFile`: migrated open, prefab stand-ins, canonical writes, and
  save adoption. Needed for step 3.
- `scripts::SceneScripts`, `weave_styles::ProjectStyles` and `play_session::start`
  (Play from unsaved edits). Needed for step 4.
- The run-edit review of changes made during Play.

Already shared: `CommandHistory`, `WorldCommand`, schema validation, `Session`,
`ProjectRun`, `ProjectAssets`, `GpuContext`, `WindowSurface`.

## Next

1. Install the APK on the Pixel 8 and record the step 1 checks above, with the
   time to first frame, in the device column.
2. Step 2: pan and zoom the canvas with touch, select by tap through the engine's
   picking, drag to move through `CommandHistory`, and show one schema-checked
   field in a Compose bottom sheet.
3. Step 3: move `SceneFile`'s open and save out of the editor crate so the phone
   and desktop save through one implementation.
