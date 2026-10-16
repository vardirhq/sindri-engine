# Scripted day/night and mounted lights

`examples/daynight` is a feature example for Explorer and external Low Tide 3D's
planned lighting track. It reuses Model Collision Lab's self-contained GLBs and
the shared runtime, material, lighting and shadow paths. In-tree game adoption
remains open.

The Camera's checked Decay script owns a configurable clock. A day defaults to
twelve simulation seconds, starting at 06:00. Smooth sine-based daylight blends
the directional sun's color/intensity, ambient intensity/color and background.
The sun rotates continuously and is disabled below the horizon. A moving,
turning imported prop carries a work spotlight and a blue point light, which
fade in as daylight fades out. Sun and carrier motion continue after the first
completed day reports the ready goal.

`day_seconds` is clamped to at least one; `start_hour` wraps over 24 hours.
`preview_hour >= 0` holds a specific hour for authoring and comparisons; -1 runs
the clock. `move_carrier` and `night_lights` independently control movement and
illumination. These are ordinary exported project properties. There is no new
host API or engine clock, and no generated API files need regeneration.

The project regression opens the real scene/assets, compiles the script and
plays the default clock and a two-second day starting at noon. Both visit
sunrise, midday, sunset and night, report the goal once and keep moving the sun
and carrier afterward. At night extraction resolves both composed local-light
positions while the directional sun is absent, and both models remain in the
render frame. Frozen-hour tests verify fixed model poses, changed sun direction
and independent night-light enablement.

The native and browser controls isolate rendering changes:

| Control | What changes | What stays fixed |
| --- | --- | --- |
| Live cycle | Clock, sun, environment, carrier and mounted lights | One running project and its retained assets |
| 10:00 / 14:00, each shadows on/off | Scripted sun direction and shadow visibility | Model poses, camera and matching per-hour lighting/environment |
| Midnight lights on/off | Mounted local illumination | Sun disabled, model poses, ambient and background |

Shadow comparisons first find shadowed pixels on the imported green ground in
each same-hour on/off pair. They require shadow coverage to leave and enter
different receiver pixels, and the horizontal centroid to move between hours.
Changes in model shading alone cannot satisfy this check. Midnight comparisons
require more than 1,000 surface pixels to brighten while preserving the sky.
The continuous browser run observes all four phase messages and the cycle goal.
Native captures sample fixed simulation times; browser color comparisons use
frozen-hour previews so screenshot scheduling cannot race past sunset. These
verify distinct sky phases and a darker night. CI retains all controls and phase
previews for native and real WebGPU Chromium.

Native frozen-hour controls remove 1,740 and add 2,158 shadowed receiver pixels,
moving their centroid from x=480.54 to x=495.21. Mounted night lights brighten
66,632 pixels with the sun off. Native cycle skies progress through RGB
[131,114,134], [97,153,196], [138,100,110] and [22,29,53].

This closes feature-level day/night and dynamic sun-shadow integration; it does
not close the Milestone 8 game acceptance. Fog/post delivery and controls,
local-light shadows, skyboxes/atmosphere, stars and weather remain separate.


Local validation passes 68 game tests (including the two new real-project
regressions), 11 catalogue checks and three documented-surface checks. Typed
preflight reports zero errors/reminders; warning-denied workspace Clippy and
WASM checks, formatting and file-size checks pass. Browser shadow controls remove
1,737 and add 2,095 receiver pixels, moving their centroid from x=481.08 to
x=495.26. Mounted lights brighten 66,494 browser pixels with the sun disabled.
The continuous browser project reaches all phases and its goal with four asset
fetches and no runtime/GPU errors. Frozen browser skies are RGB [131,113,133],
[97,153,196], [138,100,110] and [22,29,53]. Native editor Play visits all four
phases and reaches ready; setting Preview hour to 17 through the inspector saves
and holds the warm sunset. Stop/save restores seven entities, the original
environment/lights and carrier pose, while retaining the authored preview value.
The final project also reopens and reaches the default cycle goal after authoring
the Camera's resting view; Stop restores its authored state.
