# Sindri Mobile: design

Status: every screen of the prototype is built in Compose and checked against
it. Most of it still shows sample content; what is wired to the engine is
listed below.

The app is designed first and given functionality second. The interactive
prototype in [`prototype/`](prototype/index.html) is the design: its layout,
palette, type, icons, wording and flows are what the app matches. Where the app
differs from it on purpose, this file says so.

## How the design is checked

Each prototype state has a Compose counterpart drawn on the JVM at the same
viewport (412 × 915 at one pixel per dp), so the two can be set side by side
without a phone.

```bash
# The prototype, captured in each state (needs network for React and fonts):
node mobile/prototype/capture-states.js /tmp/prototype-states

# The app's screens, drawn by Robolectric and written by Roborazzi to
# mobile/android/app/build/outputs/roborazzi/<state>.png:
cd mobile/android && ./gradlew :app:recordRoborazziDebug --tests 'dev.sindri.mobile.DesignScreens'
```

Both name their images after the prototype state, so `20-inspector-half.png`
from one is compared with `20-inspector-half.png` from the other. CI's
**Android app** job records the app's set and uploads it as
`sindri-mobile-design`. In the tests the canvas is a stand-in (Lamplighter's
ground with a lamp post and Wick), because only the chrome is being compared;
on the phone it is the engine's view of the scene.

## The design system

All in `mobile/android/app/src/main/kotlin/dev/sindri/mobile/`.

| Part | Where | What it is |
| --- | --- | --- |
| Palette | `Forge.kt` | The prototype's colours by role: ground, page, well, sheet, card, raised, hairline, line; three text tones; metal, bright metal, metal fill, metal border, on-metal; success, danger; glass and popover |
| Type | `ForgeType.kt` | Inter (variable, by weight) for everything, JetBrains Mono for files and Decay |
| Icons | `ForgeType.kt` | Material Symbols Outlined, subset to the 124 icons the prototype draws, as a 27 KB font; a filled subset for play, pause and stop. `Sym` names them, so a missing icon is a compile error |
| Sprites | `Sprites.kt` | Lamplighter's sprite sheets from the bundled project, cut into cells and drawn unsmoothed, as the prototype draws them |
| Controls | `Controls.kt`, `Dashed.kt` | Primary and secondary buttons, icon buttons with spoken names, chips, segmented controls, the 46 × 28 switch, hairlines, dashed outlines |
| Chrome | `EditorScreen.kt` | Glass panels over the canvas, the bottom bar and play button, the selection chip, the overflow menu, tips and toasts |
| Sheets | `EditorSheets.kt` | The sheet frame (rounded top, handle, the prototype's heights), header and body, cards, grids, sprite thumbnails |

Fonts are bundled under their own licences in `mobile/android/licenses/`.

## Screens

| Prototype state | Compose | Designed | Wired to the engine |
| --- | --- | --- | --- |
| `01-dashboard` | `DashboardScreen` | ✅ | Continue opens the editor on the bundled Lamplighter. Samples, settings and Open folder do nothing yet |
| `02`–`04` New game | `NewGameScreen` | ✅ | The steps play out; creating a project is not wired, so the bundled Lamplighter opens instead |
| `10-editor` | `EditorScreen` | ✅ | The canvas is the engine's scene: drag to pan, pinch to zoom, tap to select, drag a selection to move it |
| `11-more-menu` | `EditorScreen` | ✅ | The switches flip; none changes the canvas yet |
| `12-add` | `EditorSheets` | ✅ | Sample prefabs; placing is not wired |
| `13-paint` | `EditorScreen` | ✅ | Brush, size and Move are chosen; painting is not wired |
| `14-assets`, `15-import` | `EditorSheets` | ✅ | Sample content; the slicer (`16-slicer`) is not built yet |
| `17-layers`, `17b-layers-objects` | `EditorSheets` | ✅ | Sample content |
| `18-scenes` | `EditorSheets` | ✅ | Sample content |
| `19-selected`, `24-multi` | `EditorScreen` | ✅ | The chip names the engine's selection. Duplicate, Select more, Group and Delete are not wired |
| `20`, `21` Inspector | `InspectorSheet` | ✅ | For the engine's selection: its position, and each component's numbers (steppers) and switches, edited through checked, undoable commands |
| `22`, `23` Behaviour | `BehaviourSheet` | ✅ | Opens for the selection with no rules; blocks are not wired to Decay |
| `25-play`, `26-frozen`, `26b-frozen-tune` | `PlayChrome` | ✅ | The chrome appears; the game does not run yet (slice step 4) |
| `27-review` | `EditorSheets` | ✅ | Nothing produces run edits yet |
| `28-coach` | `EditorScreen` | ✅ | Nothing shows tips yet |

Undo, redo and saving are wired: every engine edit is saved as it is made,
through the same code as desktop (`sindri-authoring`). Anything not wired
answers with a short "not wired yet" toast rather than pretending.

## Where the app differs from the prototype on purpose

- **No Journeys link.** The prototype's Journeys button and rail walk a reviewer
  through the design; they are not part of the app.
- **No Save button, and none needed.** The prototype has none, so the app saves
  every engine edit as it is made. `•` after the scene name marks a change not
  yet written, which is only seen if a save fails.
- **The inspector shows engine components plainly.** The prototype's inspector is
  curated: friendly section names, sliders with sensible ranges and units, light
  colour swatches. For the engine's own objects the app shows each component's
  fields as steppers and switches until a presentation layer maps components to
  the curated rows. The sample lamp post shows the curated version.
- **Place is read-only for now.** Across and Down show the position; moving is by
  dragging until stepping by number is wired.

## The journeys the design serves

The prototype names five, and they are the acceptance path for functionality:

1. **Create a new game:** New game → Top-down adventure → name it → select Wick.
2. **Build a level:** paint a path and land, add a lamp post, scale and turn it,
   change its light radius, press Play.
3. **Add gameplay:** select the lamp post, Behaviour → Lights when touched →
   Then… Open the bridge, Play and walk Wick to the lamp.
4. **Work with assets:** Assets → Import → gloom.png, check its frames and clip
   speed, make the prefab and drag Gloom into the world.
5. **Test and iterate:** Play, pause, tune the gloom's speed, resume, stop and
   keep the change.

## Next, in order

1. Run the app on the Pixel 8 and walk each screen against the prototype; fix
   what feels wrong before wiring more.
2. A presentation layer that maps engine components to the curated inspector
   rows (Place in tiles, Light 2D, Sprite, Collider), with sliders and ranges.
3. Play from the editor through the shared session (slice step 4), with pause,
   live tuning and the Review sheet.
4. The journeys above, one at a time: placing prefabs, painting, Behaviour
   blocks that write Decay, importing and slicing.
