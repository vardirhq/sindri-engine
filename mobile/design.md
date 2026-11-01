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
| `10-editor` | `EditorScreen` | ✅ | The canvas is the engine's scene: drag to pan, pinch to zoom, tap to select, drag a selection to move it. Tapping the ground lets the selection go: a tilemap is painted, not selected, as in the prototype. Back steps out of the menu, a sheet, painting, a run and the selection before leaving the editor. Moves are saved when the finger lifts |
| `11-more-menu` | `EditorScreen` | ✅ | The switches are remembered while the menu is closed. Lighting preview (off by default) draws the scene's 2D lights or leaves every sprite in its own colours; Snap keeps the turn handle to 15° and the scale handle to tenths; the tile grid is not drawn yet. Frame everything is not wired |
| `12-add` | `EditorSheets` | ✅ | Sample prefabs; placing is not wired |
| `13-paint` | `EditorScreen` | ✅ | Brush, size and Move are chosen; painting is not wired |
| `14-assets`, `15-import` | `EditorSheets` | ✅ | Sample content; the slicer (`16-slicer`) is not built yet |
| `17-layers`, `17b-layers-objects` | `EditorSheets` | ✅ | Sample content |
| `18-scenes` | `EditorSheets` | ✅ | Sample content |
| `19-selected`, `24-multi` | `EditorScreen` | ✅ | The engine draws the selection with the prototype's bright-metal edge round its own shape; the ring at its feet and the turn and scale handles turn and scale it in one undo step each. The chip names it. Duplicate, Select more, Group and Delete are not wired, so `24-multi` is not reached |
| `20`, `21` Inspector | `InspectorSheet`, `InspectorPresenter` | ✅ | The engine's selection in the design's sections: Place (across and up in tiles, turn, size), Light 2D (radius, brightness, colour swatches, shadows), Sprite (its real thumbnail, mirror, draw layer), Collider 2D (shape, sensor, reach or size), Body, Shadow; other components plainly after them; Behaviour names the script; Details lists the rest. Every row is a checked, undoable, saved edit, and a slider drag is one step. Swapping a sprite and adding a component are not wired. `20e`/`21e` draw it beside the prototype's |
| `22`, `23` Behaviour | `BehaviourSheet` | ✅ | Opens for the selection with no rules; blocks are not wired to Decay |
| `25-play`, `26-frozen`, `26b-frozen-tune` | `PlayChrome` | ✅ | ▶ runs the real game in place of the canvas: Stop, Pause, Step one frame and the clock work, and dragging anywhere shows the design's stick (`25b-play-stick`) and walks the player. Tapping something while paused to tune it, and Step back, are not wired |
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
- **Place counts Up, not Down.** The prototype's Place has *Down*, as a web
  page measures. The engine's y grows upward, and the desktop editor shows it
  that way, so the phone shows *Up* with the engine's own number rather than one
  that disagrees with every other tool.
- **Components the design does not cover are shown plainly.** The presenter
  gives the design's sections to light, sprite, collider, body, shadow,
  occluder and script; anything else (a camera, particles) follows as its own
  section of steppers, sliders (where the engine bounds the field), switches
  and short choices, so nothing the engine holds is hidden.
- **The outline is drawn by the engine, the handles by the app.** The edge
  round the selection's shape is part of the canvas's frame (copies of its
  sprite in one flat colour, in a scratch world only the canvas draws), so it
  follows the art exactly. A tilemap or cube has no outline yet, only its
  handles.

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
2. Live tuning while paused, and the Review sheet at Stop.
3. The journeys above, one at a time: placing prefabs, painting, Behaviour
   blocks that write Decay, importing and slicing.
